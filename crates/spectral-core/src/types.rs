//! Shared types used across the Spectral application.
//!
//! This module defines common newtypes and enums that provide type safety
//! and clear domain modeling.

use crate::error::SpectralError;
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::OnceLock;

/// Newtype for profile identifiers with validation.
///
/// Profile IDs must be valid UUIDs (v4 format).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProfileId(String);

impl ProfileId {
    /// Create a new `ProfileId` from a string.
    ///
    /// # Errors
    /// Returns error if the ID is not a valid UUID v4.
    pub fn new(id: impl Into<String>) -> Result<Self, SpectralError> {
        let id = id.into();
        Self::validate(&id)?;
        Ok(Self(id))
    }

    /// Create a new random `ProfileId` using UUID v4.
    #[must_use]
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Get the inner string value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validate that a string is a valid UUID v4.
    fn validate(id: &str) -> Result<(), SpectralError> {
        static UUID_REGEX: OnceLock<Regex> = OnceLock::new();
        let regex = UUID_REGEX.get_or_init(|| {
            Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")
                .expect("valid regex")
        });

        if regex.is_match(id) {
            Ok(())
        } else {
            Err(SpectralError::Validation(format!(
                "invalid profile ID: must be a valid UUID v4, got '{id}'"
            )))
        }
    }
}

impl fmt::Display for ProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Newtype for broker identifiers with validation.
///
/// Broker IDs must be lowercase alphanumeric with hyphens, 3-50 characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BrokerId(String);

impl BrokerId {
    /// Create a new `BrokerId` from a string.
    ///
    /// # Errors
    /// Returns error if the ID doesn't match the required format.
    pub fn new(id: impl Into<String>) -> Result<Self, SpectralError> {
        let id = id.into();
        Self::validate(&id)?;
        Ok(Self(id))
    }

    /// Get the inner string value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validate broker ID format: lowercase alphanumeric with hyphens, 3-50 chars.
    fn validate(id: &str) -> Result<(), SpectralError> {
        static BROKER_REGEX: OnceLock<Regex> = OnceLock::new();
        let regex = BROKER_REGEX
            .get_or_init(|| Regex::new(r"^[a-z0-9][a-z0-9-]{1,48}[a-z0-9]$").expect("valid regex"));

        if id.len() < 3 || id.len() > 50 {
            return Err(SpectralError::Validation(format!(
                "invalid broker ID: must be 3-50 characters, got {} characters",
                id.len()
            )));
        }

        if regex.is_match(id) {
            Ok(())
        } else {
            Err(SpectralError::Validation(format!(
                "invalid broker ID: must be lowercase alphanumeric with hyphens, got '{id}'"
            )))
        }
    }
}

impl fmt::Display for BrokerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Categories of personally identifiable information.
///
/// Used for tracking what types of PII are stored in profiles and
/// what was found on data broker sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PiiField {
    /// Full legal name
    FullName,
    /// First name
    FirstName,
    /// Middle name
    MiddleName,
    /// Last name
    LastName,
    /// Email address
    Email,
    /// Phone number
    Phone,
    /// Street address
    Address,
    /// City
    City,
    /// State/province
    State,
    /// ZIP/postal code
    ZipCode,
    /// Country
    Country,
    /// Date of birth
    DateOfBirth,
    /// Age
    Age,
    /// Social Security Number
    Ssn,
    /// Employer/company
    Employer,
    /// Job title
    JobTitle,
    /// Educational institution
    Education,
    /// Social media username
    SocialMedia,
    /// IP address
    IpAddress,
    /// Photo/image
    Photo,
    /// Relatives/associates
    Relatives,
    /// Previous addresses
    PreviousAddress,
    /// Other/custom field
    Other,
}

impl PiiField {
    /// Get a human-readable display name for the PII field.
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::FullName => "Full Name",
            Self::FirstName => "First Name",
            Self::MiddleName => "Middle Name",
            Self::LastName => "Last Name",
            Self::Email => "Email Address",
            Self::Phone => "Phone Number",
            Self::Address => "Street Address",
            Self::City => "City",
            Self::State => "State/Province",
            Self::ZipCode => "ZIP/Postal Code",
            Self::Country => "Country",
            Self::DateOfBirth => "Date of Birth",
            Self::Age => "Age",
            Self::Ssn => "Social Security Number",
            Self::Employer => "Employer",
            Self::JobTitle => "Job Title",
            Self::Education => "Education",
            Self::SocialMedia => "Social Media",
            Self::IpAddress => "IP Address",
            Self::Photo => "Photo",
            Self::Relatives => "Relatives",
            Self::PreviousAddress => "Previous Address",
            Self::Other => "Other",
        }
    }

    /// Get the sensitivity level (0-3, higher is more sensitive).
    #[must_use]
    pub fn sensitivity_level(&self) -> u8 {
        match self {
            Self::Ssn | Self::DateOfBirth => 3, // Highly sensitive
            Self::FullName | Self::Email | Self::Phone | Self::Address | Self::Photo => 2, // Sensitive
            Self::FirstName | Self::LastName | Self::City | Self::State | Self::Employer => 1, // Moderate
            _ => 0, // Low sensitivity
        }
    }
}

impl fmt::Display for PiiField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Wrapper around `chrono::DateTime<Utc>` for consistent timestamp handling.
///
/// Provides serialization/deserialization and utility methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    /// Create a timestamp representing the current moment.
    #[must_use]
    pub fn now() -> Self {
        Self(Utc::now())
    }

    /// Create a timestamp from a `DateTime<Utc>`.
    #[must_use]
    pub fn from_datetime(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }

    /// Get the inner `DateTime<Utc>`.
    #[must_use]
    pub fn as_datetime(&self) -> &DateTime<Utc> {
        &self.0
    }

    /// Parse a timestamp from an RFC3339 string.
    pub fn from_rfc3339(s: &str) -> Result<Self, SpectralError> {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| Self(dt.with_timezone(&Utc)))
            .map_err(|e| SpectralError::Validation(format!("invalid timestamp: {e}")))
    }

    /// Format as RFC3339 string.
    #[must_use]
    pub fn to_rfc3339(&self) -> String {
        self.0.to_rfc3339()
    }

    /// Get seconds since Unix epoch.
    #[must_use]
    pub fn timestamp(&self) -> i64 {
        self.0.timestamp()
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::now()
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_rfc3339())
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }
}

impl From<Timestamp> for DateTime<Utc> {
    fn from(ts: Timestamp) -> Self {
        ts.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_id_valid() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        let profile_id = ProfileId::new(id).expect("valid profile ID");
        assert_eq!(profile_id.as_str(), id);
    }

    #[test]
    fn test_profile_id_invalid() {
        let invalid_ids = vec![
            "not-a-uuid",
            "550e8400-e29b-51d4-a716-446655440000", // Wrong version
            "550e8400-e29b-41d4-x716-446655440000", // Invalid hex
            "",
        ];

        for id in invalid_ids {
            assert!(ProfileId::new(id).is_err());
        }
    }

    #[test]
    fn test_profile_id_generate() {
        let id1 = ProfileId::generate();
        let id2 = ProfileId::generate();
        assert_ne!(id1, id2); // Should be unique
    }

    #[test]
    fn test_broker_id_valid() {
        let valid_ids = vec![
            "spokeo",
            "been-verified",
            "fast-people-search",
            "whitepages-premium",
            "abc",
        ];

        for id in valid_ids {
            assert!(BrokerId::new(id).is_ok(), "Failed for: {id}");
        }
    }

    #[test]
    fn test_broker_id_invalid() {
        let too_long = "a".repeat(51);
        let invalid_ids = vec![
            "AB",              // Too short
            "Spokeo",          // Uppercase
            "been_verified",   // Underscore
            "fast people",     // Space
            "-spokeo",         // Starts with hyphen
            "spokeo-",         // Ends with hyphen
            too_long.as_str(), // Too long
        ];

        for id in invalid_ids {
            assert!(BrokerId::new(id).is_err(), "Should fail for: {id}");
        }
    }

    #[test]
    fn test_pii_field_display() {
        assert_eq!(PiiField::FullName.to_string(), "Full Name");
        assert_eq!(PiiField::Email.to_string(), "Email Address");
    }

    #[test]
    fn test_pii_field_sensitivity() {
        assert_eq!(PiiField::Ssn.sensitivity_level(), 3);
        assert_eq!(PiiField::Email.sensitivity_level(), 2);
        assert_eq!(PiiField::City.sensitivity_level(), 1);
    }

    #[test]
    fn test_timestamp_now() {
        let ts = Timestamp::now();
        assert!(ts.timestamp() > 0);
    }

    #[test]
    fn test_timestamp_rfc3339() {
        let ts = Timestamp::now();
        let s = ts.to_rfc3339();
        let parsed = Timestamp::from_rfc3339(&s).expect("parse RFC3339 timestamp");
        // Compare timestamps (not exact equality due to precision)
        assert_eq!(ts.timestamp(), parsed.timestamp());
    }

    #[test]
    fn test_timestamp_ordering() {
        let ts1 = Timestamp::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let ts2 = Timestamp::now();
        assert!(ts2 > ts1);
    }

    #[test]
    fn test_pii_field_serialization() {
        let field = PiiField::Email;
        let json = serde_json::to_string(&field).expect("serialize PII field");
        assert_eq!(json, "\"email\"");

        let deserialized: PiiField = serde_json::from_str(&json).expect("deserialize PII field");
        assert_eq!(deserialized, field);
    }
}
