pub mod imap;
pub mod sender;
pub mod templates;

pub use imap::{ImapConfig, PollResult};
pub use sender::SmtpConfig;
pub use templates::EmailTemplate;
