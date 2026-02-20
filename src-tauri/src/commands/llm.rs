//! LLM feature commands (email drafting, form filling, etc.)

use crate::error::CommandError;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use spectral_privacy::{CompletionRequest, PrivacyAwareLlmRouter, TaskType};
use sqlx::SqlitePool;
use tauri::State;
use tracing::info;

/// Helper function to get database pool for a vault.
///
/// # Errors
/// Returns `CommandError` if vault is locked or database access fails.
fn get_vault_pool(state: &AppState, vault_id: &str) -> Result<SqlitePool, CommandError> {
    let vault = state
        .get_vault(vault_id)
        .ok_or_else(|| CommandError::new("VAULT_LOCKED", "Vault is locked"))?;

    let pool = vault
        .database()
        .map_err(|e| {
            CommandError::new(
                "VAULT_ERROR",
                format!("Failed to access vault database: {e}"),
            )
        })?
        .pool()
        .clone();

    Ok(pool)
}

/// Email draft request parameters
#[derive(Debug, Deserialize)]
pub struct EmailDraftRequest {
    /// The context or instructions for the email
    pub prompt: String,
    /// Optional recipient information
    pub recipient: Option<String>,
    /// Optional subject hint
    pub subject: Option<String>,
    /// Optional tone preference (e.g., "formal", "casual")
    pub tone: Option<String>,
}

/// Email draft response metadata
#[derive(Debug, Serialize)]
pub struct EmailDraftMetadata {
    /// The provider used for generation
    pub provider: String,
    /// Whether PII filtering was applied
    pub pii_filtered: bool,
}

/// Email draft response
#[derive(Debug, Serialize)]
pub struct EmailDraftResponse {
    /// The generated email subject
    pub subject: String,
    /// The generated email body
    pub body: String,
    /// Metadata about the generation
    pub metadata: Option<EmailDraftMetadata>,
}

/// Build a prompt for email drafting
fn build_email_prompt(request: &EmailDraftRequest) -> String {
    let mut parts = vec!["Draft an email with the following requirements:".to_string()];

    parts.push(format!("Instructions: {}", request.prompt));

    if let Some(recipient) = &request.recipient {
        parts.push(format!("Recipient: {}", recipient));
    }

    if let Some(subject) = &request.subject {
        parts.push(format!("Subject hint: {}", subject));
    }

    if let Some(tone) = &request.tone {
        parts.push(format!("Tone: {}", tone));
    }

    parts.push("\nProvide the response in the following format:".to_string());
    parts.push("Subject: [email subject]".to_string());
    parts.push("Body: [email body]".to_string());

    parts.join("\n")
}

/// Parse email draft response from LLM
fn parse_email_response(content: &str) -> Result<(String, String), CommandError> {
    let mut subject = String::new();
    let mut body_lines = Vec::new();
    let mut in_body = false;

    for line in content.lines() {
        if line.starts_with("Subject:") {
            subject = line.trim_start_matches("Subject:").trim().to_string();
        } else if line.starts_with("Body:") {
            in_body = true;
            let body_start = line.trim_start_matches("Body:").trim();
            if !body_start.is_empty() {
                body_lines.push(body_start.to_string());
            }
        } else if in_body {
            body_lines.push(line.to_string());
        }
    }

    if subject.is_empty() {
        subject = "Email Draft".to_string();
    }

    let body = body_lines.join("\n").trim().to_string();
    if body.is_empty() {
        return Err(CommandError::new(
            "LLM_PARSE_ERROR",
            "Failed to parse email body from LLM response",
        ));
    }

    Ok((subject, body))
}

/// Draft an email using LLM.
///
/// Uses the privacy-aware LLM router to generate an email draft based on the given prompt.
/// The router will select an appropriate LLM provider based on privacy settings and
/// task preferences, and apply PII filtering if needed.
#[tauri::command]
pub async fn draft_email(
    state: State<'_, AppState>,
    vault_id: String,
    request: EmailDraftRequest,
) -> Result<EmailDraftResponse, CommandError> {
    info!("Drafting email for vault: {}", vault_id);

    let pool = get_vault_pool(&state, &vault_id)?;
    let router = PrivacyAwareLlmRouter::new(pool);

    // Build prompt
    let prompt = build_email_prompt(&request);

    // Make LLM request
    let completion_request = CompletionRequest::new(&prompt);
    let response = router
        .route(TaskType::EmailDraft, completion_request)
        .await
        .map_err(|e| {
            CommandError::new(
                "LLM_ERROR",
                format!("Failed to generate email draft: {}", e),
            )
        })?;

    // Parse response
    let (subject, body) = parse_email_response(&response.content)?;

    Ok(EmailDraftResponse {
        subject,
        body,
        metadata: Some(EmailDraftMetadata {
            provider: "stub".to_string(), // TODO: Get actual provider from router
            pii_filtered: false,          // TODO: Get actual filtering status
        }),
    })
}

// ============================================================================
// Form Filling
// ============================================================================

/// A form field to be filled
#[derive(Debug, Deserialize)]
pub struct FormField {
    /// Field identifier or name
    pub name: String,
    /// Field label or description
    pub label: String,
    /// Field type (e.g., "text", "email", "phone", "address")
    #[serde(rename = "type")]
    pub field_type: String,
    /// Whether this field is required
    pub required: Option<bool>,
}

/// Form filling request parameters
#[derive(Debug, Deserialize)]
pub struct FormFillingRequest {
    /// The form fields to fill
    pub fields: Vec<FormField>,
    /// Optional context about the form or purpose
    pub context: Option<String>,
}

/// Form filling response metadata
#[derive(Debug, Serialize)]
pub struct FormFillingMetadata {
    /// The provider used for generation
    pub provider: String,
    /// Whether PII filtering was applied
    pub pii_filtered: bool,
    /// Number of fields filled
    pub fields_filled: usize,
}

/// Form filling response
#[derive(Debug, Serialize)]
pub struct FormFillingResponse {
    /// Filled field values, keyed by field name
    pub values: std::collections::HashMap<String, String>,
    /// Metadata about the generation
    pub metadata: Option<FormFillingMetadata>,
}

/// Build a prompt for form filling
fn build_form_prompt(request: &FormFillingRequest) -> String {
    let mut parts = vec!["Fill the following form fields with appropriate values:".to_string()];

    if let Some(context) = &request.context {
        parts.push(format!("Context: {}", context));
    }

    parts.push("\nFields:".to_string());
    for field in &request.fields {
        let required = if field.required.unwrap_or(false) {
            " (required)"
        } else {
            ""
        };
        parts.push(format!(
            "- {} ({}): {}{}",
            field.name, field.field_type, field.label, required
        ));
    }

    parts.push("\nProvide the response in the following format:".to_string());
    parts.push("field_name: value".to_string());
    parts.push("\nOnly provide realistic values appropriate for each field type.".to_string());

    parts.join("\n")
}

/// Parse form filling response from LLM
fn parse_form_response(
    content: &str,
    fields: &[FormField],
) -> Result<std::collections::HashMap<String, String>, CommandError> {
    use std::collections::HashMap;

    let mut values = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse "field_name: value" format
        if let Some(colon_pos) = line.find(':') {
            let field_name = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();

            // Validate field exists
            if fields.iter().any(|f| f.name == field_name) {
                values.insert(field_name, value);
            }
        }
    }

    if values.is_empty() {
        return Err(CommandError::new(
            "LLM_PARSE_ERROR",
            "Failed to parse form values from LLM response",
        ));
    }

    Ok(values)
}

/// Fill a form using LLM.
///
/// Uses the privacy-aware LLM router to generate form field values based on
/// the field definitions and optional context. The router will select an
/// appropriate LLM provider based on privacy settings and apply PII filtering.
#[tauri::command]
pub async fn fill_form(
    state: State<'_, AppState>,
    vault_id: String,
    request: FormFillingRequest,
) -> Result<FormFillingResponse, CommandError> {
    info!("Filling form for vault: {}", vault_id);

    let pool = get_vault_pool(&state, &vault_id)?;
    let router = PrivacyAwareLlmRouter::new(pool);

    // Build prompt
    let prompt = build_form_prompt(&request);

    // Make LLM request
    let completion_request = CompletionRequest::new(&prompt);
    let response = router
        .route(TaskType::FormFill, completion_request)
        .await
        .map_err(|e| CommandError::new("LLM_ERROR", format!("Failed to fill form: {}", e)))?;

    // Parse response
    let values = parse_form_response(&response.content, &request.fields)?;
    let fields_filled = values.len();

    Ok(FormFillingResponse {
        values,
        metadata: Some(FormFillingMetadata {
            provider: "stub".to_string(), // TODO: Get actual provider from router
            pii_filtered: false,          // TODO: Get actual filtering status
            fields_filled,
        }),
    })
}
