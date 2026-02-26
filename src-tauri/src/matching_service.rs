//! Service for finding possible matches across brokers.

use spectral_db::{broker_scans, findings::Finding, matching};
use spectral_vault::{UserProfile, Vault};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Normalize string for comparison (lowercase and trim).
fn normalize_string(s: &str) -> String {
    s.to_lowercase().trim().to_string()
}

/// Calculate Jaro-Winkler similarity between two name parts.
fn name_part_similarity(part1: &str, part2: &str) -> f64 {
    strsim::jaro_winkler(part1, part2)
}

/// Calculate name similarity using Jaro-Winkler (0.0 to 1.0).
fn calculate_name_similarity(profile_name: &str, finding_name: &str) -> f64 {
    let p = normalize_string(profile_name);
    let f = normalize_string(finding_name);

    let full_score = name_part_similarity(&p, &f);

    // Also try component matching (first + last)
    let p_parts: Vec<&str> = p.split_whitespace().collect();
    let f_parts: Vec<&str> = f.split_whitespace().collect();

    let component_score = match (p_parts.len(), f_parts.len()) {
        (p_len, f_len) if p_len > 1 && f_len > 1 => {
            let first_score = name_part_similarity(p_parts[0], f_parts[0]);
            let last_score = name_part_similarity(p_parts[p_len - 1], f_parts[f_len - 1]);
            (first_score + last_score) / 2.0
        }
        (p_len, f_len) if p_len > 0 && f_len > 0 => name_part_similarity(p_parts[0], f_parts[0]),
        _ => 0.0,
    };

    full_score.max(component_score)
}

/// Collect all locations from the profile (current + previous addresses).
fn collect_profile_locations(
    profile: &UserProfile,
    vault_key: &[u8; 32],
) -> HashSet<(String, String)> {
    let mut locations = HashSet::new();

    // Add current address
    if let (Some(city), Some(state)) = (&profile.city, &profile.state) {
        if let (Ok(c), Ok(s)) = (city.decrypt(vault_key), state.decrypt(vault_key)) {
            locations.insert((normalize_string(&c), normalize_string(&s)));
        }
    }

    // Add previous addresses
    for prev in &profile.previous_addresses_v2 {
        if let (Ok(c), Ok(s)) = (prev.city.decrypt(vault_key), prev.state.decrypt(vault_key)) {
            locations.insert((normalize_string(&c), normalize_string(&s)));
        }
    }

    locations
}

/// Check if any finding addresses match the profile locations.
fn has_matching_address(finding: &Finding, profile_locations: &HashSet<(String, String)>) -> bool {
    let Some(addresses) = finding.extracted_data.get("addresses") else {
        return false;
    };

    let Some(addr_array) = addresses.as_array() else {
        return false;
    };

    for addr_val in addr_array {
        let Some(addr_str) = addr_val.as_str() else {
            continue;
        };

        let Some((city, state)) = parse_address(addr_str) else {
            continue;
        };

        if profile_locations.contains(&(normalize_string(&city), normalize_string(&state))) {
            return true;
        }
    }

    false
}

/// Check if finding location matches profile locations.
fn check_location_match(profile: &UserProfile, finding: &Finding, vault_key: &[u8; 32]) -> bool {
    let profile_locations = collect_profile_locations(profile, vault_key);
    has_matching_address(finding, &profile_locations)
}

/// Parse "City, State ZIP" format.
fn parse_address(addr: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = addr.split(',').collect();
    if parts.len() >= 2 {
        let city = parts[0].trim().to_string();
        let state = parts[1].split_whitespace().next().unwrap_or("").to_string();
        Some((city, state))
    } else {
        None
    }
}

/// Extract the profile name from encrypted fields.
fn get_profile_name(profile: &UserProfile, vault_key: &[u8; 32]) -> Option<String> {
    if let Some(full) = &profile.full_name {
        return full.decrypt(vault_key).ok();
    }

    if let (Some(first), Some(last)) = (&profile.first_name, &profile.last_name) {
        if let (Ok(f), Ok(l)) = (first.decrypt(vault_key), last.decrypt(vault_key)) {
            return Some(format!("{f} {l}"));
        }
    }

    None
}

/// Match scores for name and location similarity.
struct MatchScores {
    name_similarity: f64,
    location_matched: bool,
    combined_score: f64,
}

/// Calculate match scores for a finding.
fn calculate_match_scores(
    profile_name: &str,
    finding: &Finding,
    profile: &UserProfile,
    vault_key: &[u8; 32],
) -> MatchScores {
    let name_similarity = calculate_name_similarity(
        profile_name,
        finding
            .extracted_data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );
    let location_matched = check_location_match(profile, finding, vault_key);
    let location_score = if location_matched { 1.0 } else { 0.0 };
    // nosemgrep: llm-prompt-injection-risk
    let combined_score = (name_similarity * 0.7) + (location_score * 0.3);

    MatchScores {
        name_similarity,
        location_matched,
        combined_score,
    }
}

/// Check if scores meet minimum thresholds.
fn meets_threshold(scores: &MatchScores) -> bool {
    const MIN_NAME_SIMILARITY: f64 = 0.85;
    const MIN_COMBINED_SCORE: f64 = 0.70;

    scores.name_similarity >= MIN_NAME_SIMILARITY && scores.combined_score >= MIN_COMBINED_SCORE
}

/// Create a possible match from scores and finding.
fn create_possible_match(finding: &Finding, scores: &MatchScores) -> matching::PossibleMatch {
    matching::PossibleMatch {
        finding: finding.clone(),
        similarity_score: scores.combined_score,
        name_similarity: scores.name_similarity,
        location_matched: scores.location_matched,
        source_broker_id: finding.broker_id.clone(),
    }
}

/// Sort and limit matches to top 5 per broker.
fn finalize_matches(matches_by_broker: &mut HashMap<String, Vec<matching::PossibleMatch>>) {
    for matches in matches_by_broker.values_mut() {
        matches.sort_by(|a, b| {
            b.similarity_score
                .partial_cmp(&a.similarity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        matches.truncate(5);
    }
}

/// Find possible matches for zero-result brokers.
///
/// # Errors
/// Returns an error if database queries fail or profile data cannot be accessed.
pub async fn find_possible_matches(
    db: &spectral_db::Database,
    _vault: &Arc<Vault>,
    scan_job_id: &str,
    profile: &UserProfile,
    vault_key: &[u8; 32],
) -> Result<HashMap<String, Vec<matching::PossibleMatch>>, String> {
    // Get zero-result brokers
    let zero_result_scans = broker_scans::get_zero_result_scans(db.pool(), scan_job_id)
        .await
        .map_err(|e| format!("Failed to get zero-result scans: {e}"))?;

    if zero_result_scans.is_empty() {
        return Ok(HashMap::new());
    }

    let exclude_ids: Vec<String> = zero_result_scans
        .iter()
        .map(|s| s.broker_id.clone())
        .collect();

    // Get findings from other brokers
    let other_findings =
        matching::get_findings_from_other_brokers(db.pool(), scan_job_id, &exclude_ids)
            .await
            .map_err(|e| format!("Failed to get findings: {e}"))?;

    if other_findings.is_empty() {
        return Ok(HashMap::new());
    }

    // Get profile name
    let profile_name = match get_profile_name(profile, vault_key) {
        Some(n) => n,
        None => return Ok(HashMap::new()),
    };

    // Match findings against profile
    let mut matches_by_broker: HashMap<String, Vec<matching::PossibleMatch>> = HashMap::new();

    for finding in other_findings {
        let finding_name = finding
            .extracted_data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if finding_name.is_empty() {
            continue;
        }

        let scores = calculate_match_scores(&profile_name, &finding, profile, vault_key);

        if meets_threshold(&scores) {
            let possible_match = create_possible_match(&finding, &scores);

            for zero_scan in &zero_result_scans {
                matches_by_broker
                    .entry(zero_scan.broker_id.clone())
                    .or_default()
                    .push(possible_match.clone());
            }
        }
    }

    finalize_matches(&mut matches_by_broker);
    Ok(matches_by_broker)
}
