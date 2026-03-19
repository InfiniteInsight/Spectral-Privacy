//! Spectral Discovery - PII scanning library

pub mod patterns;
pub mod scanner;
pub mod types;

pub use patterns::Matcher;
pub use scanner::{create_scanner_channels, ScanResult, Scanner};
pub use types::*;
