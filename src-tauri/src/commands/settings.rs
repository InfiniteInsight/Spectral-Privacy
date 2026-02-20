use crate::error::CommandError;

#[tauri::command]
pub async fn test_smtp_connection(
    host: String,
    port: u16,
    username: String,
    password: String,
) -> Result<(), CommandError> {
    // Stub implementation - will use spectral-mail in later task
    let _ = (host, port, username, password);
    Ok(())
}

#[tauri::command]
pub async fn test_imap_connection(
    host: String,
    port: u16,
    username: String,
    password: String,
) -> Result<(), CommandError> {
    use spectral_mail::imap::{poll_for_verifications, ImapConfig};
    use std::collections::HashMap;

    let config = ImapConfig {
        host,
        port,
        username,
        password,
    };

    // Run synchronous IMAP polling in blocking task
    let result =
        tokio::task::spawn_blocking(move || poll_for_verifications(&config, &HashMap::new()))
            .await
            .map_err(|e| CommandError::new("TASK_JOIN_ERROR", format!("Task join error: {}", e)))?;

    if let Some(err) = result.errors.first() {
        return Err(CommandError::new("IMAP_CONNECTION_ERROR", err.clone()));
    }

    Ok(())
}
