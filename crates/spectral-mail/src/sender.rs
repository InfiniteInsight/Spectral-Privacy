use crate::templates::EmailTemplate;
use sha2::{Digest, Sha256};

pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

/// Returns a `mailto:` URL for the given email.
pub fn to_mailto_url(email: &EmailTemplate) -> String {
    let subject = urlencoding::encode(&email.subject);
    let body = urlencoding::encode(&email.body);
    format!("mailto:{}?subject={}&body={}", email.to, subject, body)
}

/// Sends via SMTP using lettre.
pub async fn send_smtp(
    email: &EmailTemplate,
    from: &str,
    config: &SmtpConfig,
) -> Result<(), String> {
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{Message, SmtpTransport, Transport};

    let msg = Message::builder()
        .from(from.parse().map_err(|e| format!("Bad from address: {e}"))?)
        .to(email
            .to
            .parse()
            .map_err(|e| format!("Bad to address: {e}"))?)
        .subject(&email.subject)
        .body(email.body.clone())
        .map_err(|e| format!("Failed to build message: {e}"))?;

    let creds = Credentials::new(config.username.clone(), config.password.clone());
    let transport = SmtpTransport::relay(&config.host)
        .map_err(|e| format!("SMTP relay error: {e}"))?
        .port(config.port)
        .credentials(creds)
        .build();

    transport
        .send(&msg)
        .map_err(|e| format!("SMTP send failed: {e}"))?;
    Ok(())
}

/// Returns SHA-256 hex of email body (for logging — never store the body itself).
pub fn body_hash(body: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::EmailTemplate;

    #[test]
    fn test_mailto_url_format() {
        let email = EmailTemplate {
            to: "optout@broker.com".to_string(),
            subject: "Opt-Out Request".to_string(),
            body: "Please remove me.".to_string(),
        };
        let url = to_mailto_url(&email);
        assert!(url.starts_with("mailto:optout@broker.com?"));
        assert!(url.contains("subject="));
    }

    #[test]
    fn test_body_hash_is_deterministic() {
        let h1 = body_hash("hello");
        let h2 = body_hash("hello");
        assert_eq!(h1, h2);
        assert_ne!(h1, body_hash("world"));
    }
}
