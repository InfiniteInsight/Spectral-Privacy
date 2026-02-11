// Placeholder for error types
pub type Result<T> = std::result::Result<T, BrowserError>;

#[derive(Debug, thiserror::Error)]
pub enum BrowserError {}
