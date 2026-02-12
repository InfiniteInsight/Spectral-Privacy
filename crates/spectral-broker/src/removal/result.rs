//! Removal result types and outcomes.

use serde::{Deserialize, Serialize};

/// Outcome of a removal attempt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RemovalOutcome {
    /// Form submitted successfully
    Submitted,

    /// Requires email verification to complete
    RequiresEmailVerification {
        /// The email address that needs verification
        email: String,
        /// The address where verification was sent
        sent_to: String,
    },

    /// CAPTCHA detected, requires user intervention
    RequiresCaptcha {
        /// URL to the CAPTCHA challenge
        captcha_url: String,
    },

    /// Broker requires account creation first
    RequiresAccountCreation,

    /// Submission failed with reason
    Failed {
        /// Human-readable failure reason
        reason: String,
        /// Optional technical error details
        error_details: Option<String>,
    },
}

impl RemovalOutcome {
    /// Check if the outcome requires user action
    #[must_use]
    pub fn requires_user_action(&self) -> bool {
        matches!(
            self,
            Self::RequiresEmailVerification { .. }
                | Self::RequiresCaptcha { .. }
                | Self::RequiresAccountCreation
        )
    }

    /// Check if the outcome is a failure
    #[must_use]
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Check if the outcome is successful
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Submitted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_requires_user_action() {
        let outcome = RemovalOutcome::RequiresCaptcha {
            captcha_url: "https://example.com".to_string(),
        };
        assert!(outcome.requires_user_action());

        let outcome = RemovalOutcome::Submitted;
        assert!(!outcome.requires_user_action());
    }

    #[test]
    fn test_is_failure() {
        let outcome = RemovalOutcome::Failed {
            reason: "Timeout".to_string(),
            error_details: None,
        };
        assert!(outcome.is_failure());

        let outcome = RemovalOutcome::Submitted;
        assert!(!outcome.is_failure());
    }

    #[test]
    fn test_is_success() {
        let outcome = RemovalOutcome::Submitted;
        assert!(outcome.is_success());

        let outcome = RemovalOutcome::Failed {
            reason: "Error".to_string(),
            error_details: None,
        };
        assert!(!outcome.is_success());
    }
}
