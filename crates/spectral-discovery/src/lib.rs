//! Spectral Discovery Module
//!
//! Local PII discovery for scanning filesystems, browsers, and email.

pub mod filesystem;

// Re-export main types
pub use filesystem::{
    is_scannable, scan_directory, scan_file, FileScanResult, PiiMatch, PiiPatterns,
};
