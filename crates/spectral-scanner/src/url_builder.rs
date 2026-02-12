use crate::error::{Result, ScanError};
use spectral_broker::SearchMethod;
use spectral_core::BrokerId;
use spectral_vault::UserProfile;

pub fn build_search_url(
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
                    .map_err(|e| ScanError::SelectorsOutdated {
                        broker_id: BrokerId::new("unknown").expect("valid broker id"),
                        reason: format!("Failed to decrypt first_name: {}", e),
                    })?;
                url = url.replace("{first}", &decrypted.to_lowercase());
            }
            if let Some(last) = &profile.last_name {
                let decrypted = last
                    .decrypt(key)
                    .map_err(|e| ScanError::SelectorsOutdated {
                        broker_id: BrokerId::new("unknown").expect("valid broker id"),
                        reason: format!("Failed to decrypt last_name: {}", e),
                    })?;
                url = url.replace("{last}", &decrypted.to_lowercase());
            }
            if let Some(state) = &profile.state {
                let decrypted = state
                    .decrypt(key)
                    .map_err(|e| ScanError::SelectorsOutdated {
                        broker_id: BrokerId::new("unknown").expect("valid broker id"),
                        reason: format!("Failed to decrypt state: {}", e),
                    })?;
                url = url.replace("{state}", &decrypted);
            }
            if let Some(city) = &profile.city {
                let decrypted = city
                    .decrypt(key)
                    .map_err(|e| ScanError::SelectorsOutdated {
                        broker_id: BrokerId::new("unknown").expect("valid broker id"),
                        reason: format!("Failed to decrypt city: {}", e),
                    })?;
                url = url.replace("{city}", &decrypted.to_lowercase().replace(" ", "-"));
            }

            Ok(url)
        }
        _ => Err(ScanError::SelectorsOutdated {
            broker_id: BrokerId::new("unknown").expect("valid broker id"),
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
        let url =
            build_search_url(&method, &profile, &key).expect("should build URL from template");

        assert_eq!(url, "https://example.com/john-doe/CA/springfield");
    }
}
