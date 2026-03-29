//! Scheduler command handlers.

use crate::error::CommandError;
use crate::state::AppState;
use spectral_db::{Database, EncryptedPool};
use spectral_mail::sender::SmtpConfig;
use spectral_privacy::{llm_settings::TaskType, CompletionRequest, PrivacyAwareLlmRouter};
use spectral_scanner::{BrokerFilter, ScanOrchestrator};
use spectral_scheduler::{next_run_timestamp, JobType, ScheduledJob};
use sqlx::SqlitePool;
use std::sync::Arc;
use tracing::{error, info};

/// Interval for disabled jobs (far future to prevent execution)
const DISABLED_JOB_INTERVAL_DAYS: u32 = 365 * 10; // 10 years

#[tauri::command]
pub async fn get_scheduled_jobs(
    vault_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ScheduledJob>, CommandError> {
    let vault = state.get_vault(&vault_id).ok_or_else(|| {
        CommandError::new(
            "VAULT_NOT_UNLOCKED",
            format!("Vault {vault_id} not unlocked"),
        )
    })?;
    let db = vault.database().map_err(|e| {
        CommandError::new("DATABASE_ERROR", format!("Failed to access database: {e}"))
    })?;

    db.get_scheduled_jobs().await.map_err(|e| {
        CommandError::new(
            "DATABASE_ERROR",
            format!("Failed to get scheduled jobs: {e}"),
        )
    })
}

#[tauri::command]
pub async fn update_scheduled_job(
    vault_id: String,
    job_id: String,
    interval_days: u32,
    enabled: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    info!(
        "Updating job {} - interval: {}, enabled: {}",
        job_id, interval_days, enabled
    );

    let vault = state.get_vault(&vault_id).ok_or_else(|| {
        CommandError::new(
            "VAULT_NOT_UNLOCKED",
            format!("Vault {vault_id} not unlocked"),
        )
    })?;
    let db = vault.database().map_err(|e| {
        CommandError::new("DATABASE_ERROR", format!("Failed to access database: {e}"))
    })?;

    // Update interval and enabled status
    let next_run = if enabled {
        next_run_timestamp(interval_days)
    } else {
        // If disabled, set next_run far in future
        next_run_timestamp(DISABLED_JOB_INTERVAL_DAYS)
    };

    sqlx::query(
        "UPDATE scheduled_jobs SET interval_days = ?, enabled = ?, next_run_at = ? WHERE id = ?",
    )
    .bind(i64::from(interval_days))
    .bind(if enabled { 1 } else { 0 })
    .bind(&next_run)
    .bind(&job_id)
    .execute(db.pool())
    .await
    .map_err(|e| CommandError::new("DATABASE_ERROR", format!("Failed to update job: {e}")))?;

    Ok(())
}

#[tauri::command]
pub async fn run_job_now(
    vault_id: String,
    job_type: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    info!("Manual job trigger: {} for vault {}", job_type, vault_id);

    let job_type: JobType = serde_json::from_value(serde_json::Value::String(job_type.clone()))
        .map_err(|e| {
            CommandError::new(
                "INVALID_JOB_TYPE",
                format!("Invalid job type '{job_type}': {e}"),
            )
        })?;

    let vault = state.get_vault(&vault_id).ok_or_else(|| {
        CommandError::new(
            "VAULT_NOT_UNLOCKED",
            format!("Vault {vault_id} not unlocked"),
        )
    })?;

    match job_type {
        JobType::ScanAll => run_scan_all_job(&vault_id, vault, &state).await,
        JobType::VerifyRemovals => Err(CommandError::new(
            "NOT_IMPLEMENTED",
            "VerifyRemovals job type not yet implemented. This feature requires re-scanning brokers with submitted/completed removal attempts to verify removal success.".to_string(),
        )),
        JobType::PollImap => {
            let db = vault.database().map_err(|e| {
                CommandError::new("DATABASE_ERROR", format!("Failed to get vault database: {e}"))
            })?;
            run_poll_imap_job(db.pool()).await
        }
        JobType::FollowUpReminders => {
            let db = vault.database().map_err(|e| {
                CommandError::new("DATABASE_ERROR", format!("Failed to get vault database: {e}"))
            })?;
            run_followup_reminders_job(db.pool()).await
        }
    }
}

async fn run_scan_all_job(
    vault_id: &str,
    vault: std::sync::Arc<spectral_vault::Vault>,
    state: &AppState,
) -> Result<(), CommandError> {
    info!("Executing ScanAll job for vault {}", vault_id);

    let db = vault.database().map_err(|e| {
        CommandError::new(
            "DATABASE_ERROR",
            format!("Failed to get vault database: {e}"),
        )
    })?;
    let vault_key = vault
        .encryption_key()
        .map_err(|e| CommandError::new("VAULT_ERROR", format!("Failed to get vault key: {e}")))?;

    let profile_ids = vault.list_profiles().await.map_err(|e| {
        CommandError::new("DATABASE_ERROR", format!("Failed to list profiles: {e}"))
    })?;

    if profile_ids.is_empty() {
        return Err(CommandError::new(
            "NO_PROFILES",
            "No profiles found in vault. Create a profile first.".to_string(),
        ));
    }

    let profile_id = &profile_ids[0];
    info!("Using profile {} for scheduled scan", profile_id);

    let profile = vault
        .load_profile(profile_id)
        .await
        .map_err(|e| CommandError::new("DATABASE_ERROR", format!("Failed to load profile: {e}")))?;

    let browser_engine = state.get_or_init_browser_engine().await.map_err(|e| {
        CommandError::new(
            "BROWSER_ERROR",
            format!("Failed to get browser engine: {e}"),
        )
    })?;

    let pool = db.pool().clone();
    let vault_key_vec = vault_key.to_vec();
    let encrypted_pool = EncryptedPool::from_pool(pool, vault_key_vec);
    let database = Database::from_encrypted_pool(encrypted_pool);
    let db_arc = Arc::new(database);

    let orchestrator = ScanOrchestrator::new(state.broker_registry.clone(), browser_engine, db_arc)
        .with_max_concurrent_scans(4);

    info!("Starting scheduled scan with all auto-scan brokers");

    let _job_id = orchestrator
        .start_scan(&profile, BrokerFilter::All, vault_key)
        .await
        .map_err(|e| {
            error!("Scheduled scan failed: {}", e);
            CommandError::new("SCAN_ERROR", format!("Scan failed: {e}"))
        })?;

    info!("Scheduled scan started successfully");
    Ok(())
}

async fn run_poll_imap_job(pool: &SqlitePool) -> Result<(), CommandError> {
    info!("Executing PollImap job");

    let imap_config = spectral_mail::settings::get_imap_config(pool)
        .await
        .map_err(|e| {
            CommandError::new(
                "EMAIL_SETTINGS_ERROR",
                format!("Failed to load IMAP config: {e}"),
            )
        })?
        .ok_or_else(|| {
            info!("IMAP not configured — skipping PollImap job");
            CommandError::new(
                "IMAP_NOT_CONFIGURED",
                "IMAP is not configured. Enable IMAP in Settings → Email to use this feature."
                    .to_string(),
            )
        })?;

    let rows: Vec<(String, String)> = sqlx::query_as(
        r"
        SELECT er.recipient, er.attempt_id
        FROM email_removals er
        JOIN removal_attempts ra ON ra.id = er.attempt_id
        WHERE ra.status = 'Submitted'
        ",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        CommandError::new(
            "DATABASE_ERROR",
            format!("Failed to query email removals: {e}"),
        )
    })?;

    if rows.is_empty() {
        info!("No submitted email removal attempts — nothing to check");
        return Ok(());
    }

    let broker_map: std::collections::HashMap<String, String> = rows
        .into_iter()
        .map(|(email, attempt_id)| (email.to_lowercase(), attempt_id))
        .collect();

    info!(
        "Polling IMAP for {} pending email removal attempts",
        broker_map.len()
    );

    let result = tokio::task::spawn_blocking({
        let config = imap_config.clone();
        let map = broker_map.clone();
        move || spectral_mail::imap::poll_for_verifications(&config, &map)
    })
    .await
    .map_err(|e| CommandError::new("TASK_JOIN_ERROR", format!("IMAP task join error: {e}")))?;

    for err in &result.errors {
        tracing::warn!("IMAP poll error: {}", err);
    }

    if result.verified.is_empty() {
        info!("No new broker confirmations found in inbox");
        return Ok(());
    }

    info!(
        "Found {} broker confirmation(s) in inbox",
        result.verified.len()
    );

    let smtp_config = spectral_mail::settings::get_smtp_config(pool)
        .await
        .ok()
        .flatten();
    let cc_addr = spectral_mail::settings::get_cc_address(pool)
        .await
        .ok()
        .flatten();

    let llm_pool = pool.clone();
    let llm_available = spectral_privacy::get_primary_provider(&llm_pool)
        .await
        .ok()
        .flatten()
        .is_some();

    let now = chrono::Utc::now().to_rfc3339();
    for (broker_email, attempt_id) in &result.verified {
        sqlx::query(
            "UPDATE removal_attempts SET status = 'Completed', completed_at = ? WHERE id = ?",
        )
        .bind(&now)
        .bind(attempt_id)
        .execute(pool)
        .await
        .map_err(|e| {
            CommandError::new(
                "DATABASE_ERROR",
                format!("Failed to mark attempt {attempt_id} as completed: {e}"),
            )
        })?;

        info!("Marked removal attempt {} as Completed", attempt_id);

        if let (Some(body), Some(ref smtp), true) =
            (result.bodies.get(broker_email), &smtp_config, llm_available)
        {
            if let Err(e) = handle_llm_reply(
                body,
                broker_email,
                smtp,
                cc_addr.as_deref(),
                attempt_id,
                pool,
                &llm_pool,
            )
            .await
            {
                tracing::warn!("LLM reply failed for attempt {}: {}", attempt_id, e);
            }
        }
    }

    Ok(())
}

async fn run_followup_reminders_job(pool: &SqlitePool) -> Result<(), CommandError> {
    info!("Executing FollowUpReminders job");

    let due = spectral_db::get_due_followups(pool).await.map_err(|e| {
        CommandError::new(
            "DATABASE_ERROR",
            format!("Failed to fetch due follow-ups: {e}"),
        )
    })?;

    if due.is_empty() {
        info!("No follow-ups due");
        return Ok(());
    }

    info!("{} follow-up(s) due", due.len());

    let smtp_config = spectral_mail::settings::get_smtp_config(pool)
        .await
        .ok()
        .flatten();
    let llm_available = spectral_privacy::get_primary_provider(pool)
        .await
        .ok()
        .flatten()
        .is_some();

    for followup in &due {
        if let (Some(ref smtp), true) = (&smtp_config, llm_available) {
            match send_auto_followup(followup, smtp, pool).await {
                Ok(()) => {
                    if let Err(e) =
                        spectral_db::mark_followup_sent(pool, &followup.id, "smtp_auto").await
                    {
                        tracing::warn!("Failed to mark follow-up {} as sent: {}", followup.id, e);
                    } else {
                        info!(
                            "Auto-sent follow-up for attempt {} to {}",
                            followup.attempt_id, followup.recipient
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to auto-send follow-up for attempt {}: {}",
                        followup.attempt_id,
                        e
                    );
                }
            }
        } else {
            info!(
                "Follow-up for attempt {} due but LLM/SMTP not configured — surfacing in UI",
                followup.attempt_id
            );
        }
    }

    Ok(())
}

/// Draft and send an automated follow-up email using the LLM, then log it.
async fn send_auto_followup(
    followup: &spectral_db::RemovalFollowup,
    smtp: &SmtpConfig,
    pool: &SqlitePool,
) -> Result<(), String> {
    let router = PrivacyAwareLlmRouter::new(pool.clone());

    let prompt = format!(
        "Draft a short, professional follow-up email (3-5 sentences) for a data deletion \
        request sent 15 days ago to {} that has not yet been confirmed. Reference the original \
        CCPA/GDPR data deletion request. Ask for a status update and confirmation of deletion. \
        Provide only the email body — no subject line.",
        followup.recipient
    );

    let request = CompletionRequest::new(prompt).with_max_tokens(256);
    let response = router
        .route(TaskType::EmailDraft, request)
        .await
        .map_err(|e| format!("LLM request failed: {e}"))?;

    let body = response.content.trim().to_string();

    let template = spectral_mail::templates::EmailTemplate {
        to: followup.recipient.clone(),
        subject: "Follow-Up: Data Deletion Request".to_string(),
        body: body.clone(),
    };

    spectral_mail::sender::send_smtp(&template, &smtp.username, smtp, None)
        .await
        .map_err(|e| format!("SMTP send failed: {e}"))?;

    let now = chrono::Utc::now().to_rfc3339();
    let body_hash = spectral_mail::sender::body_hash(&body);
    sqlx::query(
        "INSERT INTO email_removals \
         (id, attempt_id, broker_id, recipient, method, subject, body_hash, sent_at) \
         VALUES (lower(hex(randomblob(16))), ?, ?, ?, 'smtp_followup', \
                 'Follow-Up: Data Deletion Request', ?, ?)",
    )
    .bind(&followup.attempt_id)
    .bind(&followup.broker_id)
    .bind(&followup.recipient)
    .bind(&body_hash)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to log follow-up to email_removals: {e}"))?;

    Ok(())
}

/// Analyze a broker reply email with an LLM and, if a response is warranted, send it via SMTP.
///
/// Logs the reply to `email_removals` with `send_method = 'smtp_reply'`.
async fn handle_llm_reply(
    body: &str,
    broker_email: &str,
    smtp: &SmtpConfig,
    cc: Option<&str>,
    attempt_id: &str,
    pool: &SqlitePool,
    llm_pool: &SqlitePool,
) -> Result<(), String> {
    let router = PrivacyAwareLlmRouter::new(llm_pool.clone());

    let prompt = format!(
        "You received the following email from a data broker in response to a data removal request:\n\n\
        ---\n{body}\n---\n\n\
        Does this email require a reply from the user? \
        If the broker is asking for additional information or confirmation, draft a short, \
        professional reply (2-4 sentences) that provides whatever is needed to complete the removal. \
        Begin your reply with REPLY: followed by the reply text.\n\
        If no reply is needed (the removal is confirmed or the email is informational), \
        respond with exactly: NO_REPLY_NEEDED"
    );

    let request = CompletionRequest::new(prompt).with_max_tokens(512);

    let response = router
        .route(TaskType::EmailDraft, request)
        .await
        .map_err(|e| format!("LLM request failed: {e}"))?;

    let content = response.content.trim();

    if content.starts_with("NO_REPLY_NEEDED") {
        tracing::debug!("LLM determined no reply needed for attempt {}", attempt_id);
        return Ok(());
    }

    let reply_body = if let Some(stripped) = content.strip_prefix("REPLY:") {
        stripped.trim().to_string()
    } else {
        // LLM didn't follow format — treat the whole response as the reply body
        content.to_string()
    };

    let from_addr = &smtp.username;
    let reply_template = spectral_mail::templates::EmailTemplate {
        to: broker_email.to_string(),
        subject: "Re: Data Removal Request".to_string(),
        body: reply_body,
    };

    spectral_mail::sender::send_smtp(&reply_template, from_addr, smtp, cc)
        .await
        .map_err(|e| format!("SMTP reply send failed: {e}"))?;

    tracing::info!(
        "Sent LLM-drafted reply to {} for attempt {}",
        broker_email,
        attempt_id
    );

    let now = chrono::Utc::now().to_rfc3339();
    let subject_hash = spectral_mail::sender::body_hash("Re: Data Removal Request");
    sqlx::query(
        "INSERT INTO email_removals (id, attempt_id, broker_id, recipient, method, subject, body_hash, sent_at)
         VALUES (lower(hex(randomblob(16))), ?, '', ?, 'smtp_reply', 'Re: Data Removal Request', ?, ?)",
    )
    .bind(attempt_id)
    .bind(broker_email)
    .bind(&subject_hash)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to log reply to email_removals: {e}"))?;

    Ok(())
}
