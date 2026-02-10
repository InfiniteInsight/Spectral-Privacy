//! PII detection and filtering for LLM requests.

use crate::error::{LlmError, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// PII filter that detects and sanitizes personally identifiable information.
///
/// The filter uses regex patterns to detect common PII types and applies
/// a configurable strategy (redact, tokenize, or block).
#[derive(Debug, Clone)]
pub struct PiiFilter {
    patterns: Vec<PiiPattern>,
    strategy: FilterStrategy,
}

impl Default for PiiFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl PiiFilter {
    /// Create a new PII filter with default patterns and redaction strategy.
    #[must_use]
    pub fn new() -> Self {
        Self {
            patterns: default_patterns(),
            strategy: FilterStrategy::Redact,
        }
    }

    /// Create a PII filter with a specific strategy.
    #[must_use]
    pub fn with_strategy(strategy: FilterStrategy) -> Self {
        Self {
            patterns: default_patterns(),
            strategy,
        }
    }

    /// Scan text for PII and apply the configured filter strategy.
    ///
    /// # Errors
    /// Returns error if PII is detected and the strategy is `Block`.
    pub fn filter(&self, text: &str) -> Result<FilterResult> {
        let mut detections = Vec::new();

        // Scan for PII using all patterns
        for pattern in &self.patterns {
            if let Some(captures) = pattern.regex.captures(text) {
                if let Some(matched) = captures.get(0) {
                    detections.push(PiiDetection {
                        pii_type: pattern.pii_type,
                        start: matched.start(),
                        end: matched.end(),
                        value: matched.as_str().to_string(),
                    });
                }
            }
        }

        // If no PII detected, return original text
        if detections.is_empty() {
            return Ok(FilterResult {
                filtered_text: text.to_string(),
                detections,
                token_map: None,
            });
        }

        // Apply strategy
        match &self.strategy {
            FilterStrategy::Block => Err(LlmError::PiiBlocked {
                details: format!(
                    "detected {} PII fields: {}",
                    detections.len(),
                    detections
                        .iter()
                        .map(|d| d.pii_type.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            }),
            FilterStrategy::Redact => {
                let filtered = Self::apply_redaction(text, &detections);
                Ok(FilterResult {
                    filtered_text: filtered,
                    detections,
                    token_map: None,
                })
            }
            FilterStrategy::Tokenize => {
                let (filtered, token_map) = Self::apply_tokenization(text, &detections);
                Ok(FilterResult {
                    filtered_text: filtered,
                    detections,
                    token_map: Some(token_map),
                })
            }
        }
    }

    /// Apply redaction to text by replacing PII with placeholder strings.
    fn apply_redaction(text: &str, detections: &[PiiDetection]) -> String {
        let mut result = text.to_string();
        // Apply replacements in reverse order to maintain indices
        let mut sorted_detections = detections.to_vec();
        sorted_detections.sort_by_key(|d| std::cmp::Reverse(d.start));

        for detection in sorted_detections {
            let placeholder = format!("[REDACTED_{}]", detection.pii_type.as_str());
            result.replace_range(detection.start..detection.end, &placeholder);
        }

        result
    }

    /// Apply tokenization by replacing PII with reversible tokens.
    fn apply_tokenization(
        text: &str,
        detections: &[PiiDetection],
    ) -> (String, HashMap<String, String>) {
        let mut result = text.to_string();
        let mut token_map = HashMap::new();

        // Apply replacements in reverse order to maintain indices
        let mut sorted_detections = detections.to_vec();
        sorted_detections.sort_by_key(|d| std::cmp::Reverse(d.start));

        for (idx, detection) in sorted_detections.iter().enumerate() {
            let token = format!("__PII_TOKEN_{idx}__");
            token_map.insert(token.clone(), detection.value.clone());
            result.replace_range(detection.start..detection.end, &token);
        }

        (result, token_map)
    }

    /// Detokenize text by replacing tokens with original PII values.
    #[must_use]
    pub fn detokenize(&self, text: &str, token_map: &HashMap<String, String>) -> String {
        let mut result = text.to_string();
        for (token, original) in token_map {
            result = result.replace(token, original);
        }
        result
    }
}

/// Result of PII filtering.
#[derive(Debug, Clone)]
pub struct FilterResult {
    /// The filtered text (with PII removed/replaced)
    pub filtered_text: String,

    /// List of PII detections found
    pub detections: Vec<PiiDetection>,

    /// Token map for detokenization (if using Tokenize strategy)
    pub token_map: Option<HashMap<String, String>>,
}

impl FilterResult {
    /// Check if any PII was detected.
    #[must_use]
    pub fn has_pii(&self) -> bool {
        !self.detections.is_empty()
    }

    /// Get the count of detected PII fields.
    #[must_use]
    pub fn pii_count(&self) -> usize {
        self.detections.len()
    }
}

/// A detected instance of PII.
#[derive(Debug, Clone)]
pub struct PiiDetection {
    /// Type of PII detected
    pub pii_type: PiiType,

    /// Start position in original text
    pub start: usize,

    /// End position in original text
    pub end: usize,

    /// The detected value
    pub value: String,
}

/// Strategy for handling detected PII.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterStrategy {
    /// Replace PII with `[REDACTED_TYPE]` placeholders
    Redact,

    /// Replace PII with reversible tokens for re-injection
    Tokenize,

    /// Refuse to send if PII is detected
    Block,
}

/// Types of PII that can be detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PiiType {
    /// Email address
    Email,
    /// Phone number
    Phone,
    /// Social Security Number
    Ssn,
    /// Credit card number
    CreditCard,
    /// Street address
    Address,
    /// IP address
    IpAddress,
}

impl PiiType {
    /// Get the string representation of the PII type.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Email => "EMAIL",
            Self::Phone => "PHONE",
            Self::Ssn => "SSN",
            Self::CreditCard => "CREDIT_CARD",
            Self::Address => "ADDRESS",
            Self::IpAddress => "IP_ADDRESS",
        }
    }
}

/// A PII detection pattern.
#[derive(Clone)]
struct PiiPattern {
    pii_type: PiiType,
    regex: Regex,
}

impl std::fmt::Debug for PiiPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PiiPattern")
            .field("pii_type", &self.pii_type)
            .field("regex", &self.regex.as_str())
            .finish()
    }
}

/// Default PII detection patterns.
fn default_patterns() -> Vec<PiiPattern> {
    vec![
        // Email pattern
        PiiPattern {
            pii_type: PiiType::Email,
            regex: EMAIL_REGEX.clone(),
        },
        // Phone pattern (US format)
        PiiPattern {
            pii_type: PiiType::Phone,
            regex: PHONE_REGEX.clone(),
        },
        // SSN pattern (XXX-XX-XXXX)
        PiiPattern {
            pii_type: PiiType::Ssn,
            regex: SSN_REGEX.clone(),
        },
        // Credit card pattern (simple check)
        PiiPattern {
            pii_type: PiiType::CreditCard,
            regex: CREDIT_CARD_REGEX.clone(),
        },
        // IPv4 address
        PiiPattern {
            pii_type: PiiType::IpAddress,
            regex: IPV4_REGEX.clone(),
        },
    ]
}

// Compiled regex patterns
static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").expect("valid email regex")
});

static PHONE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:\+?1[-.\s]?)?\(?([0-9]{3})\)?[-.\s]?([0-9]{3})[-.\s]?([0-9]{4})\b")
        .expect("valid phone regex")
});

static SSN_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").expect("valid SSN regex"));

static CREDIT_CARD_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(?:\d{4}[-\s]?){3}\d{4}\b").expect("valid credit card regex"));

static IPV4_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").expect("valid IPv4 regex"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_detection() {
        let filter = PiiFilter::new();
        let result = filter
            .filter("Contact me at john.doe@example.com")
            .expect("filter text");

        assert!(result.has_pii());
        assert_eq!(result.pii_count(), 1);
        assert_eq!(result.detections[0].pii_type, PiiType::Email);
    }

    #[test]
    fn test_phone_detection() {
        let filter = PiiFilter::new();
        let result = filter
            .filter("Call me at (555) 123-4567")
            .expect("filter text");

        assert!(result.has_pii());
        assert_eq!(result.pii_count(), 1);
        assert_eq!(result.detections[0].pii_type, PiiType::Phone);
    }

    #[test]
    fn test_ssn_detection() {
        let filter = PiiFilter::new();
        let result = filter.filter("SSN: 123-45-6789").expect("filter text");

        assert!(result.has_pii());
        assert_eq!(result.pii_count(), 1);
        assert_eq!(result.detections[0].pii_type, PiiType::Ssn);
    }

    #[test]
    fn test_redaction_strategy() {
        let filter = PiiFilter::with_strategy(FilterStrategy::Redact);
        let result = filter
            .filter("Email: test@example.com")
            .expect("filter text");

        assert!(result.filtered_text.contains("[REDACTED_EMAIL]"));
        assert!(!result.filtered_text.contains("test@example.com"));
    }

    #[test]
    fn test_tokenization_strategy() {
        let filter = PiiFilter::with_strategy(FilterStrategy::Tokenize);
        let result = filter
            .filter("Email: test@example.com")
            .expect("filter text");

        assert!(result.filtered_text.contains("__PII_TOKEN_"));
        assert!(result.token_map.is_some());

        let token_map = result.token_map.expect("token map exists");
        let detokenized = filter.detokenize(&result.filtered_text, &token_map);
        assert_eq!(detokenized, "Email: test@example.com");
    }

    #[test]
    fn test_block_strategy() {
        let filter = PiiFilter::with_strategy(FilterStrategy::Block);
        let result = filter.filter("Email: test@example.com");

        assert!(result.is_err());
        match result {
            Err(LlmError::PiiBlocked { details }) => {
                assert!(details.contains("EMAIL"));
            }
            _ => panic!("expected PiiBlocked error"),
        }
    }

    #[test]
    fn test_no_pii() {
        let filter = PiiFilter::new();
        let result = filter
            .filter("This is a normal message with no PII")
            .expect("filter text");

        assert!(!result.has_pii());
        assert_eq!(result.pii_count(), 0);
        assert_eq!(result.filtered_text, "This is a normal message with no PII");
    }

    #[test]
    fn test_multiple_pii_types() {
        let filter = PiiFilter::new();
        let result = filter
            .filter("Contact: john@example.com or call (555) 123-4567")
            .expect("filter text");

        assert!(result.has_pii());
        assert_eq!(result.pii_count(), 2);

        let types: Vec<PiiType> = result.detections.iter().map(|d| d.pii_type).collect();
        assert!(types.contains(&PiiType::Email));
        assert!(types.contains(&PiiType::Phone));
    }

    #[test]
    fn test_ipv4_detection() {
        let filter = PiiFilter::new();
        let result = filter.filter("Server at 192.168.1.1").expect("filter text");

        assert!(result.has_pii());
        assert_eq!(result.pii_count(), 1);
        assert_eq!(result.detections[0].pii_type, PiiType::IpAddress);
    }
}
