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
    pub fn has_email(&self, text: &str) -> bool {
        self.email.is_match(text)
    }

    /// Check if text contains a phone number
    pub fn has_phone(&self, text: &str) -> bool {
        self.phone.is_match(text)
    }

    /// Check if text contains an SSN
    pub fn has_ssn(&self, text: &str) -> bool {
        self.ssn.is_match(text)
    }

    /// Find all PII matches in text
    pub fn find_all(&self, text: &str) -> Vec<PiiMatch> {
        let mut matches = Vec::new();

        if self.has_email(text) {
            matches.push(PiiMatch::Email);
        }
        if self.has_phone(text) {
            matches.push(PiiMatch::Phone);
        }
        if self.has_ssn(text) {
            matches.push(PiiMatch::Ssn);
        }

        matches
    }
}

impl Default for PiiPatterns {
    fn default() -> Self {
        Self::new()
    }
}

/// Type of PII found
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PiiMatch {
    Email,
    Phone,
    Ssn,
}

impl PiiMatch {
    /// Get human-readable description of the PII type
    pub fn description(&self) -> &'static str {
        match self {
            PiiMatch::Email => "Email address",
            PiiMatch::Phone => "Phone number",
            PiiMatch::Ssn => "Social Security Number",
        }
    }

    /// Get risk level for this type of PII
    pub fn risk_level(&self) -> &'static str {
        match self {
            PiiMatch::Email => "medium",
            PiiMatch::Phone => "medium",
            PiiMatch::Ssn => "critical",
        }
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
        assert!(matches.contains(&PiiMatch::Email));
        assert!(matches.contains(&PiiMatch::Phone));
        assert!(matches.contains(&PiiMatch::Ssn));
    }

    #[test]
    fn test_pii_match_description() {
        assert_eq!(PiiMatch::Email.description(), "Email address");
        assert_eq!(PiiMatch::Phone.description(), "Phone number");
        assert_eq!(PiiMatch::Ssn.description(), "Social Security Number");
    }

    #[test]
    fn test_pii_match_risk_level() {
        assert_eq!(PiiMatch::Email.risk_level(), "medium");
        assert_eq!(PiiMatch::Phone.risk_level(), "medium");
        assert_eq!(PiiMatch::Ssn.risk_level(), "critical");
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
