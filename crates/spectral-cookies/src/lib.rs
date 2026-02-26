//! Browser cookie scanning and removal for Spectral.
//!
//! This crate provides functionality to:
//! - Detect installed browsers (Chrome, Firefox, Safari, Edge, Brave)
//! - Read cookie databases from each browser
//! - Match cookies against broker definitions
//! - Safely remove cookies with backup/restore capability
//! - Handle browser locks and concurrent access

pub mod browser;
pub mod error;
pub mod matcher;
pub mod remover;
pub mod scanner;

pub use browser::{Browser, BrowserProfile, BrowserType};
pub use error::{CookieError, Result};
pub use matcher::{BrokerCookiePattern, CookieMatcher};
pub use remover::{CookieRemover, RemovalResult};
pub use scanner::{CookieScanResult, CookieScanner, ScannedCookie};

/// Cookie data structure representing a browser cookie.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub creation_time: Option<i64>,
    pub expiry_time: Option<i64>,
    pub last_access_time: Option<i64>,
    pub is_secure: bool,
    pub is_httponly: bool,
    pub same_site: Option<SameSite>,
}

/// SameSite cookie attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SameSite {
    None,
    Lax,
    Strict,
}

impl std::fmt::Display for SameSite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SameSite::None => write!(f, "None"),
            SameSite::Lax => write!(f, "Lax"),
            SameSite::Strict => write!(f, "Strict"),
        }
    }
}

impl std::str::FromStr for SameSite {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "None" => Ok(SameSite::None),
            "Lax" => Ok(SameSite::Lax),
            "Strict" => Ok(SameSite::Strict),
            _ => Err(format!("Invalid SameSite value: {}", s)),
        }
    }
}
