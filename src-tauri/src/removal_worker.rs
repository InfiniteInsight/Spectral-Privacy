//! Worker tasks for removal form submission.
//!
//! Handles async removal submission with retry logic, CAPTCHA detection,
//! and database state management.

use spectral_broker::removal::RemovalOutcome;
use spectral_vault::UserProfile;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{error, warn};

/// Result of a removal submission worker task.
#[derive(Debug)]
pub struct WorkerResult {
    pub removal_attempt_id: String,
    pub outcome: RemovalOutcome,
}

/// Map profile and finding data to form fields.
///
/// Extracts required fields from profile and finding for form submission.
pub fn map_fields_for_submission(
    profile: &UserProfile,
    finding_listing_url: &str,
    key: &[u8; 32],
) -> Result<HashMap<String, String>, String> {
    let mut fields = HashMap::new();

    // listing_url from finding
    fields.insert("listing_url".to_string(), finding_listing_url.to_string());

    // Email from profile (required)
    let email = profile
        .email
        .as_ref()
        .ok_or("Missing required field: email")?
        .decrypt(key)
        .map_err(|e| format!("Failed to decrypt email: {}", e))?;
    fields.insert("email".to_string(), email);

    // First name (required)
    let first_name = profile
        .first_name
        .as_ref()
        .ok_or("Missing required field: first_name")?
        .decrypt(key)
        .map_err(|e| format!("Failed to decrypt first_name: {}", e))?;
    fields.insert("first_name".to_string(), first_name);

    // Last name (required)
    let last_name = profile
        .last_name
        .as_ref()
        .ok_or("Missing required field: last_name")?
        .decrypt(key)
        .map_err(|e| format!("Failed to decrypt last_name: {}", e))?;
    fields.insert("last_name".to_string(), last_name);

    Ok(fields)
}

/// Retry a task with exponential backoff.
///
/// Attempts the task up to `max_attempts` times with increasing delays:
/// - After 1st failure: 30 seconds
/// - After 2nd failure: 2 minutes
/// - After 3rd+ failure: 5 minutes
///
/// Returns `Ok(T)` on success or `Err(E)` if all attempts are exhausted.
pub async fn retry_with_backoff<F, Fut, T, E>(mut task_fn: F, max_attempts: u32) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    let delays = [
        Duration::from_secs(30),     // 30 seconds
        Duration::from_secs(2 * 60), // 2 minutes
        Duration::from_secs(5 * 60), // 5 minutes
    ];

    for attempt in 1..=max_attempts {
        match task_fn().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempt >= max_attempts {
                    error!(
                        "Task failed after {} attempts (max: {})",
                        attempt, max_attempts
                    );
                    return Err(e);
                }

                let delay = if attempt == 1 {
                    delays[0]
                } else if attempt == 2 {
                    delays[1]
                } else {
                    delays[2]
                };

                warn!(
                    "Task failed on attempt {}/{}. Retrying in {:?}...",
                    attempt, max_attempts, delay
                );

                tokio::time::sleep(delay).await;
            }
        }
    }

    unreachable!("Loop should have returned via Ok or Err")
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral_core::types::{ProfileId, Timestamp};
    use spectral_vault::cipher::EncryptedField;

    fn test_key() -> [u8; 32] {
        [0u8; 32]
    }

    #[allow(deprecated)]
    fn create_test_profile(key: &[u8; 32]) -> UserProfile {
        UserProfile {
            id: ProfileId::generate(),
            full_name: None,
            first_name: Some(
                EncryptedField::encrypt(&"John".to_string(), key).expect("encrypt first_name"),
            ),
            middle_name: None,
            last_name: Some(
                EncryptedField::encrypt(&"Doe".to_string(), key).expect("encrypt last_name"),
            ),
            email: Some(
                EncryptedField::encrypt(&"john@example.com".to_string(), key)
                    .expect("encrypt email"),
            ),
            phone: None,
            address: None,
            city: None,
            state: None,
            zip_code: None,
            country: None,
            date_of_birth: None,
            ssn: None,
            employer: None,
            job_title: None,
            education: None,
            social_media: None,
            previous_addresses_v1: None,
            phone_numbers: vec![],
            previous_addresses_v2: vec![],
            aliases: vec![],
            relatives: vec![],
            created_at: Timestamp::now(),
            updated_at: Timestamp::now(),
        }
    }

    #[test]
    fn test_map_fields_success() {
        let key = test_key();
        let profile = create_test_profile(&key);
        let listing_url = "https://spokeo.com/person/123";

        let fields = map_fields_for_submission(&profile, listing_url, &key).expect("map fields");

        assert_eq!(fields.get("listing_url"), Some(&listing_url.to_string()));
        assert_eq!(fields.get("email"), Some(&"john@example.com".to_string()));
        assert_eq!(fields.get("first_name"), Some(&"John".to_string()));
        assert_eq!(fields.get("last_name"), Some(&"Doe".to_string()));
    }

    #[test]
    fn test_map_fields_missing_email() {
        let key = test_key();
        let mut profile = create_test_profile(&key);
        profile.email = None;
        let listing_url = "https://spokeo.com/person/123";

        let result = map_fields_for_submission(&profile, listing_url, &key);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Missing required field: email"));
    }

    #[tokio::test]
    async fn test_retry_with_backoff_succeeds_on_second_attempt() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();

        let task = || {
            let count = attempt_count_clone.clone();
            async move {
                // nosemgrep: llm-prompt-injection-risk
                let current = count.fetch_add(1, Ordering::SeqCst) + 1;
                if current < 2 {
                    Err("Transient error")
                } else {
                    Ok("Success")
                }
            }
        };

        let result = retry_with_backoff(task, 3).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
        assert_eq!(attempt_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_retry_with_backoff_exhausts_attempts() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();

        let task = || {
            let count = attempt_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Err::<(), _>("Persistent error")
            }
        };

        let result = retry_with_backoff(task, 3).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Persistent error");
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
    }
}
