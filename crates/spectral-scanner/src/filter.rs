#![allow(clippy::must_use_candidate)]
#![allow(clippy::match_same_arms)]

use serde::{Deserialize, Serialize};
use spectral_broker::{BrokerDefinition, SearchMethod};
use spectral_core::PiiField;
use spectral_vault::UserProfile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrokerFilter {
    All,
    Category(String),
    Specific(Vec<String>),
}

impl BrokerFilter {
    pub fn matches(&self, broker: &BrokerDefinition) -> bool {
        match self {
            BrokerFilter::All => true,
            BrokerFilter::Category(cat) => {
                // Serialize the category to kebab-case for comparison
                // This should never fail as BrokerCategory is a simple enum with serde derive
                let category_str = serde_json::to_string(&broker.broker.category)
                    .expect("BrokerCategory serialization should never fail")
                    .trim_matches('"')
                    .to_string();
                &category_str == cat
            }
            BrokerFilter::Specific(ids) => ids.iter().any(|id| broker.broker.id.as_str() == id),
        }
    }
}

/// Checks if the user profile contains all required fields for a broker.
///
/// # Parameters
/// * `broker` - The broker definition containing search field requirements
/// * `profile` - The user profile to check for completeness
/// * `_key` - Encryption key (reserved for future use when validating field decryptability)
///
/// # Returns
/// * `Ok(())` if all required fields are present
/// * `Err(Vec<PiiField>)` containing the list of missing fields
pub fn check_profile_completeness(
    broker: &BrokerDefinition,
    profile: &UserProfile,
    _key: &[u8; 32],
) -> Result<(), Vec<PiiField>> {
    let requires_fields = match &broker.search {
        SearchMethod::UrlTemplate {
            requires_fields, ..
        } => requires_fields,
        SearchMethod::WebForm {
            requires_fields, ..
        } => requires_fields,
        SearchMethod::Manual { .. } => return Ok(()),
    };

    let mut missing = Vec::new();

    for field in requires_fields {
        #[allow(deprecated)]
        let is_present = match field {
            PiiField::FullName => profile.full_name.is_some(),
            PiiField::FirstName => profile.first_name.is_some(),
            PiiField::LastName => profile.last_name.is_some(),
            PiiField::MiddleName => profile.middle_name.is_some(),
            PiiField::DateOfBirth => profile.date_of_birth.is_some(),
            PiiField::Age => {
                // Age can be derived from date_of_birth if present
                profile.date_of_birth.is_some()
            }
            PiiField::State => profile.state.is_some(),
            PiiField::City => profile.city.is_some(),
            PiiField::ZipCode => profile.zip_code.is_some(),
            PiiField::Address => profile.address.is_some(),
            PiiField::Phone => profile.phone.is_some() || !profile.phone_numbers.is_empty(),
            PiiField::Email => profile.email.is_some(),
            PiiField::Country => profile.country.is_some(),
            PiiField::Ssn => profile.ssn.is_some(),
            PiiField::Employer => profile.employer.is_some(),
            PiiField::JobTitle => profile.job_title.is_some(),
            PiiField::Education => profile.education.is_some(),
            PiiField::SocialMedia => profile.social_media.is_some(),
            PiiField::Relatives => !profile.relatives.is_empty(),
            PiiField::PreviousAddress => !profile.previous_addresses_v2.is_empty(),
            // Fields we don't track in profile
            PiiField::IpAddress | PiiField::Photo | PiiField::Other => false,
        };

        if !is_present {
            missing.push(*field);
        }
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use spectral_broker::{BrokerCategory, BrokerMetadata, RemovalDifficulty, RemovalMethod};
    use spectral_core::{BrokerId, ProfileId};
    use spectral_vault::EncryptedField;

    fn mock_broker(category: BrokerCategory, requires: Vec<PiiField>) -> BrokerDefinition {
        BrokerDefinition {
            broker: BrokerMetadata {
                id: BrokerId::new("test").expect("valid test broker ID"),
                name: "Test".to_string(),
                url: "https://example.com".to_string(),
                domain: "example.com".to_string(),
                category,
                difficulty: RemovalDifficulty::Easy,
                typical_removal_days: 7,
                recheck_interval_days: 30,
                last_verified: NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid test date"),
                scan_priority: spectral_broker::ScanPriority::OnRequest,
                region_relevance: vec!["Global".to_string()],
            },
            search: SearchMethod::UrlTemplate {
                template: "https://example.com/{first}-{last}".to_string(),
                requires_fields: requires,
                result_selectors: None,
            },
            removal: RemovalMethod::Manual {
                instructions: "Manual removal".to_string(),
            },
        }
    }

    #[test]
    fn test_filter_all() {
        let broker = mock_broker(BrokerCategory::PeopleSearch, vec![]);
        assert!(BrokerFilter::All.matches(&broker));
    }

    #[test]
    fn test_filter_category() {
        let broker1 = mock_broker(BrokerCategory::PeopleSearch, vec![]);
        let broker2 = mock_broker(BrokerCategory::DataAggregator, vec![]);

        let filter = BrokerFilter::Category("people-search".to_string());
        assert!(filter.matches(&broker1));
        assert!(!filter.matches(&broker2));
    }

    #[test]
    fn test_profile_completeness_missing_fields() {
        let broker = mock_broker(
            BrokerCategory::PeopleSearch,
            vec![PiiField::FirstName, PiiField::LastName, PiiField::State],
        );

        let profile_id =
            ProfileId::new("550e8400-e29b-41d4-a716-446655440000").expect("valid test profile ID");
        let mut profile = UserProfile::new(profile_id);
        let key = [0x42; 32];

        profile.first_name = Some(
            EncryptedField::encrypt(&"John".to_string(), &key)
                .expect("encryption should succeed in test"),
        );
        // Missing last_name and state

        let result = check_profile_completeness(&broker, &profile, &key);
        assert!(result.is_err());
        let missing = result.unwrap_err();
        assert_eq!(missing.len(), 2);
        assert!(missing.contains(&PiiField::LastName));
        assert!(missing.contains(&PiiField::State));
    }

    #[test]
    fn test_filter_specific_empty() {
        let broker = mock_broker(BrokerCategory::PeopleSearch, vec![]);
        let filter = BrokerFilter::Specific(vec![]);
        assert!(!filter.matches(&broker));
    }

    #[test]
    fn test_profile_completeness_all_fields_present() {
        let broker = mock_broker(
            BrokerCategory::PeopleSearch,
            vec![PiiField::FirstName, PiiField::LastName, PiiField::State],
        );

        let profile_id =
            ProfileId::new("550e8400-e29b-41d4-a716-446655440000").expect("valid test profile ID");
        let mut profile = UserProfile::new(profile_id);
        let key = [0x42; 32];

        profile.first_name = Some(
            EncryptedField::encrypt(&"John".to_string(), &key)
                .expect("encryption should succeed in test"),
        );
        profile.last_name = Some(
            EncryptedField::encrypt(&"Doe".to_string(), &key)
                .expect("encryption should succeed in test"),
        );
        profile.state = Some(
            EncryptedField::encrypt(&"CA".to_string(), &key)
                .expect("encryption should succeed in test"),
        );

        let result = check_profile_completeness(&broker, &profile, &key);
        assert!(result.is_ok());
    }

    #[test]
    fn test_manual_search_method_always_succeeds() {
        let broker = BrokerDefinition {
            broker: BrokerMetadata {
                id: BrokerId::new("test").expect("valid test broker ID"),
                name: "Test".to_string(),
                url: "https://example.com".to_string(),
                domain: "example.com".to_string(),
                category: BrokerCategory::PeopleSearch,
                difficulty: RemovalDifficulty::Easy,
                typical_removal_days: 7,
                recheck_interval_days: 30,
                last_verified: NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid test date"),
                scan_priority: spectral_broker::ScanPriority::OnRequest,
                region_relevance: vec!["Global".to_string()],
            },
            search: SearchMethod::Manual {
                url: "https://example.com/search".to_string(),
                instructions: "Manual search".to_string(),
            },
            removal: RemovalMethod::Manual {
                instructions: "Manual removal".to_string(),
            },
        };

        let profile_id =
            ProfileId::new("550e8400-e29b-41d4-a716-446655440000").expect("valid test profile ID");
        let profile = UserProfile::new(profile_id);
        let key = [0x42; 32];

        let result = check_profile_completeness(&broker, &profile, &key);
        assert!(result.is_ok());
    }
}
