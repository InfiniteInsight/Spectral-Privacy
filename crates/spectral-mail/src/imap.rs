//! IMAP poller — monitors inbox for broker verification emails.

use std::collections::HashMap;

/// Check if a sender address matches any known broker email address.
pub fn matches_broker_sender(sender: &str, broker_emails: &[String]) -> bool {
    broker_emails.iter().any(|b| b.eq_ignore_ascii_case(sender))
}

/// Configuration for the IMAP poller
#[derive(Clone, Debug)]
pub struct ImapConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
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

    // Connect with rustls TLS (handled by rustls-tls feature flag)
    let client = match imap::ClientBuilder::new(&config.host, config.port).connect() {
        Ok(c) => c,
        Err(e) => {
            result.errors.push(format!("IMAP connect error: {e}"));
            return result;
        }
    };

    let mut session = match client.login(&config.username, &config.password) {
        Ok(s) => s,
        Err((e, _)) => {
            result.errors.push(format!("IMAP login error: {e}"));
            return result;
        }
    };

    if let Err(e) = session.select("INBOX") {
        result.errors.push(format!("IMAP select INBOX error: {e}"));
        let _ = session.logout();
        return result;
    }

    // Search for recent unseen messages (last 7 days)
    let seven_days_ago = {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let days_ago = now.saturating_sub(7 * 24 * 3600);
        format_imap_date(days_ago)
    };

    let query = format!("UNSEEN SINCE {seven_days_ago}");
    let uids = match session.search(&query) {
        Ok(ids) => ids,
        Err(e) => {
            result.errors.push(format!("IMAP search error: {e}"));
            let _ = session.logout();
            return result;
        }
    };

    if uids.is_empty() {
        let _ = session.logout();
        return result;
    }

    let uid_list: Vec<String> = uids.iter().map(|u| u.to_string()).collect();
    let fetch_query = uid_list.join(",");

    let messages = match session.fetch(&fetch_query, "RFC822.HEADER") {
        Ok(m) => m,
        Err(e) => {
            result.errors.push(format!("IMAP fetch error: {e}"));
            let _ = session.logout();
            return result;
        }
    };

    for msg in messages.iter() {
        if let Some(header_bytes) = msg.header() {
            let headers = String::from_utf8_lossy(header_bytes);
            if let Some(from) = extract_from_header(&headers) {
                if let Some(attempt_id) = broker_email_to_attempt.get(&from.to_lowercase()) {
                    result.verified.insert(from.clone(), attempt_id.clone());
                }
            }
        }
    }

    let _ = session.logout();
    result
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
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let days = unix_secs / 86400;
    let y = 1970 + (days / 365) as u32; // nosemgrep: llm-prompt-injection-risk
    let day_of_year = days % 365;
    let month_idx = (day_of_year / 30).min(11) as usize;
    let day = (day_of_year % 30) + 1; // nosemgrep: llm-prompt-injection-risk
    format!("{:02}-{}-{}", day, months[month_idx], y)
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
