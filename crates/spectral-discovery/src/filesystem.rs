//! Filesystem PII discovery scanner
//!
//! Scans local files for personally identifiable information (PII)
//! including email addresses, phone numbers, and SSNs.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, warn};

/// Maximum file size to scan (100MB)
const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

/// Maximum directory depth to scan
const MAX_SCAN_DEPTH: usize = 10;

/// Compiled regex patterns (initialized once at startup)
static EMAIL_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b")
        .expect("Email regex is hardcoded and valid")
});

static PHONE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:\+?1[-.\s]?)?(?:\([0-9]{3}\)|[0-9]{3})[-.\s]?[0-9]{3}[-.\s]?[0-9]{4}")
        .expect("Phone regex is hardcoded and valid")
});

static SSN_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").expect("SSN regex is hardcoded and valid"));

/// Pattern matchers for different types of PII
#[derive(Debug)]
pub struct PiiPatterns {
    email: Regex,
    phone: Regex,
    ssn: Regex,
}

impl PiiPatterns {
    /// Create a new set of PII pattern matchers
    pub fn new() -> Self {
        Self {
            email: EMAIL_PATTERN.clone(),
            phone: PHONE_PATTERN.clone(),
            ssn: SSN_PATTERN.clone(),
        }
    }

    /// Check if text contains an email address
    #[must_use]
    pub fn has_email(&self, text: &str) -> bool {
        self.email.is_match(text)
    }

    /// Check if text contains a phone number
    #[must_use]
    pub fn has_phone(&self, text: &str) -> bool {
        self.phone.is_match(text)
    }

    /// Check if text contains an SSN
    #[must_use]
    pub fn has_ssn(&self, text: &str) -> bool {
        self.ssn.is_match(text)
    }

    /// Find all PII matches in text with line numbers
    #[must_use]
    pub fn find_all(&self, text: &str) -> Vec<PiiMatch> {
        let mut matches = Vec::new();

        for (line_num, line) in text.lines().enumerate() {
            // Find emails
            for email_match in self.email.find_iter(line) {
                matches.push(PiiMatch {
                    pii_type: PiiType::Email,
                    matched_value: email_match.as_str().to_string(),
                    line_number: line_num + 1, // nosemgrep: llm-prompt-injection-risk
                });
            }

            // Find phone numbers
            for phone_match in self.phone.find_iter(line) {
                matches.push(PiiMatch {
                    pii_type: PiiType::Phone,
                    matched_value: phone_match.as_str().to_string(),
                    line_number: line_num + 1, // nosemgrep: llm-prompt-injection-risk
                });
            }

            // Find SSNs
            for ssn_match in self.ssn.find_iter(line) {
                matches.push(PiiMatch {
                    pii_type: PiiType::Ssn,
                    matched_value: ssn_match.as_str().to_string(),
                    line_number: line_num + 1, // nosemgrep: llm-prompt-injection-risk
                });
            }
        }

        matches
    }
}

impl Default for PiiPatterns {
    fn default() -> Self {
        Self::new()
    }
}

/// Type of PII found with details
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PiiMatch {
    pub pii_type: PiiType,
    pub matched_value: String,
    pub line_number: usize,
}

/// Type of PII
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PiiType {
    Email,
    Phone,
    Ssn,
}

impl PiiType {
    /// Get human-readable description of the PII type
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            PiiType::Email => "Email address",
            PiiType::Phone => "Phone number",
            PiiType::Ssn => "Social Security Number",
        }
    }

    /// Get risk level for this type of PII
    #[must_use]
    pub fn risk_level(&self) -> &'static str {
        match self {
            PiiType::Email => "medium",
            PiiType::Phone => "medium",
            PiiType::Ssn => "critical",
        }
    }

    /// Get PII type identifier for database storage
    #[must_use]
    pub fn pii_type_str(&self) -> &'static str {
        match self {
            PiiType::Email => "email",
            PiiType::Phone => "phone",
            PiiType::Ssn => "ssn",
        }
    }
}

impl PiiMatch {
    /// Get human-readable description of the PII type
    #[must_use]
    pub fn description(&self) -> &'static str {
        self.pii_type.description()
    }

    /// Get risk level for this type of PII
    #[must_use]
    pub fn risk_level(&self) -> &'static str {
        self.pii_type.risk_level()
    }

    /// Get PII type identifier for database storage
    #[must_use]
    pub fn pii_type_str(&self) -> &'static str {
        self.pii_type.pii_type_str()
    }
}

/// Result of scanning a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileScanResult {
    pub path: PathBuf,
    pub matches: Vec<PiiMatch>,
}

/// Extensions that are safe to scan as text files
const SCANNABLE_EXTENSIONS: &[&str] = &["txt", "csv", "json", "md", "log"];

/// Check if a file should be scanned based on its extension
#[must_use]
pub fn is_scannable(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        if let Some(ext_str) = ext.to_str() {
            return SCANNABLE_EXTENSIONS.contains(&ext_str.to_lowercase().as_str());
        }
    }
    false
}

/// Scan a single file for PII
pub async fn scan_file(path: &Path, patterns: &PiiPatterns) -> Option<FileScanResult> {
    if !is_scannable(path) {
        return None;
    }

    // Check file size before reading
    let metadata = match fs::metadata(path).await {
        Ok(m) => m,
        Err(e) => {
            debug!("Failed to read metadata for {}: {}", path.display(), e);
            return None;
        }
    };

    // Skip files that are too large
    if metadata.len() > MAX_FILE_SIZE {
        debug!(
            "Skipping large file ({}MB): {}",
            metadata.len() / 1024 / 1024,
            path.display()
        );
        return None;
    }

    match fs::read_to_string(path).await {
        Ok(contents) => {
            let matches = patterns.find_all(&contents);
            if matches.is_empty() {
                None
            } else {
                debug!("Found PII in file: {:?}", path);
                Some(FileScanResult {
                    path: path.to_path_buf(),
                    matches,
                })
            }
        }
        Err(e) => {
            warn!("Failed to read file {:?}: {}", path, e);
            None
        }
    }
}

/// Recursively scan a directory for files containing PII
pub async fn scan_directory(dir: &Path, patterns: &PiiPatterns) -> Vec<FileScanResult> {
    scan_directory_impl(dir, patterns, MAX_SCAN_DEPTH).await
}

/// Internal implementation that boxes the future to handle recursion with depth limiting
fn scan_directory_impl<'a>(
    dir: &'a Path,
    patterns: &'a PiiPatterns,
    max_depth: usize,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<FileScanResult>> + Send + 'a>> {
    Box::pin(async move {
        // Check depth limit
        if max_depth == 0 {
            debug!("Max depth reached, skipping: {:?}", dir);
            return Vec::new();
        }

        let mut results = Vec::new();

        let mut entries = match fs::read_dir(dir).await {
            Ok(entries) => entries,
            Err(e) => {
                warn!("Failed to read directory {:?}: {}", dir, e);
                return results;
            }
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();

            // Get metadata to check for symlinks and file type
            let metadata = match entry.metadata().await {
                Ok(m) => m,
                Err(e) => {
                    debug!("Failed to read metadata for {:?}: {}", path, e);
                    continue;
                }
            };

            // Skip symlinks to prevent symlink attacks and infinite loops
            if metadata.is_symlink() {
                debug!("Skipping symlink: {:?}", path);
                continue;
            }

            if metadata.is_dir() {
                // Recursively scan subdirectories with decremented depth
                let mut subdir_results = scan_directory_impl(&path, patterns, max_depth - 1).await;
                results.append(&mut subdir_results);
            } else if metadata.is_file() {
                // Scan individual file
                if let Some(result) = scan_file(&path, patterns).await {
                    results.push(result);
                }
            }
        }

        results
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_pattern() {
        let patterns = PiiPatterns::new();

        assert!(patterns.has_email("contact me at john@example.com"));
        assert!(patterns.has_email("Email: alice.smith@company.co.uk"));
        assert!(!patterns.has_email("not an email"));
        assert!(!patterns.has_email("@invalid"));
    }

    #[test]
    fn test_phone_pattern() {
        let patterns = PiiPatterns::new();

        assert!(patterns.has_phone("Call (555) 123-4567"));
        assert!(patterns.has_phone("Phone: 555-123-4567"));
        assert!(patterns.has_phone("Contact: 555.123.4567"));
        assert!(patterns.has_phone("Number: 5551234567"));
        assert!(!patterns.has_phone("not a phone"));
    }

    #[test]
    fn test_ssn_pattern() {
        let patterns = PiiPatterns::new();

        assert!(patterns.has_ssn("SSN: 123-45-6789"));
        assert!(patterns.has_ssn("Social Security Number 987-65-4321"));
        assert!(!patterns.has_ssn("not an ssn"));
        assert!(!patterns.has_ssn("12345678")); // No dashes
    }

    #[test]
    fn test_find_all() {
        let patterns = PiiPatterns::new();

        let text = "Contact: john@example.com, Phone: 555-123-4567, SSN: 123-45-6789";
        let matches = patterns.find_all(text);

        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].pii_type, PiiType::Email);
        assert_eq!(matches[0].matched_value, "john@example.com");
        assert_eq!(matches[0].line_number, 1);
        assert_eq!(matches[1].pii_type, PiiType::Phone);
        assert_eq!(matches[1].matched_value, "555-123-4567");
        assert_eq!(matches[2].pii_type, PiiType::Ssn);
        assert_eq!(matches[2].matched_value, "123-45-6789");
    }

    #[test]
    fn test_pii_type_description() {
        assert_eq!(PiiType::Email.description(), "Email address");
        assert_eq!(PiiType::Phone.description(), "Phone number");
        assert_eq!(PiiType::Ssn.description(), "Social Security Number");
    }

    #[test]
    fn test_pii_type_risk_level() {
        assert_eq!(PiiType::Email.risk_level(), "medium");
        assert_eq!(PiiType::Phone.risk_level(), "medium");
        assert_eq!(PiiType::Ssn.risk_level(), "critical");
    }

    #[test]
    fn test_is_scannable() {
        assert!(is_scannable(Path::new("document.txt")));
        assert!(is_scannable(Path::new("data.csv")));
        assert!(is_scannable(Path::new("config.json")));
        assert!(is_scannable(Path::new("README.md")));
        assert!(is_scannable(Path::new("app.log")));
        assert!(!is_scannable(Path::new("image.png")));
        assert!(!is_scannable(Path::new("video.mp4")));
        assert!(!is_scannable(Path::new("binary.exe")));
    }
}
