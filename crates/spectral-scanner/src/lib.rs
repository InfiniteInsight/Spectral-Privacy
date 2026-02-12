//! Spectral Scanner - Automated broker scanning and result management
//!
//! This crate orchestrates scanning data broker sites to find user PII,
//! presents findings for user verification, and integrates with the removal system.

pub mod error;

pub use error::{Result, ScanError};

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        // Placeholder test to verify crate builds
    }
}
