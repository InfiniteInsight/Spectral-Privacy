//! LLM feature commands (email drafting, form filling, etc.)

use crate::error::CommandError;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use spectral_privacy::{CompletionRequest, PrivacyAwareLlmRouter, TaskType};
use tauri::State;
use tracing::info;

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

    // Get vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| CommandError::new("VAULT_LOCKED", "Vault is locked"))?;

    let pool = vault
        .database()
        .map_err(|e| {
            CommandError::new(
                "VAULT_ERROR",
                format!("Failed to access vault database: {}", e),
            )
        })?
        .pool()
        .clone();

    // Create router
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
