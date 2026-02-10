use chrono::{Datelike, NaiveDate};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use spectral_core::error::SpectralError;

/// Input type for creating/updating a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInput {
    pub first_name: String,
    pub middle_name: Option<String>,
    pub last_name: String,
    pub email: String,
    pub date_of_birth: Option<NaiveDate>,
    pub address_line1: String,
    pub address_line2: Option<String>,
    pub city: String,
    pub state: String, // US state code (e.g., "CA")
    pub zip_code: String,
}

impl ProfileInput {
    /// Validate all fields
    pub fn validate(&self) -> Result<(), SpectralError> {
        validate_name(&self.first_name)?;
        if let Some(ref middle) = self.middle_name {
            if !middle.is_empty() {
                validate_name(middle)?;
            }
        }
        validate_name(&self.last_name)?;
        validate_email(&self.email)?;
        if let Some(dob) = self.date_of_birth {
            validate_date_of_birth(dob)?;
        }
        validate_us_state(&self.state)?;
        validate_zip_code(&self.zip_code)?;
        Ok(())
    }
}

/// Output type for profile data (returned to frontend)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileOutput {
    pub id: String,
    pub first_name: String,
    pub middle_name: Option<String>,
    pub last_name: String,
    pub email: String,
    pub date_of_birth: Option<NaiveDate>,
    pub address_line1: String,
    pub address_line2: Option<String>,
    pub city: String,
    pub state: String,
    pub zip_code: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Summary type for profile listings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSummary {
    pub id: String,
    pub full_name: String,
    pub email: String,
    pub created_at: String,
}

// Validation functions

// Compile regexes once using Lazy for performance
static EMAIL_REGEX: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
        .expect("Email regex should be valid")
});

static ZIP_CODE_REGEX: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"^\d{5}(-\d{4})?$").expect("ZIP code regex should be valid"));

const US_STATES: &[&str] = &[
    "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID", "IL", "IN", "IA", "KS",
    "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ", "NM", "NY",
    "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT", "VT", "VA", "WA", "WV",
    "WI", "WY",
];

pub fn validate_name(name: &str) -> Result<(), SpectralError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(SpectralError::Validation(
            "Name cannot be empty".to_string(),
        ));
    }

    // Allow letters, spaces, hyphens, and apostrophes
    let valid = trimmed
        .chars()
        .all(|c| c.is_alphabetic() || c.is_whitespace() || c == '-' || c == '\'');

    if !valid {
        return Err(SpectralError::Validation(
            "Name can only contain letters, spaces, hyphens, and apostrophes".to_string(),
        ));
    }

    Ok(())
}

pub fn validate_email(email: &str) -> Result<(), SpectralError> {
    if !EMAIL_REGEX.is_match(email) {
        return Err(SpectralError::Validation(
            "Invalid email format".to_string(),
        ));
    }

    Ok(())
}

pub fn validate_date_of_birth(dob: NaiveDate) -> Result<(), SpectralError> {
    let today = chrono::Local::now().date_naive();

    // Calculate age properly accounting for leap years
    let mut age = today.year() - dob.year();

    // Subtract 1 if birthday hasn't occurred yet this year
    if today.month() < dob.month() || (today.month() == dob.month() && today.day() < dob.day()) {
        age -= 1;
    }

    if age < 13 {
        return Err(SpectralError::Validation(
            "Must be at least 13 years old".to_string(),
        ));
    }
    if age > 120 {
        return Err(SpectralError::Validation(
            "Invalid date of birth".to_string(),
        ));
    }

    Ok(())
}

pub fn validate_us_state(state: &str) -> Result<(), SpectralError> {
    if !US_STATES.contains(&state) {
        return Err(SpectralError::Validation(
            "Invalid US state code".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_zip_code(zip: &str) -> Result<(), SpectralError> {
    if !ZIP_CODE_REGEX.is_match(zip) {
        return Err(SpectralError::Validation(
            "Invalid ZIP code format (use 12345 or 12345-6789)".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name() {
        assert!(validate_name("John").is_ok());
        assert!(validate_name("Mary-Jane").is_ok());
        assert!(validate_name("O'Brien").is_ok());
        assert!(validate_name("").is_err());
        assert!(validate_name("   ").is_err());
        assert!(validate_name("123").is_err());
        assert!(validate_name("John123").is_err());
    }

    #[test]
    fn test_validate_email() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("user.name+tag@example.co.uk").is_ok());
        assert!(validate_email("invalid").is_err());
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("user@").is_err());
    }

    #[test]
    fn test_validate_date_of_birth() {
        let today = chrono::Local::now().date_naive();
        let valid_date = today - chrono::Duration::days(365 * 20); // 20 years ago
        let too_young = today - chrono::Duration::days(365 * 10); // 10 years old
        let too_old = today - chrono::Duration::days(365 * 150); // 150 years old

        assert!(validate_date_of_birth(valid_date).is_ok());
        assert!(validate_date_of_birth(too_young).is_err());
        assert!(validate_date_of_birth(too_old).is_err());
    }

    #[test]
    fn test_validate_us_state() {
        assert!(validate_us_state("CA").is_ok());
        assert!(validate_us_state("NY").is_ok());
        assert!(validate_us_state("XX").is_err());
        assert!(validate_us_state("California").is_err());
    }

    #[test]
    fn test_validate_zip_code() {
        assert!(validate_zip_code("12345").is_ok());
        assert!(validate_zip_code("12345-6789").is_ok());
        assert!(validate_zip_code("1234").is_err());
        assert!(validate_zip_code("123456").is_err());
        assert!(validate_zip_code("abcde").is_err());
    }

    #[test]
    fn test_profile_input_validation() {
        let valid_input = ProfileInput {
            first_name: "John".to_string(),
            middle_name: Some("A".to_string()),
            last_name: "Doe".to_string(),
            email: "john@example.com".to_string(),
            date_of_birth: Some(
                chrono::Local::now().date_naive() - chrono::Duration::days(365 * 30),
            ),
            address_line1: "123 Main St".to_string(),
            address_line2: Some("Apt 4B".to_string()),
            city: "San Francisco".to_string(),
            state: "CA".to_string(),
            zip_code: "94102".to_string(),
        };
        assert!(valid_input.validate().is_ok());

        let invalid_email = ProfileInput {
            email: "invalid".to_string(),
            ..valid_input.clone()
        };
        assert!(invalid_email.validate().is_err());
    }
}
