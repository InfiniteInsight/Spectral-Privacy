//! Matching operations for finding similar people across brokers.

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use strsim::jaro_winkler;

/// Minimum combined similarity score for a finding to be considered a match.
pub const DEFAULT_MATCH_THRESHOLD: f64 = 0.75;

/// Weight given to name similarity vs. location in the combined score.
const NAME_WEIGHT: f64 = 0.70;
const LOCATION_WEIGHT: f64 = 0.30;

/// Base SQL query for fetching findings with broker scan join.
const FINDINGS_BASE_QUERY: &str = "SELECT f.id, f.broker_scan_id, f.broker_id, f.profile_id,
        f.listing_url, f.verification_status, f.extracted_data,
        f.discovered_at, f.verified_at, f.verified_by_user,
        f.removal_attempt_id
 FROM findings f
 JOIN broker_scans bs ON f.broker_scan_id = bs.id";

/// A possible match found on another broker for a zero-result broker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PossibleMatch {
    /// The finding from another broker that might match
    pub finding: crate::findings::Finding,
    /// Combined similarity score (0.0 to 1.0)
    pub similarity_score: f64,
    /// Name similarity score (0.0 to 1.0)
    pub name_similarity: f64,
    /// Whether location matched with profile
    pub location_matched: bool,
    /// Broker ID where this finding was found
    pub source_broker_id: String,
}

/// Compute name similarity between a finding's extracted name and a profile name.
///
/// Returns a score in [0.0, 1.0]. Uses Jaro-Winkler on the lowercased full name.
/// Bonuses: last-name-only match scores at least 0.70 so a common last name is
/// still surfaced for user review; exact full-name match returns 1.0.
pub fn score_name(extracted_name: &str, first_name: &str, last_name: &str) -> f64 {
    let extracted = extracted_name.trim().to_lowercase();
    if extracted.is_empty() {
        return 0.0;
    }
    let full_name = format!("{} {}", first_name.to_lowercase(), last_name.to_lowercase());
    let full_score = jaro_winkler(&extracted, &full_name);

    // Also score last name alone (many brokers show "Last, First" or just last)
    let last_lower = last_name.to_lowercase();
    let last_score = if extracted.contains(&last_lower) {
        // Exact last-name substring: minimum 0.70, but boosted by how closely
        // the full string matches just the last name token.
        let raw = jaro_winkler(&extracted, &last_lower);
        0.70_f64.max(raw * 0.85)
    } else {
        jaro_winkler(&extracted, &last_lower) * 0.85
    };

    full_score.max(last_score)
}

/// Compute location similarity between a finding's addresses and a profile location.
///
/// Returns 1.0 if any extracted address contains the city+state, 0.5 if only
/// the state matches, and 0.0 otherwise. Case-insensitive.
pub fn score_location(addresses: &[String], city: Option<&str>, state: Option<&str>) -> f64 {
    if addresses.is_empty() {
        return 0.0;
    }
    let city_lower = city.map(|c| c.to_lowercase());
    let state_lower = state.map(|s| s.to_lowercase());

    let mut best = 0.0_f64;
    for addr in addresses {
        let addr_lower = addr.to_lowercase();
        let city_match = city_lower
            .as_deref()
            .is_some_and(|c| addr_lower.contains(c));
        let state_match = state_lower
            .as_deref()
            .is_some_and(|s| addr_lower.contains(s));

        let score = match (city_match, state_match) {
            (true, true) => 1.0,
            (false, true) => 0.5,
            (true, false) => 0.4,
            (false, false) => 0.0,
        };
        if score > best {
            best = score;
        }
    }
    best
}

/// Score a single finding against a profile.
///
/// Returns `None` if the finding's extracted data has no name field.
pub fn score_finding(
    finding: &crate::findings::Finding,
    first_name: &str,
    last_name: &str,
    city: Option<&str>,
    state: Option<&str>,
) -> Option<PossibleMatch> {
    // Extract name from the JSON extracted_data
    let extracted_name = finding.extracted_data.get("name")?.as_str()?;

    let name_similarity = score_name(extracted_name, first_name, last_name);

    // Extract addresses array
    let addresses: Vec<String> = finding
        .extracted_data
        .get("addresses")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    let location_score = score_location(&addresses, city, state);
    let location_matched = location_score >= 0.5;

    let similarity_score = NAME_WEIGHT * name_similarity + LOCATION_WEIGHT * location_score;

    Some(PossibleMatch {
        source_broker_id: finding.broker_id.clone(),
        finding: finding.clone(),
        similarity_score,
        name_similarity,
        location_matched,
    })
}

/// Score and filter a list of findings against a profile.
///
/// Returns only findings that exceed `threshold`, sorted by descending score.
pub fn filter_and_score_findings(
    findings: Vec<crate::findings::Finding>,
    first_name: &str,
    last_name: &str,
    city: Option<&str>,
    state: Option<&str>,
    threshold: f64,
) -> Vec<PossibleMatch> {
    let mut scored: Vec<PossibleMatch> = findings
        .iter()
        .filter_map(|f| score_finding(f, first_name, last_name, city, state))
        .filter(|m| m.similarity_score >= threshold)
        .collect();

    scored.sort_by(|a, b| {
        b.similarity_score
            .partial_cmp(&a.similarity_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored
}

/// Get all findings from other brokers in the same scan job.
///
/// Excludes findings from zero-result brokers.
///
/// # Errors
/// Returns `sqlx::Error` if the database query fails.
pub async fn get_findings_from_other_brokers(
    pool: &Pool<Sqlite>,
    scan_job_id: &str,
    exclude_broker_ids: &[String],
) -> Result<Vec<crate::findings::Finding>, sqlx::Error> {
    let query = if exclude_broker_ids.is_empty() {
        format!("{FINDINGS_BASE_QUERY} WHERE bs.scan_job_id = ? ORDER BY f.discovered_at DESC")
    } else {
        let placeholders = exclude_broker_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "{FINDINGS_BASE_QUERY} WHERE bs.scan_job_id = ? AND f.broker_id NOT IN ({placeholders}) ORDER BY f.discovered_at DESC"
        )
    };

    let mut query_builder = sqlx::query(&query).bind(scan_job_id);
    for broker_id in exclude_broker_ids {
        query_builder = query_builder.bind(broker_id);
    }

    let rows = query_builder.fetch_all(pool).await?;
    crate::findings::parse_findings_from_rows(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_name_exact() {
        assert!(score_name("John Smith", "John", "Smith") > 0.95);
    }

    #[test]
    fn test_score_name_case_insensitive() {
        assert!(score_name("john smith", "John", "Smith") > 0.95);
    }

    #[test]
    fn test_score_name_last_name_only() {
        // Last name match alone should score >= 0.70
        let score = score_name("Smith, Robert", "John", "Smith");
        assert!(score >= 0.70, "last-name match scored {}", score);
    }

    #[test]
    fn test_score_name_no_match() {
        let score = score_name("Alice Johnson", "John", "Smith");
        assert!(score < 0.75, "unrelated name scored {}", score);
    }

    #[test]
    fn test_score_name_empty() {
        assert_eq!(score_name("", "John", "Smith"), 0.0);
    }

    #[test]
    fn test_score_location_city_and_state() {
        let addrs = vec!["Los Angeles, CA".to_string()];
        assert_eq!(score_location(&addrs, Some("Los Angeles"), Some("CA")), 1.0);
    }

    #[test]
    fn test_score_location_state_only() {
        let addrs = vec!["Sacramento, CA".to_string()];
        let score = score_location(&addrs, Some("Los Angeles"), Some("CA"));
        assert_eq!(score, 0.5);
    }

    #[test]
    fn test_score_location_no_match() {
        let addrs = vec!["New York, NY".to_string()];
        assert_eq!(score_location(&addrs, Some("Los Angeles"), Some("CA")), 0.0);
    }

    #[test]
    fn test_score_location_empty() {
        assert_eq!(score_location(&[], Some("LA"), Some("CA")), 0.0);
    }
}
