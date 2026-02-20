//! IMAP poller — monitors inbox for broker verification emails.

use std::collections::HashMap;

/// Maximum age of verification emails to search for
const VERIFICATION_WINDOW_DAYS: u64 = 7;
const SECONDS_PER_DAY: u64 = 86400;

/// Check if a sender address matches any known broker email address.
pub fn matches_broker_sender(sender: &str, broker_emails: &[String]) -> bool {
    broker_emails.iter().any(|b| b.eq_ignore_ascii_case(sender))
}

/// Configuration for the IMAP poller
#[derive(Clone)]
pub struct ImapConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

impl std::fmt::Debug for ImapConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImapConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

/// Result of a single polling pass
#[derive(Debug, Default)]
pub struct PollResult {
    pub verified: HashMap<String, String>,
    pub errors: Vec<String>,
}

/// Poll IMAP inbox for broker verification emails (SYNCHRONOUS - wrap in spawn_blocking if needed)
pub fn poll_for_verifications(
    config: &ImapConfig,
    broker_email_to_attempt: &HashMap<String, String>,
) -> PollResult {
    let mut result = PollResult::default();

    if broker_email_to_attempt.is_empty() {
        return result;
    }

    let mut session = match establish_imap_session(config, &mut result) {
        Some(s) => s,
        None => return result,
    };

    // Search for recent unseen messages (last 7 days)
    let seven_days_ago = {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let days_ago = now.saturating_sub(VERIFICATION_WINDOW_DAYS * SECONDS_PER_DAY);
        format_imap_date(days_ago)
    };

    let query = format!("UNSEEN SINCE {seven_days_ago}");
    let uids = match session.search(&query) {
        Ok(ids) => ids,
        Err(e) => {
            tracing::warn!("IMAP search error: {}", e);
            result.errors.push(format!("IMAP search error: {e}"));
            let _ = session.logout();
            return result;
        }
    };

    tracing::debug!("Found {} unseen messages in last 7 days", uids.len());

    if uids.is_empty() {
        let _ = session.logout();
        return result;
    }

    let uid_list: Vec<String> = uids.iter().map(|u| u.to_string()).collect();
    let fetch_query = uid_list.join(",");

    let messages = match session.fetch(&fetch_query, "RFC822.HEADER") {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("IMAP fetch error: {}", e);
            result.errors.push(format!("IMAP fetch error: {e}"));
            let _ = session.logout();
            return result;
        }
    };

    result.verified = extract_verifications_from_messages(messages.iter(), broker_email_to_attempt);

    let _ = session.logout();
    result
}

/// Establish IMAP session (connect + login + select INBOX)
fn establish_imap_session(
    config: &ImapConfig,
    result: &mut PollResult,
) -> Option<imap::Session<Box<dyn imap::ImapConnection>>> {
    tracing::debug!("Connecting to IMAP server {}:{}", config.host, config.port);

    let client = match imap::ClientBuilder::new(&config.host, config.port).connect() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("IMAP connect error: {}", e);
            result.errors.push(format!("IMAP connect error: {e}"));
            return None;
        }
    };

    let mut session = match client.login(&config.username, &config.password) {
        Ok(s) => s,
        Err((e, _)) => {
            tracing::warn!("IMAP login error: {}", e);
            result.errors.push(format!("IMAP login error: {e}"));
            return None;
        }
    };

    tracing::debug!("Successfully logged into IMAP server");

    if let Err(e) = session.select("INBOX") {
        tracing::warn!("IMAP select INBOX error: {}", e);
        result.errors.push(format!("IMAP select INBOX error: {e}"));
        let _ = session.logout();
        return None;
    }

    Some(session)
}

/// Extract verifications from fetched messages
fn extract_verifications_from_messages<'a, T>(
    messages: T,
    broker_email_to_attempt: &HashMap<String, String>,
) -> HashMap<String, String>
where
    T: IntoIterator<Item = &'a imap::types::Fetch<'a>>,
{
    let mut verified = HashMap::new();

    for msg in messages.into_iter() {
        if let Some(header_bytes) = msg.header() {
            let headers = String::from_utf8_lossy(header_bytes);
            if let Some(from) = extract_from_header(&headers) {
                if let Some(attempt_id) = broker_email_to_attempt.get(&from.to_lowercase()) {
                    tracing::info!(
                        "Found verification email from {} for attempt {}",
                        from,
                        attempt_id
                    );
                    verified.insert(from.clone(), attempt_id.clone());
                }
            }
        }
    }

    verified
}

fn extract_from_header(headers: &str) -> Option<String> {
    for line in headers.lines() {
        if line.to_ascii_lowercase().starts_with("from:") {
            let value = line[5..].trim();
            // Extract email from "From: Name <email@domain.com>" or "From: email@domain.com"
            if let Some(start) = value.find('<') {
                if let Some(end) = value.find('>') {
                    return Some(value[start + 1..end].to_lowercase());
                }
            }
            return Some(value.to_lowercase());
        }
    }
    None
}

fn format_imap_date(unix_secs: u64) -> String {
    use chrono::{DateTime, Utc};
    let dt = DateTime::<Utc>::from_timestamp(unix_secs as i64, 0).unwrap_or_else(Utc::now);
    dt.format("%d-%b-%Y").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_broker_email_exact() {
        let broker_emails = vec!["optout@spokeo.com".to_string()];
        assert!(matches_broker_sender("optout@spokeo.com", &broker_emails));
    }

    #[test]
    fn test_match_broker_email_no_match() {
        let broker_emails = vec!["optout@spokeo.com".to_string()];
        assert!(!matches_broker_sender("noreply@random.com", &broker_emails));
    }
}
