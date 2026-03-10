//! Filesystem PII discovery scanner
//!
//! Scans local files for personally identifiable information (PII)
//! by searching for the user's specific email addresses, phone numbers, and SSN.
//!
//! This scanner is designed to find the user's OWN information in files,
//! not generic PII from random people.

use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, warn};

/// Maximum file size to scan (100MB)
const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

/// Maximum directory depth to scan
const MAX_SCAN_DEPTH: usize = 10;

/// User-specific PII to search for
#[derive(Debug, Clone)]
pub struct UserPii {
    /// User's email addresses
    pub emails: Vec<String>,
    /// User's phone numbers (normalized to digits only)
    pub phones: Vec<String>,
    /// User's Social Security Number (if provided)
    pub ssn: Option<String>,
}

impl UserPii {
    /// Create a new UserPii with no data
    pub fn empty() -> Self {
        Self {
            emails: Vec::new(),
            phones: Vec::new(),
            ssn: None,
        }
    }
}

/// Pattern matchers for user-specific PII
#[derive(Debug)]
pub struct PiiPatterns {
    /// User's email addresses (lowercase)
    user_emails: Vec<String>,
    /// User's phone numbers (normalized to digits only, e.g., "5551234567")
    user_phones: Vec<String>,
    /// User's SSN (if provided)
    user_ssn: Option<String>,
}

impl PiiPatterns {
    /// Create PII patterns from user-specific information
    pub fn from_user_pii(user_pii: UserPii) -> Self {
        Self {
            user_emails: user_pii
                .emails
                .into_iter()
                .map(|e| e.to_lowercase())
                .collect(),
            user_phones: user_pii
                .phones
                .into_iter()
                .map(|p| normalize_phone(&p))
                .collect(),
            user_ssn: user_pii.ssn,
        }
    }

    /// Check if text contains user's email address (case-insensitive)
    #[must_use]
    pub fn has_email(&self, text: &str) -> bool {
        if self.user_emails.is_empty() {
            return false;
        }
        let text_lower = text.to_lowercase();
        self.user_emails
            .iter()
            .any(|email| text_lower.contains(email))
    }

    /// Check if text contains user's phone number (any format)
    #[must_use]
    pub fn has_phone(&self, text: &str) -> bool {
        if self.user_phones.is_empty() {
            return false;
        }
        let normalized_text = normalize_phone(text);
        self.user_phones
            .iter()
            .any(|phone| normalized_text.contains(phone))
    }

    /// Check if text contains user's SSN
    #[must_use]
    pub fn has_ssn(&self, text: &str) -> bool {
        if let Some(ssn) = &self.user_ssn {
            text.contains(ssn)
        } else {
            false
        }
    }

    /// Find all PII matches in text with line numbers
    #[must_use]
    pub fn find_all(&self, text: &str) -> Vec<PiiMatch> {
        let mut matches = Vec::new();

        for (line_num, line) in text.lines().enumerate() {
            let line_lower = line.to_lowercase();
            let line_normalized = normalize_phone(line);

            // Find user's email addresses
            for email in &self.user_emails {
                if line_lower.contains(email) {
                    // Find all occurrences of this email in the line
                    let regex = RegexBuilder::new(&regex::escape(email))
                        .case_insensitive(true)
                        .build()
                        .expect("regex::escape produces valid regex patterns");

                    for email_match in regex.find_iter(line) {
                        matches.push(PiiMatch {
                            pii_type: PiiType::Email,
                            matched_value: email_match.as_str().to_string(),
                            line_number: line_num + 1, // nosemgrep: llm-prompt-injection-risk
                        });
                    }
                }
            }

            // Find user's phone numbers
            for phone in &self.user_phones {
                if line_normalized.contains(phone) {
                    // Try to find the phone number in various formats in the original line
                    if let Some(matched_phone) = find_phone_in_line(line, phone) {
                        matches.push(PiiMatch {
                            pii_type: PiiType::Phone,
                            matched_value: matched_phone,
                            line_number: line_num + 1, // nosemgrep: llm-prompt-injection-risk
                        });
                    }
                }
            }

            // Find user's SSN
            if let Some(ssn) = &self.user_ssn {
                if line.contains(ssn) {
                    matches.push(PiiMatch {
                        pii_type: PiiType::Ssn,
                        matched_value: ssn.clone(),
                        line_number: line_num + 1, // nosemgrep: llm-prompt-injection-risk
                    });
                }
            }
        }

        matches
    }
}

/// Normalize phone number to digits only
fn normalize_phone(text: &str) -> String {
    text.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// Find a phone number in a line given its normalized form
fn find_phone_in_line(line: &str, normalized_phone: &str) -> Option<String> {
    // Common phone number patterns to try matching
    let patterns = [
        // (555) 123-4567
        format!(
            r"\({}\)\s*{}-{}",
            &normalized_phone[0..3],
            &normalized_phone[3..6],
            &normalized_phone[6..10]
        ),
        // 555-123-4567
        format!(
            r"{}-{}-{}",
            &normalized_phone[0..3],
            &normalized_phone[3..6],
            &normalized_phone[6..10]
        ),
        // 555.123.4567
        format!(
            r"{}\.{}\.{}",
            &normalized_phone[0..3],
            &normalized_phone[3..6],
            &normalized_phone[6..10]
        ),
        // 5551234567 (no formatting)
        normalized_phone.to_string(),
    ];

    for pattern in &patterns {
        if let Ok(regex) = regex::Regex::new(pattern) {
            if let Some(m) = regex.find(line) {
                return Some(m.as_str().to_string());
            }
        }
    }

    None
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
    fn test_user_email_detection() {
        let user_pii = UserPii {
            emails: vec!["john@example.com".to_string()],
            phones: vec![],
            ssn: None,
        };
        let patterns = PiiPatterns::from_user_pii(user_pii);

        // Should find user's email
        assert!(patterns.has_email("contact me at john@example.com"));
        assert!(patterns.has_email("Email: JOHN@EXAMPLE.COM")); // Case insensitive

        // Should NOT find other emails
        assert!(!patterns.has_email("Email: alice@company.com"));
        assert!(!patterns.has_email("not an email"));
    }

    #[test]
    fn test_user_phone_detection() {
        let user_pii = UserPii {
            emails: vec![],
            phones: vec!["5551234567".to_string()],
            ssn: None,
        };
        let patterns = PiiPatterns::from_user_pii(user_pii);

        // Should find user's phone in various formats
        assert!(patterns.has_phone("Call (555) 123-4567"));
        assert!(patterns.has_phone("Phone: 555-123-4567"));
        assert!(patterns.has_phone("Contact: 555.123.4567"));
        assert!(patterns.has_phone("Number: 5551234567"));

        // Should NOT find other phone numbers
        assert!(!patterns.has_phone("Call (999) 888-7777"));
    }

    #[test]
    fn test_user_ssn_detection() {
        let user_pii = UserPii {
            emails: vec![],
            phones: vec![],
            ssn: Some("123-45-6789".to_string()),
        };
        let patterns = PiiPatterns::from_user_pii(user_pii);

        // Should find user's SSN
        assert!(patterns.has_ssn("SSN: 123-45-6789"));
        assert!(patterns.has_ssn("Social Security Number 123-45-6789"));

        // Should NOT find other SSNs
        assert!(!patterns.has_ssn("SSN: 987-65-4321"));
        assert!(!patterns.has_ssn("not an ssn"));
    }

    #[test]
    fn test_find_all_user_specific() {
        let user_pii = UserPii {
            emails: vec!["john@example.com".to_string()],
            phones: vec!["5551234567".to_string()],
            ssn: Some("123-45-6789".to_string()),
        };
        let patterns = PiiPatterns::from_user_pii(user_pii);

        let text = "Contact: john@example.com, Phone: 555-123-4567, SSN: 123-45-6789\n\
                    Other person: alice@company.com, Phone: (999) 888-7777";
        let matches = patterns.find_all(text);

        // Should only find user's PII, not the other person's
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
