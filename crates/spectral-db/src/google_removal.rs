//! Google removal request tracking.
//!
//! This module provides functionality for generating and tracking Google search
//! removal requests for confirmed findings.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};
use uuid::Uuid;

/// A Google removal request record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleRemovalRequest {
    /// Unique identifier for the request
    pub id: String,
    /// ID of the finding this request is for
    pub finding_id: String,
    /// Current status of the removal request
    pub status: GoogleRemovalStatus,
    /// Pre-filled URL to Google's removal form
    pub google_removal_url: String,
    /// When the URL was generated
    pub generated_at: DateTime<Utc>,
    /// When the user reported submitting the request
    pub submitted_at: Option<DateTime<Utc>>,
    /// When the user reported the request was completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Optional user notes
    pub notes: Option<String>,
}

/// Status of a Google removal request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum GoogleRemovalStatus {
    /// URL has been generated but not yet submitted
    URLGenerated,
    /// User has reported submitting to Google
    Submitted,
    /// User has reported the removal is complete
    Completed,
    /// Request failed or was rejected
    Failed,
}

impl GoogleRemovalStatus {
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "URLGenerated" => Ok(Self::URLGenerated),
            "Submitted" => Ok(Self::Submitted),
            "Completed" => Ok(Self::Completed),
            "Failed" => Ok(Self::Failed),
            _ => Err(format!("Invalid GoogleRemovalStatus: {s}")),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::URLGenerated => "URLGenerated",
            Self::Submitted => "Submitted",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
        }
    }
}

/// Generate Google removal URL for a finding.
///
/// Creates a deep link to Google's "Results about you" feature with a pre-filled
/// search query based on the person's name and optional address/phone.
#[must_use]
pub fn generate_removal_url(name: &str, address: Option<&str>, phone: Option<&str>) -> String {
    let mut query_parts = vec![name.to_string()];

    if let Some(addr) = address {
        query_parts.push(addr.to_string());
    }
    if let Some(ph) = phone {
        query_parts.push(ph.to_string());
    }

    let query = query_parts.join(" ");
    let encoded = urlencoding::encode(&query);

    format!("https://myactivity.google.com/results-about-you?q={encoded}")
}

/// Create a new Google removal request.
///
/// This function inserts a new removal request record with status `URLGenerated`.
pub async fn create_request(
    pool: &Pool<Sqlite>,
    finding_id: String,
    removal_url: String,
) -> Result<GoogleRemovalRequest, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let timestamp_str = now.to_rfc3339();

    // Use INSERT OR IGNORE to make this idempotent - if a request already exists
    // for this finding_id (UNIQUE constraint), the insert will be silently ignored
    sqlx::query(
        "INSERT OR IGNORE INTO google_removal_requests
         (id, finding_id, status, google_removal_url, generated_at)
         VALUES (?, ?, 'URLGenerated', ?, ?)",
    )
    .bind(&id)
    .bind(&finding_id)
    .bind(&removal_url)
    .bind(&timestamp_str)
    .execute(pool)
    .await?;

    // Fetch the actual record (either the one we just created or the existing one)
    get_by_finding_id(pool, &finding_id)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)
}

/// Get a Google removal request by finding ID.
pub async fn get_by_finding_id(
    pool: &Pool<Sqlite>,
    finding_id: &str,
) -> Result<Option<GoogleRemovalRequest>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, finding_id, status, google_removal_url,
                generated_at, submitted_at, completed_at, notes
         FROM google_removal_requests
         WHERE finding_id = ?",
    )
    .bind(finding_id)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        // nosemgrep: use-zeroize-for-secrets
        let id: String = row.get("id");
        // nosemgrep: use-zeroize-for-secrets
        let finding_id: String = row.get("finding_id");
        // nosemgrep: use-zeroize-for-secrets
        let status_str: String = row.get("status");
        // nosemgrep: use-zeroize-for-secrets
        let google_removal_url: String = row.get("google_removal_url");
        // nosemgrep: use-zeroize-for-secrets
        let generated_at_str: String = row.get("generated_at");
        let submitted_at_str: Option<String> = row.get("submitted_at");
        let completed_at_str: Option<String> = row.get("completed_at");
        let notes: Option<String> = row.get("notes");

        let status = GoogleRemovalStatus::from_str(&status_str).map_err(|e| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e,
            )))
        })?;

        let generated_at = DateTime::parse_from_rfc3339(&generated_at_str)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?
            .with_timezone(&Utc);

        let submitted_at = submitted_at_str
            .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
            .transpose()
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        let completed_at = completed_at_str
            .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
            .transpose()
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        return Ok(Some(GoogleRemovalRequest {
            id,
            finding_id,
            status,
            google_removal_url,
            generated_at,
            submitted_at,
            completed_at,
            notes,
        }));
    }

    Ok(None)
}

/// Update the status of a Google removal request.
///
/// This is called when the user reports that they have submitted or completed
/// a removal request through Google's interface.
pub async fn update_status(
    pool: &Pool<Sqlite>,
    request_id: &str,
    status: GoogleRemovalStatus,
    notes: Option<String>,
) -> Result<(), sqlx::Error> {
    let timestamp = Utc::now().to_rfc3339();
    let status_str = status.as_str();

    match status {
        GoogleRemovalStatus::Submitted => {
            sqlx::query(
                "UPDATE google_removal_requests
                 SET status = ?, submitted_at = ?, notes = ?
                 WHERE id = ?",
            )
            .bind(status_str)
            .bind(&timestamp)
            .bind(&notes)
            .bind(request_id)
            .execute(pool)
            .await?;
        }
        GoogleRemovalStatus::Completed => {
            sqlx::query(
                "UPDATE google_removal_requests
                 SET status = ?, completed_at = ?, notes = ?
                 WHERE id = ?",
            )
            .bind(status_str)
            .bind(&timestamp)
            .bind(&notes)
            .bind(request_id)
            .execute(pool)
            .await?;
        }
        GoogleRemovalStatus::Failed => {
            sqlx::query(
                "UPDATE google_removal_requests
                 SET status = ?, notes = ?
                 WHERE id = ?",
            )
            .bind(status_str)
            .bind(&notes)
            .bind(request_id)
            .execute(pool)
            .await?;
        }
        GoogleRemovalStatus::URLGenerated => {
            // For URLGenerated status, just update the status field
            sqlx::query(
                "UPDATE google_removal_requests
                 SET status = ?
                 WHERE id = ?",
            )
            .bind(status_str)
            .bind(request_id)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_removal_url_name_only() {
        let url = generate_removal_url("John Doe", None, None);
        assert!(url.contains("John%20Doe"));
        assert!(url.starts_with("https://myactivity.google.com/results-about-you?q="));
    }

    #[test]
    fn test_generate_removal_url_with_address() {
        let url = generate_removal_url("John Doe", Some("123 Main St"), None);
        assert!(url.contains("John%20Doe"));
        assert!(url.contains("123%20Main%20St"));
    }

    #[test]
    fn test_generate_removal_url_with_phone() {
        let url = generate_removal_url("John Doe", None, Some("555-1234"));
        assert!(url.contains("John%20Doe"));
        assert!(url.contains("555-1234"));
    }

    #[test]
    fn test_generate_removal_url_all_fields() {
        let url = generate_removal_url("John Doe", Some("123 Main St"), Some("555-1234"));
        assert!(url.contains("John%20Doe"));
        assert!(url.contains("123%20Main%20St"));
        assert!(url.contains("555-1234"));
    }

    #[test]
    fn test_status_roundtrip() {
        let statuses = vec![
            GoogleRemovalStatus::URLGenerated,
            GoogleRemovalStatus::Submitted,
            GoogleRemovalStatus::Completed,
            GoogleRemovalStatus::Failed,
        ];

        for status in statuses {
            let str_val = status.as_str();
            // nosemgrep: no-unwrap-in-production
            let parsed = GoogleRemovalStatus::from_str(str_val).unwrap();
            assert_eq!(status.as_str(), parsed.as_str());
        }
    }
}
