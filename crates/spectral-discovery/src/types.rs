//! Shared types for PII discovery scanning

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for which PII types to scan for
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    /// Scan for email addresses
    pub scan_emails: bool,
    /// Scan for phone numbers
    pub scan_phones: bool,
    /// Scan for SSNs
    pub scan_ssn: bool,
    /// Scan for physical addresses
    pub scan_addresses: bool,
    /// Scan for names
    pub scan_names: bool,
    /// Scan for dates of birth
    pub scan_dob: bool,
    /// Optional custom directories to scan
    pub custom_directories: Option<Vec<PathBuf>>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            scan_emails: true,
            scan_phones: true,
            scan_ssn: true,
            scan_addresses: true,
            scan_names: true,
            scan_dob: true,
            custom_directories: None,
        }
    }
}

/// User's personally identifiable information
#[derive(Debug, Clone, Default)]
pub struct UserPii {
    /// Email addresses
    pub emails: Vec<String>,
    /// Phone numbers
    pub phones: Vec<String>,
    /// Social Security Number
    pub ssn: Option<String>,
    /// Physical addresses
    pub addresses: Vec<AddressInfo>,
    /// Names (full, first, last, nicknames)
    pub names: Vec<String>,
    /// Date of birth
    pub date_of_birth: Option<String>,
}

/// Physical address information
#[derive(Debug, Clone)]
pub struct AddressInfo {
    /// Street address
    pub street: Option<String>,
    /// City
    pub city: Option<String>,
    /// State
    pub state: Option<String>,
    /// ZIP code
    pub zip: Option<String>,
}

/// Types of personally identifiable information
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PiiType {
    /// Email address
    Email,
    /// Phone number
    Phone,
    /// Social Security Number
    Ssn,
    /// Physical address
    Address,
    /// Personal name
    Name,
    /// Date of birth
    DateOfBirth,
}

impl PiiType {
    /// Get string representation
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Phone => "phone",
            Self::Ssn => "ssn",
            Self::Address => "address",
            Self::Name => "name",
            Self::DateOfBirth => "dob",
        }
    }

    /// Get human-readable description
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Email => "Email Address",
            Self::Phone => "Phone Number",
            Self::Ssn => "Social Security Number",
            Self::Address => "Physical Address",
            Self::Name => "Personal Name",
            Self::DateOfBirth => "Date of Birth",
        }
    }

    /// Get risk level for this PII type
    #[must_use]
    pub fn risk_level(&self) -> &'static str {
        match self {
            Self::Ssn => "critical",
            Self::DateOfBirth => "high",
            Self::Address | Self::Phone | Self::Email => "medium",
            Self::Name => "low",
        }
    }
}

/// A single PII match found in a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiMatch {
    /// Type of PII found
    pub pii_type: PiiType,
    /// The actual matched value
    pub matched_value: String,
    /// Line number where found
    pub line_number: usize,
    /// Content of the line
    pub line_content: String,
}

/// Results from scanning a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileScanResult {
    /// File path
    pub path: PathBuf,
    /// All matches found in this file
    pub matches: Vec<PiiMatch>,
}

/// Progress update during a scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    /// Total files scanned so far
    pub files_scanned: usize,
    /// Files that had findings
    pub files_with_findings: usize,
    /// Current directory being scanned
    pub current_directory: String,
    /// Whether the scan is complete
    pub is_complete: bool,
    /// Whether the scan was stopped
    pub was_stopped: bool,
}

/// Commands that can be sent to the scanner
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanCommand {
    /// Continue scanning
    Continue,
    /// Pause the scan
    Pause,
    /// Stop the scan
    Stop,
}

/// File extensions that can be scanned for PII
pub const SCANNABLE_EXTENSIONS: &[&str] = &[
    "txt",
    "csv",
    "json",
    "md",
    "log",
    "xml",
    "yaml",
    "yml",
    "ini",
    "cfg",
    "conf",
    "properties",
    "toml",
    "html",
    "htm",
    "js",
    "ts",
    "jsx",
    "tsx",
    "py",
    "rb",
    "php",
    "java",
    "c",
    "cpp",
    "h",
    "rs",
    "go",
    "swift",
    "sql",
    "sh",
    "bash",
    "ps1",
];

/// Maximum file size to scan (10MB)
pub const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Maximum directory depth to traverse
pub const MAX_SCAN_DEPTH: usize = 15;
