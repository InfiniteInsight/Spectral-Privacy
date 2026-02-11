//! Broker definition types and structures.
//!
//! This module defines the data structures for broker definitions loaded from TOML files.

use crate::error::{BrokerError, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use spectral_core::{BrokerId, PiiField};
use std::collections::HashMap;

/// Complete broker definition loaded from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerDefinition {
    /// Core broker metadata
    pub broker: BrokerMetadata,

    /// Search configuration
    pub search: SearchMethod,

    /// Removal/opt-out configuration
    pub removal: RemovalMethod,
}

impl BrokerDefinition {
    /// Get the broker ID.
    #[must_use]
    pub fn id(&self) -> &BrokerId {
        &self.broker.id
    }

    /// Get the broker name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.broker.name
    }

    /// Get the broker category.
    #[must_use]
    pub fn category(&self) -> BrokerCategory {
        self.broker.category
    }

    /// Validate the broker definition for completeness and correctness.
    pub fn validate(&self) -> Result<()> {
        // Validate broker metadata
        if self.broker.name.is_empty() {
            return Err(BrokerError::ValidationError {
                broker_id: self.broker.id.to_string(),
                reason: "broker name cannot be empty".to_string(),
            });
        }

        if self.broker.url.is_empty() {
            return Err(BrokerError::ValidationError {
                broker_id: self.broker.id.to_string(),
                reason: "broker URL cannot be empty".to_string(),
            });
        }

        // Validate typical_removal_days is reasonable
        if self.broker.typical_removal_days == 0 || self.broker.typical_removal_days > 365 {
            return Err(BrokerError::ValidationError {
                broker_id: self.broker.id.to_string(),
                reason: format!(
                    "typical_removal_days must be 1-365, got {}",
                    self.broker.typical_removal_days
                ),
            });
        }

        // Validate recheck_interval_days is reasonable
        if self.broker.recheck_interval_days == 0 || self.broker.recheck_interval_days > 365 {
            return Err(BrokerError::ValidationError {
                broker_id: self.broker.id.to_string(),
                reason: format!(
                    "recheck_interval_days must be 1-365, got {}",
                    self.broker.recheck_interval_days
                ),
            });
        }

        // Validate search method
        self.search.validate(&self.broker.id)?;

        // Validate removal method
        self.removal.validate(&self.broker.id)?;

        Ok(())
    }
}

/// Core broker metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerMetadata {
    /// Unique broker identifier (e.g., "spokeo", "beenverified")
    pub id: BrokerId,

    /// Human-readable broker name
    pub name: String,

    /// Broker website URL
    pub url: String,

    /// Broker domain (e.g., "spokeo.com")
    pub domain: String,

    /// Broker category
    pub category: BrokerCategory,

    /// Difficulty level for removal
    pub difficulty: RemovalDifficulty,

    /// Typical number of days for removal to complete
    pub typical_removal_days: u32,

    /// Days between rechecks after removal
    pub recheck_interval_days: u32,

    /// Date when this definition was last verified (YYYY-MM-DD)
    pub last_verified: NaiveDate,
}

/// Categories of data brokers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BrokerCategory {
    /// People search engines (Spokeo, `BeenVerified`, etc.)
    PeopleSearch,
    /// Background check services
    BackgroundCheck,
    /// Data aggregators
    DataAggregator,
    /// Financial/credit data
    Financial,
    /// Government records
    GovernmentRecords,
    /// Marketing data brokers
    Marketing,
    /// Social media aggregators
    SocialMedia,
    /// Other/uncategorized
    Other,
}

impl BrokerCategory {
    /// Get a human-readable display name.
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::PeopleSearch => "People Search",
            Self::BackgroundCheck => "Background Check",
            Self::DataAggregator => "Data Aggregator",
            Self::Financial => "Financial",
            Self::GovernmentRecords => "Government Records",
            Self::Marketing => "Marketing",
            Self::SocialMedia => "Social Media",
            Self::Other => "Other",
        }
    }
}

/// Difficulty level for removal from a broker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RemovalDifficulty {
    /// Simple web form, quick response
    Easy,
    /// Requires email verification or multiple steps
    Medium,
    /// Requires manual intervention, phone calls, or legal action
    Hard,
}

/// Methods for searching a broker site.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "kebab-case")]
pub enum SearchMethod {
    /// URL template with variable substitution
    #[serde(rename = "url-template")]
    UrlTemplate {
        /// URL template with placeholders (e.g., `https://example.com/{first}-{last}/{state}`)
        template: String,
        /// PII fields required for search
        requires_fields: Vec<PiiField>,
    },

    /// Web form that needs to be filled out
    WebForm {
        /// URL of the search form
        url: String,
        /// Form field mappings
        fields: HashMap<String, String>,
        /// PII fields required for search
        requires_fields: Vec<PiiField>,
    },

    /// Requires manual search (no automation possible)
    Manual {
        /// URL to start manual search
        url: String,
        /// Instructions for manual search
        instructions: String,
    },
}

impl SearchMethod {
    /// Validate the search method configuration.
    fn validate(&self, broker_id: &BrokerId) -> Result<()> {
        match self {
            Self::UrlTemplate {
                template,
                requires_fields,
            } => {
                if template.is_empty() {
                    return Err(BrokerError::ValidationError {
                        broker_id: broker_id.to_string(),
                        reason: "URL template cannot be empty".to_string(),
                    });
                }
                if requires_fields.is_empty() {
                    return Err(BrokerError::ValidationError {
                        broker_id: broker_id.to_string(),
                        reason: "UrlTemplate requires at least one PII field".to_string(),
                    });
                }
            }
            Self::WebForm {
                url,
                fields,
                requires_fields,
            } => {
                if url.is_empty() {
                    return Err(BrokerError::ValidationError {
                        broker_id: broker_id.to_string(),
                        reason: "WebForm URL cannot be empty".to_string(),
                    });
                }
                if fields.is_empty() {
                    return Err(BrokerError::ValidationError {
                        broker_id: broker_id.to_string(),
                        reason: "WebForm requires at least one field mapping".to_string(),
                    });
                }
                if requires_fields.is_empty() {
                    return Err(BrokerError::ValidationError {
                        broker_id: broker_id.to_string(),
                        reason: "WebForm requires at least one PII field".to_string(),
                    });
                }
            }
            Self::Manual { url, instructions } => {
                if url.is_empty() {
                    return Err(BrokerError::ValidationError {
                        broker_id: broker_id.to_string(),
                        reason: "Manual search URL cannot be empty".to_string(),
                    });
                }
                if instructions.is_empty() {
                    return Err(BrokerError::ValidationError {
                        broker_id: broker_id.to_string(),
                        reason: "Manual search instructions cannot be empty".to_string(),
                    });
                }
            }
        }
        Ok(())
    }
}

/// CSS selectors for web form elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormSelectors {
    /// Selector for listing URL input
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listing_url_input: Option<String>,

    /// Selector for email input
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_input: Option<String>,

    /// Selector for first name input
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name_input: Option<String>,

    /// Selector for last name input
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name_input: Option<String>,

    /// Selector for submit button
    pub submit_button: String,

    /// Selector for CAPTCHA iframe or container
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captcha_frame: Option<String>,

    /// Selector for success confirmation message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_indicator: Option<String>,
}

/// Methods for removal/opt-out from a broker.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "kebab-case")]
pub enum RemovalMethod {
    /// Web form submission
    #[serde(rename = "web-form")]
    WebForm {
        /// URL of the opt-out form
        url: String,
        /// Form field mappings (e.g., "`listing_url`" -> "`{found_listing_url}`")
        fields: HashMap<String, String>,
        /// CSS selectors for form elements
        form_selectors: FormSelectors,
        /// Confirmation method
        confirmation: ConfirmationType,
        /// Additional notes or instructions
        #[serde(default)]
        notes: String,
    },

    /// Email-based removal
    Email {
        /// Email address for removal requests
        email: String,
        /// Email subject template
        subject: String,
        /// Email body template
        body: String,
        /// Expected response time in days
        response_days: u32,
        /// Additional notes
        #[serde(default)]
        notes: String,
    },

    /// Phone-based removal
    Phone {
        /// Phone number to call
        phone: String,
        /// Instructions for phone call
        instructions: String,
    },

    /// Manual process with instructions
    Manual {
        /// Instructions for manual removal
        instructions: String,
    },
}

impl RemovalMethod {
    /// Validate the removal method configuration.
    fn validate(&self, broker_id: &BrokerId) -> Result<()> {
        match self {
            Self::WebForm {
                url,
                fields,
                form_selectors,
                ..
            } => Self::validate_web_form(broker_id, url, fields, form_selectors),
            Self::Email {
                email,
                subject,
                body,
                response_days,
                ..
            } => Self::validate_email(broker_id, email, subject, body, *response_days),
            Self::Phone {
                phone,
                instructions,
            } => Self::validate_phone(broker_id, phone, instructions),
            Self::Manual { instructions } => Self::validate_manual(broker_id, instructions),
        }
    }

    fn validate_web_form(
        broker_id: &BrokerId,
        url: &str,
        fields: &HashMap<String, String>,
        form_selectors: &FormSelectors,
    ) -> Result<()> {
        if url.is_empty() {
            return Err(BrokerError::ValidationError {
                broker_id: broker_id.to_string(),
                reason: "removal.url cannot be empty for web-form method".to_string(),
            });
        }

        if fields.is_empty() {
            return Err(BrokerError::ValidationError {
                broker_id: broker_id.to_string(),
                reason: "removal.fields cannot be empty for web-form method".to_string(),
            });
        }

        if form_selectors.submit_button.is_empty() {
            return Err(BrokerError::ValidationError {
                broker_id: broker_id.to_string(),
                reason: "removal.form_selectors.submit_button is required".to_string(),
            });
        }

        Ok(())
    }

    fn validate_email(
        broker_id: &BrokerId,
        email: &str,
        subject: &str,
        body: &str,
        response_days: u32,
    ) -> Result<()> {
        if email.is_empty() {
            return Err(BrokerError::ValidationError {
                broker_id: broker_id.to_string(),
                reason: "Email removal requires email address".to_string(),
            });
        }
        if subject.is_empty() {
            return Err(BrokerError::ValidationError {
                broker_id: broker_id.to_string(),
                reason: "Email removal requires subject template".to_string(),
            });
        }
        if body.is_empty() {
            return Err(BrokerError::ValidationError {
                broker_id: broker_id.to_string(),
                reason: "Email removal requires body template".to_string(),
            });
        }
        if response_days == 0 || response_days > 90 {
            return Err(BrokerError::ValidationError {
                broker_id: broker_id.to_string(),
                reason: format!("response_days must be 1-90, got {response_days}"),
            });
        }
        Ok(())
    }

    fn validate_phone(broker_id: &BrokerId, phone: &str, instructions: &str) -> Result<()> {
        if phone.is_empty() {
            return Err(BrokerError::ValidationError {
                broker_id: broker_id.to_string(),
                reason: "Phone removal requires phone number".to_string(),
            });
        }
        if instructions.is_empty() {
            return Err(BrokerError::ValidationError {
                broker_id: broker_id.to_string(),
                reason: "Phone removal requires instructions".to_string(),
            });
        }
        Ok(())
    }

    fn validate_manual(broker_id: &BrokerId, instructions: &str) -> Result<()> {
        if instructions.is_empty() {
            return Err(BrokerError::ValidationError {
                broker_id: broker_id.to_string(),
                reason: "Manual removal requires instructions".to_string(),
            });
        }
        Ok(())
    }
}

/// How removal confirmation is handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfirmationType {
    /// Requires email verification via link
    EmailVerification,
    /// Automatic confirmation (no follow-up needed)
    Automatic,
    /// Manual verification required (check back later)
    Manual,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broker_category_display() {
        assert_eq!(BrokerCategory::PeopleSearch.display_name(), "People Search");
        assert_eq!(
            BrokerCategory::BackgroundCheck.display_name(),
            "Background Check"
        );
    }

    #[test]
    fn test_removal_difficulty_ordering() {
        assert!(RemovalDifficulty::Easy < RemovalDifficulty::Medium);
        assert!(RemovalDifficulty::Medium < RemovalDifficulty::Hard);
    }

    #[test]
    fn test_search_method_validation() {
        let broker_id = BrokerId::new("test-broker").expect("valid broker ID");

        // Valid URL template
        let method = SearchMethod::UrlTemplate {
            template: "https://example.com/{first}-{last}".to_string(),
            requires_fields: vec![PiiField::FirstName, PiiField::LastName],
        };
        assert!(method.validate(&broker_id).is_ok());

        // Empty template should fail
        let method = SearchMethod::UrlTemplate {
            template: String::new(),
            requires_fields: vec![PiiField::FirstName],
        };
        assert!(method.validate(&broker_id).is_err());

        // No required fields should fail
        let method = SearchMethod::UrlTemplate {
            template: "https://example.com/{first}-{last}".to_string(),
            requires_fields: vec![],
        };
        assert!(method.validate(&broker_id).is_err());
    }

    #[test]
    fn test_removal_method_validation() {
        let broker_id = BrokerId::new("test-broker").expect("valid broker ID");

        // Valid web form
        let mut fields = HashMap::new();
        fields.insert("email".to_string(), "{user_email}".to_string());
        let form_selectors = FormSelectors {
            listing_url_input: Some("#listing-url".to_string()),
            email_input: Some("input[name='email']".to_string()),
            first_name_input: None,
            last_name_input: None,
            submit_button: "button[type='submit']".to_string(),
            captcha_frame: None,
            success_indicator: Some(".success".to_string()),
        };
        let method = RemovalMethod::WebForm {
            url: "https://example.com/optout".to_string(),
            fields,
            form_selectors,
            confirmation: ConfirmationType::EmailVerification,
            notes: String::new(),
        };
        assert!(method.validate(&broker_id).is_ok());

        // Empty URL should fail
        let mut fields = HashMap::new();
        fields.insert("email".to_string(), "{user_email}".to_string());
        let form_selectors = FormSelectors {
            listing_url_input: Some("#listing-url".to_string()),
            email_input: Some("input[name='email']".to_string()),
            first_name_input: None,
            last_name_input: None,
            submit_button: "button[type='submit']".to_string(),
            captcha_frame: None,
            success_indicator: Some(".success".to_string()),
        };
        let method = RemovalMethod::WebForm {
            url: String::new(),
            fields,
            form_selectors,
            confirmation: ConfirmationType::EmailVerification,
            notes: String::new(),
        };
        assert!(method.validate(&broker_id).is_err());

        // Valid email removal
        let method = RemovalMethod::Email {
            email: "privacy@example.com".to_string(),
            subject: "Removal Request".to_string(),
            body: "Please remove my data".to_string(),
            response_days: 7,
            notes: String::new(),
        };
        assert!(method.validate(&broker_id).is_ok());

        // Invalid response days
        let method = RemovalMethod::Email {
            email: "privacy@example.com".to_string(),
            subject: "Removal Request".to_string(),
            body: "Please remove my data".to_string(),
            response_days: 0,
            notes: String::new(),
        };
        assert!(method.validate(&broker_id).is_err());
    }

    #[test]
    fn test_broker_definition_validation() {
        let broker_id = BrokerId::new("test-broker").expect("valid broker ID");
        let mut fields = HashMap::new();
        fields.insert("email".to_string(), "{user_email}".to_string());

        let form_selectors = FormSelectors {
            listing_url_input: Some("#listing-url".to_string()),
            email_input: Some("input[name='email']".to_string()),
            first_name_input: None,
            last_name_input: None,
            submit_button: "button[type='submit']".to_string(),
            captcha_frame: None,
            success_indicator: Some(".success".to_string()),
        };

        let definition = BrokerDefinition {
            broker: BrokerMetadata {
                id: broker_id.clone(),
                name: "Test Broker".to_string(),
                url: "https://test.com".to_string(),
                domain: "test.com".to_string(),
                category: BrokerCategory::PeopleSearch,
                difficulty: RemovalDifficulty::Easy,
                typical_removal_days: 7,
                recheck_interval_days: 30,
                last_verified: NaiveDate::from_ymd_opt(2025, 5, 1).expect("valid date"),
            },
            search: SearchMethod::UrlTemplate {
                template: "https://test.com/{first}-{last}".to_string(),
                requires_fields: vec![PiiField::FirstName, PiiField::LastName],
            },
            removal: RemovalMethod::WebForm {
                url: "https://test.com/optout".to_string(),
                fields,
                form_selectors,
                confirmation: ConfirmationType::EmailVerification,
                notes: String::new(),
            },
        };

        assert!(definition.validate().is_ok());

        // Test invalid typical_removal_days
        let mut invalid_def = definition.clone();
        invalid_def.broker.typical_removal_days = 0;
        assert!(invalid_def.validate().is_err());

        // Test invalid recheck_interval_days
        let mut invalid_def = definition.clone();
        invalid_def.broker.recheck_interval_days = 500;
        assert!(invalid_def.validate().is_err());

        // Test empty name
        let mut invalid_def = definition;
        invalid_def.broker.name = String::new();
        assert!(invalid_def.validate().is_err());
    }
}
