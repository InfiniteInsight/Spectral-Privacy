//! Worker tasks for removal form submission.
//!
//! Handles async removal submission with retry logic, CAPTCHA detection,
//! and database state management.

use spectral_broker::removal::RemovalOutcome;
use spectral_vault::UserProfile;
use std::collections::HashMap;

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
}
