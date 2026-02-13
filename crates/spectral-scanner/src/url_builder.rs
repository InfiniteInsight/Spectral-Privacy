#![allow(clippy::uninlined_format_args)]

use crate::error::{Result, ScanError};
use spectral_broker::SearchMethod;
use spectral_core::BrokerId;
use spectral_vault::UserProfile;

/// Simple URL encoding for profile data
/// Encodes spaces as hyphens and removes special characters
fn url_encode_simple(s: &str) -> String {
    s.chars()
        .filter_map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                Some(c)
            } else if c.is_whitespace() {
                Some('-')
            } else {
                None
            }
        })
        .collect()
}

pub fn build_search_url(
    broker_id: &BrokerId,
    method: &SearchMethod,
    profile: &UserProfile,
    key: &[u8; 32],
) -> Result<String> {
    match method {
        SearchMethod::UrlTemplate { template, .. } => {
            let mut url = template.clone();

            // Replace placeholders
            if let Some(first) = &profile.first_name {
                let decrypted = first
                    .decrypt(key)
                    .map_err(|e| ScanError::ProfileDataError {
                        broker_id: broker_id.clone(),
                        reason: format!("Failed to decrypt first_name: {}", e),
                    })?;
                let encoded = url_encode_simple(&decrypted.to_lowercase());
                url = url.replace("{first}", &encoded);
            }
            if let Some(last) = &profile.last_name {
                let decrypted = last.decrypt(key).map_err(|e| ScanError::ProfileDataError {
                    broker_id: broker_id.clone(),
                    reason: format!("Failed to decrypt last_name: {}", e),
                })?;
                let encoded = url_encode_simple(&decrypted.to_lowercase());
                url = url.replace("{last}", &encoded);
            }
            if let Some(state) = &profile.state {
                let decrypted = state
                    .decrypt(key)
                    .map_err(|e| ScanError::ProfileDataError {
                        broker_id: broker_id.clone(),
                        reason: format!("Failed to decrypt state: {}", e),
                    })?;
                let encoded = url_encode_simple(&decrypted);
                url = url.replace("{state}", &encoded);
            }
            if let Some(city) = &profile.city {
                let decrypted = city.decrypt(key).map_err(|e| ScanError::ProfileDataError {
                    broker_id: broker_id.clone(),
                    reason: format!("Failed to decrypt city: {}", e),
                })?;
                let encoded = url_encode_simple(&decrypted.to_lowercase());
                url = url.replace("{city}", &encoded);
            }

            Ok(url)
        }
        _ => Err(ScanError::ProfileDataError {
            broker_id: broker_id.clone(),
            reason: "URL building only supported for UrlTemplate search method".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral_core::{PiiField, ProfileId};
    use spectral_vault::cipher::encrypt_string;

    fn test_key() -> [u8; 32] {
        [0x42; 32]
    }

    fn mock_profile() -> UserProfile {
        let key = test_key();
        let mut profile = UserProfile::new(ProfileId::generate());
        profile.first_name = Some(encrypt_string("John", &key).expect("encrypt first_name"));
        profile.last_name = Some(encrypt_string("Doe", &key).expect("encrypt last_name"));
        profile.state = Some(encrypt_string("CA", &key).expect("encrypt state"));
        profile.city = Some(encrypt_string("Springfield", &key).expect("encrypt city"));
        profile
    }

    #[test]
    fn test_build_url_from_template() {
        let broker_id = BrokerId::new("test-broker").expect("valid broker id");
        let method = SearchMethod::UrlTemplate {
            template: "https://example.com/{first}-{last}/{state}/{city}".to_string(),
            requires_fields: vec![
                PiiField::FirstName,
                PiiField::LastName,
                PiiField::State,
                PiiField::City,
            ],
            result_selectors: None,
        };

        let profile = mock_profile();
        let key = test_key();
        let url = build_search_url(&broker_id, &method, &profile, &key)
            .expect("should build URL from template");

        assert_eq!(url, "https://example.com/john-doe/CA/springfield");
    }
}
