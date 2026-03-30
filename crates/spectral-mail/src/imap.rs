//! IMAP poller — monitors inbox for broker verification emails.

use std::collections::HashMap;

/// Maximum age of verification emails to search for
const VERIFICATION_WINDOW_DAYS: u64 = 7;
const SECONDS_PER_DAY: u64 = 86400;

/// Check if a sender address matches any known broker email address.
#[must_use]
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
    /// broker_email → attempt_id for each email matched to a known broker
    pub verified: HashMap<String, String>,
    /// broker_email → plain-text message body for matched emails (for LLM reply analysis)
    pub bodies: HashMap<String, String>,
    pub errors: Vec<String>,
}

/// Poll IMAP inbox for broker verification emails (SYNCHRONOUS - wrap in `spawn_blocking` if needed)
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

    // Fetch full RFC822 messages so we can extract both headers and body
    let messages = match session.fetch(&fetch_query, "RFC822") {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("IMAP fetch error: {}", e);
            result.errors.push(format!("IMAP fetch error: {e}"));
            let _ = session.logout();
            return result;
        }
    };

    let (verified, bodies) =
        extract_verifications_from_messages(messages.iter(), broker_email_to_attempt);
    result.verified = verified;
    result.bodies = bodies;

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

/// Extract verifications and message bodies from fetched messages.
///
/// Returns `(verified, bodies)` where:
/// - `verified`: broker_email → attempt_id
/// - `bodies`: broker_email → plain-text body (for LLM reply analysis)
fn extract_verifications_from_messages<'a, T>(
    messages: T,
    broker_email_to_attempt: &HashMap<String, String>,
) -> (HashMap<String, String>, HashMap<String, String>)
where
    T: IntoIterator<Item = &'a imap::types::Fetch<'a>>,
{
    let mut verified = HashMap::new();
    let mut bodies = HashMap::new();

    for msg in messages.into_iter() {
        // RFC822 fetch returns the full raw message in body()
        let raw = msg.body().map(String::from_utf8_lossy);
        if let Some(raw_str) = raw {
            if let Some(from) = extract_from_header_raw(&raw_str) {
                if let Some(attempt_id) = broker_email_to_attempt.get(&from.to_lowercase()) {
                    tracing::info!(
                        "Found verification email from {} for attempt {}",
                        from,
                        attempt_id
                    );
                    verified.insert(from.clone(), attempt_id.clone());
                    // Extract plain-text body (everything after the header block)
                    let body_text = extract_body_text(&raw_str);
                    bodies.insert(from.clone(), body_text);
                }
            }
        }
    }

    (verified, bodies)
}

/// Extract plain-text body from a raw RFC822 message (everything after the blank line).
fn extract_body_text(raw: &str) -> String {
    // RFC822 headers end at the first blank line (\r\n\r\n or \n\n)
    if let Some(pos) = raw.find("\r\n\r\n") {
        raw[pos + 4..].trim().to_string()
    } else if let Some(pos) = raw.find("\n\n") {
        raw[pos + 2..].trim().to_string()
    } else {
        String::new()
    }
}

/// Extract the From address from a raw RFC822 message (headers + body).
fn extract_from_header_raw(raw: &str) -> Option<String> {
    // Only look in the header section (before the blank line)
    let header_section = if let Some(pos) = raw.find("\r\n\r\n") {
        &raw[..pos]
    } else if let Some(pos) = raw.find("\n\n") {
        &raw[..pos]
    } else {
        raw
    };
    extract_from_header(header_section)
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
