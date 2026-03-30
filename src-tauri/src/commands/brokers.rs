//! Broker explorer commands.
//!
//! Provides commands to list and query broker definitions.

use crate::error::CommandError;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use spectral_broker::definition::BrokerDefinition;

/// Indicates what kind of automated action is possible for a broker's search.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchMethodType {
    /// URL template — can auto-scan this broker
    Scannable,
    /// Web form — automation possible but requires form interaction
    WebForm,
    /// Manual — no automated search; user must act directly
    Manual,
}

/// Summary information about a broker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerSummary {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub category: String,
    pub difficulty: String,
    pub typical_removal_days: u32,
}

impl From<&BrokerDefinition> for BrokerSummary {
    fn from(def: &BrokerDefinition) -> Self {
        BrokerSummary {
            id: def.broker.id.to_string(),
            name: def.broker.name.clone(),
            domain: def.broker.domain.clone(),
            category: format!("{:?}", def.broker.category),
            difficulty: format!("{:?}", def.broker.difficulty),
            typical_removal_days: def.broker.typical_removal_days,
        }
    }
}

/// Email template information for removal.
#[derive(Debug, Serialize, Deserialize)]
pub struct EmailTemplate {
    pub email: String,
    pub subject: String,
    pub body: String,
    pub response_days: u32,
    pub notes: Option<String>,
}

/// Email fallback information when web forms fail.
#[derive(Debug, Serialize, Deserialize)]
pub struct EmailFallbackResponse {
    pub enabled: bool,
    pub email: String,
    pub phone: Option<String>,
    pub ccpa_phone: Option<String>,
    pub subject: Option<String>,
    pub subject_required: bool,
    pub required_fields: Vec<String>,
    pub processing_days: u32,
    pub email_template: Option<String>,
    pub notes: String,
    pub network_note: Option<String>,
    pub ccpa_compliant: bool,
    pub gdpr_compliant: bool,
}

/// Detailed information about a broker including user scan status.
#[derive(Debug, Serialize, Deserialize)]
pub struct BrokerDetail {
    #[serde(flatten)]
    pub summary: BrokerSummary,
    pub removal_method: String,
    pub url: String,
    pub recheck_interval_days: u32,
    pub last_verified: String,
    pub scan_status: Option<String>,
    pub finding_count: Option<i64>,
    pub email_template: Option<EmailTemplate>,
    pub email_fallback: Option<EmailFallbackResponse>,
    /// Whether this broker can be auto-scanned or requires manual action.
    pub search_method_type: SearchMethodType,
    /// URL for the removal action form, or the broker homepage for manual methods.
    pub removal_action_url: Option<String>,
}

/// List all broker definitions.
#[tauri::command]
pub async fn list_brokers(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<BrokerSummary>, CommandError> {
    let definitions = state.broker_definitions();
    Ok(definitions.iter().map(BrokerSummary::from).collect())
}

/// Get detailed information about a specific broker.
#[tauri::command]
pub async fn get_broker_detail(
    broker_id: String,
    vault_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<BrokerDetail, CommandError> {
    // Get broker definition
    let def = state.get_broker_definition(&broker_id).ok_or_else(|| {
        CommandError::new("BROKER_NOT_FOUND", format!("Broker {broker_id} not found"))
    })?;

    // Look up vault scan status for this broker
    let vault = state.get_vault(&vault_id).ok_or_else(|| {
        CommandError::new(
            "VAULT_NOT_UNLOCKED",
            format!("Vault {vault_id} not unlocked"),
        )
    })?;

    let db = vault.database().map_err(|e| {
        CommandError::new("DATABASE_ERROR", format!("Failed to access database: {e}"))
    })?;

    // Query for findings - count how many findings exist for this broker
    let finding_result: Result<(i64,), _> =
        sqlx::query_as("SELECT COUNT(*) as count FROM findings WHERE broker_id = ?")
            .bind(&broker_id)
            .fetch_one(db.pool())
            .await;

    let (scan_status, finding_count) = match finding_result {
        Ok((count,)) => {
            if count > 0 {
                (Some("Found".to_string()), Some(count))
            } else {
                (Some("NotFound".to_string()), Some(0))
            }
        }
        Err(_) => (None, None),
    };

    // Extract email template if removal method is Email
    let email_template = if let spectral_broker::definition::RemovalMethod::Email {
        email,
        subject,
        body,
        response_days,
        notes,
    } = &def.removal
    {
        Some(EmailTemplate {
            email: email.clone(),
            subject: subject.clone(),
            body: body.clone(),
            response_days: *response_days,
            notes: if notes.is_empty() {
                None
            } else {
                Some(notes.clone())
            },
        })
    } else {
        None
    };

    // Extract email fallback if present in WebForm or BrowserForm methods
    let email_fallback = match &def.removal {
        spectral_broker::definition::RemovalMethod::WebForm { email_fallback, .. }
        | spectral_broker::definition::RemovalMethod::BrowserForm { email_fallback, .. } => {
            email_fallback
                .as_ref()
                .map(|fallback| EmailFallbackResponse {
                    enabled: fallback.enabled,
                    email: fallback.email.clone(),
                    phone: fallback.phone.clone(),
                    ccpa_phone: fallback.ccpa_phone.clone(),
                    subject: fallback.subject.clone(),
                    subject_required: fallback.subject_required,
                    required_fields: fallback.required_fields.clone(),
                    processing_days: fallback.processing_days,
                    email_template: fallback.email_template.clone(),
                    notes: fallback.notes.clone(),
                    network_note: fallback.network_note.clone(),
                    ccpa_compliant: fallback.ccpa_compliant,
                    gdpr_compliant: fallback.gdpr_compliant,
                })
        }
        _ => None,
    };

    let search_method_type = match &def.search {
        spectral_broker::definition::SearchMethod::UrlTemplate { .. } => {
            SearchMethodType::Scannable
        }
        spectral_broker::definition::SearchMethod::WebForm { .. } => SearchMethodType::WebForm,
        spectral_broker::definition::SearchMethod::Manual { .. } => SearchMethodType::Manual,
    };

    let removal_action_url = match &def.removal {
        spectral_broker::definition::RemovalMethod::WebForm { url, .. }
        | spectral_broker::definition::RemovalMethod::BrowserForm { url, .. } => Some(url.clone()),
        spectral_broker::definition::RemovalMethod::Manual { .. } => Some(def.broker.url.clone()),
        spectral_broker::definition::RemovalMethod::Email { .. }
        | spectral_broker::definition::RemovalMethod::Phone { .. } => None,
    };

    Ok(BrokerDetail {
        summary: BrokerSummary::from(&def),
        removal_method: format!("{:?}", def.removal),
        url: def.broker.url.clone(),
        recheck_interval_days: def.broker.recheck_interval_days,
        last_verified: def.broker.last_verified.to_string(),
        scan_status,
        finding_count,
        email_template,
        email_fallback,
        search_method_type,
        removal_action_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral_broker::definition::{
        BrokerCategory, BrokerMetadata, ConfirmationType, FormSelectors, RemovalDifficulty,
        RemovalMethod,
    };
    use std::collections::HashMap;

    #[test]
    fn test_broker_summary_from_definition() {
        // Create minimal definition for testing
        let def = BrokerDefinition {
            broker: BrokerMetadata {
                id: spectral_core::BrokerId::new("spokeo").expect("valid broker id"),
                name: "Spokeo".to_string(),
                url: "https://spokeo.com".to_string(),
                domain: "spokeo.com".to_string(),
                category: BrokerCategory::PeopleSearch,
                difficulty: RemovalDifficulty::Easy,
                typical_removal_days: 7,
                recheck_interval_days: 30,
                last_verified: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid date"),
                scan_priority: spectral_broker::ScanPriority::OnRequest,
                region_relevance: vec!["Global".to_string()],
            },
            search: spectral_broker::definition::SearchMethod::UrlTemplate {
                template: "https://spokeo.com/{first}-{last}".to_string(),
                requires_fields: vec![spectral_core::PiiField::FirstName],
                result_selectors: None,
            },
            removal: RemovalMethod::WebForm {
                url: "https://spokeo.com/optout".to_string(),
                fields: {
                    let mut map = HashMap::new();
                    map.insert("email".to_string(), "{user_email}".to_string());
                    map
                },
                form_selectors: FormSelectors {
                    email_input: Some("#email".to_string()),
                    submit_button: "button[type=submit]".to_string(),
                    ..Default::default()
                },
                confirmation: ConfirmationType::EmailVerification,
                notes: String::new(),
                email_fallback: None,
            },
        };

        let summary = BrokerSummary::from(&def);
        assert_eq!(summary.id, "spokeo");
        assert_eq!(summary.name, "Spokeo");
        assert_eq!(summary.domain, "spokeo.com");
        assert_eq!(summary.typical_removal_days, 7);
    }
}
