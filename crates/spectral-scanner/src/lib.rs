//! Spectral Scanner - Automated broker scanning and result management
//!
//! This crate orchestrates scanning data broker sites to find user PII,
//! presents findings for user verification, and integrates with the removal system.

pub mod error;
pub mod url_builder;

pub use error::{Result, ScanError};
pub use url_builder::build_search_url;

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        // Placeholder test to verify crate builds
    }
}
