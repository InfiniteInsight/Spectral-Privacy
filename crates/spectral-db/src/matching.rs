//! Matching operations for finding similar people across brokers.

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};

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
    if exclude_broker_ids.is_empty() {
        // No brokers to exclude, get all findings
        let rows = sqlx::query(
            "SELECT f.id, f.broker_scan_id, f.broker_id, f.profile_id,
                    f.listing_url, f.verification_status, f.extracted_data,
                    f.discovered_at, f.verified_at, f.verified_by_user,
                    f.removal_attempt_id
             FROM findings f
             JOIN broker_scans bs ON f.broker_scan_id = bs.id
             WHERE bs.scan_job_id = ?
             ORDER BY f.discovered_at DESC",
        )
        .bind(scan_job_id)
        .fetch_all(pool)
        .await?;

        return crate::findings::parse_findings_from_rows(rows);
    }

    // Build dynamic query with placeholders for excluded brokers
    let placeholders = exclude_broker_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");

    let query = format!(
        "SELECT f.id, f.broker_scan_id, f.broker_id, f.profile_id,
                f.listing_url, f.verification_status, f.extracted_data,
                f.discovered_at, f.verified_at, f.verified_by_user,
                f.removal_attempt_id
         FROM findings f
         JOIN broker_scans bs ON f.broker_scan_id = bs.id
         WHERE bs.scan_job_id = ?
           AND f.broker_id NOT IN ({placeholders})
         ORDER BY f.discovered_at DESC"
    );

    let mut query_builder = sqlx::query(&query).bind(scan_job_id);
    for broker_id in exclude_broker_ids {
        query_builder = query_builder.bind(broker_id);
    }

    let rows = query_builder.fetch_all(pool).await?;
    crate::findings::parse_findings_from_rows(rows)
}
