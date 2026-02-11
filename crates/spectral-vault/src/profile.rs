//! User profile with encrypted PII fields.
//!
//! Manages user profile data with field-level encryption. All PII is stored
//! as encrypted fields that can only be decrypted when the vault is unlocked.

use crate::cipher::EncryptedField;
use crate::error::{Result, VaultError};
use serde::{Deserialize, Serialize};
use spectral_core::types::{ProfileId, Timestamp};
use spectral_db::Database;

/// User profile with encrypted PII fields.
///
/// All personally identifiable information is stored as `EncryptedField<T>`,
/// ensuring that PII is encrypted at rest in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    /// Unique profile identifier
    pub id: ProfileId,
    /// Full name
    pub full_name: Option<EncryptedField<String>>,
    /// First name
    pub first_name: Option<EncryptedField<String>>,
    /// Middle name
    pub middle_name: Option<EncryptedField<String>>,
    /// Last name
    pub last_name: Option<EncryptedField<String>>,
    /// Email address
    pub email: Option<EncryptedField<String>>,
    /// Phone number
    #[deprecated(note = "Use phone_numbers instead")]
    pub phone: Option<EncryptedField<String>>,
    /// Street address
    pub address: Option<EncryptedField<String>>,
    /// City
    pub city: Option<EncryptedField<String>>,
    /// State/province
    pub state: Option<EncryptedField<String>>,
    /// ZIP/postal code
    pub zip_code: Option<EncryptedField<String>>,
    /// Country
    pub country: Option<EncryptedField<String>>,
    /// Date of birth (ISO 8601 format)
    pub date_of_birth: Option<EncryptedField<String>>,
    /// Social Security Number
    pub ssn: Option<EncryptedField<String>>,
    /// Employer/company
    pub employer: Option<EncryptedField<String>>,
    /// Job title
    pub job_title: Option<EncryptedField<String>>,
    /// Educational institution
    pub education: Option<EncryptedField<String>>,
    /// Social media usernames (JSON array)
    pub social_media: Option<EncryptedField<Vec<String>>>,
    /// Previous addresses (JSON array)
    #[deprecated(note = "Use previous_addresses_v2 instead")]
    pub previous_addresses_v1: Option<EncryptedField<Vec<String>>>,
    /// Phase 2: Phone numbers with type classification
    #[serde(default)]
    pub phone_numbers: Vec<PhoneNumber>,
    /// Phase 2: Previous addresses with structured data
    #[serde(default, rename = "previous_addresses_v2")]
    pub previous_addresses_v2: Vec<PreviousAddress>,
    /// Phase 2: Aliases or alternative names
    #[serde(default)]
    pub aliases: Vec<EncryptedField<String>>,
    /// Phase 2: Relatives and family members
    #[serde(default)]
    pub relatives: Vec<Relative>,
    /// Profile creation timestamp
    pub created_at: Timestamp,
    /// Profile last update timestamp
    pub updated_at: Timestamp,
}

/// Type of phone number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PhoneType {
    /// Mobile/cellular phone
    Mobile,
    /// Home phone
    Home,
    /// Work/office phone
    Work,
}

/// Phone number with type classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneNumber {
    /// Encrypted phone number
    pub number: EncryptedField<String>,
    /// Type of phone number
    pub phone_type: PhoneType,
}

/// Previous address with date range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviousAddress {
    /// Address line 1 (street)
    pub address_line1: EncryptedField<String>,
    /// Address line 2 (apt/suite) - optional
    pub address_line2: Option<EncryptedField<String>>,
    /// City
    pub city: EncryptedField<String>,
    /// State/province
    pub state: EncryptedField<String>,
    /// ZIP/postal code
    pub zip_code: EncryptedField<String>,
    /// Start date (YYYY-MM-DD format)
    pub lived_from: Option<String>,
    /// End date (YYYY-MM-DD format)
    pub lived_to: Option<String>,
}

/// Type of relationship to a relative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum RelationshipType {
    /// Spouse (married partner)
    Spouse,
    /// Partner (unmarried partner)
    Partner,
    /// Parent
    Parent,
    /// Child
    Child,
    /// Sibling (brother or sister)
    Sibling,
    /// Other relationship
    Other,
}

/// Relative or family member information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relative {
    /// Encrypted name of relative
    pub name: EncryptedField<String>,
    /// Type of relationship
    pub relationship: RelationshipType,
}

/// Profile completeness tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CompletenessTier {
    /// 0-30 points: Limited information
    Minimal,
    /// 31-60 points: Basic information
    Basic,
    /// 61-85 points: Good information
    Good,
    /// 86-100 points: Excellent information
    Excellent,
}

/// Profile completeness metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileCompleteness {
    /// Raw score (0-100)
    pub score: u32,
    /// Maximum possible score
    pub max_score: u32,
    /// Percentage (0-100)
    pub percentage: u32,
    /// Completeness tier
    pub tier: CompletenessTier,
    /// User-friendly message
    pub message: String,
}

impl UserProfile {
    /// Create a new empty user profile.
    #[must_use]
    #[allow(deprecated)]
    pub fn new(id: ProfileId) -> Self {
        let now = Timestamp::now();
        Self {
            id,
            full_name: None,
            first_name: None,
            middle_name: None,
            last_name: None,
            email: None,
            phone: None,
            address: None,
            city: None,
            state: None,
            zip_code: None,
            country: None,
            date_of_birth: None,
            ssn: None,
            employer: None,
            job_title: None,
            education: None,
            social_media: None,
            previous_addresses_v1: None,
            // Phase 2 fields
            phone_numbers: Vec::new(),
            previous_addresses_v2: Vec::new(),
            aliases: Vec::new(),
            relatives: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Save the profile to the database.
    ///
    /// The entire profile is serialized and stored as an encrypted blob
    /// in the profiles table.
    ///
    /// # Arguments
    /// * `db` - Database connection
    /// * `key` - Encryption key for the profile data blob
    ///
    /// # Errors
    /// Returns error if serialization or database operation fails.
    pub async fn save(&self, db: &Database, key: &[u8; 32]) -> Result<()> {
        // Serialize the profile
        let profile_json = serde_json::to_vec(self)
            .map_err(|e| VaultError::Serialization(format!("failed to serialize profile: {e}")))?;

        // Encrypt the entire profile blob
        let encrypted = EncryptedField::<Vec<u8>>::encrypt(&profile_json, key)?;

        // Store in database
        sqlx::query(
            "INSERT OR REPLACE INTO profiles (id, data, nonce, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(self.id.as_str())
        .bind(encrypted.ciphertext())
        .bind(&encrypted.nonce()[..])
        .bind(self.created_at.to_rfc3339())
        .bind(self.updated_at.to_rfc3339())
        .execute(db.pool())
        .await
        .map_err(spectral_db::DatabaseError::from)?;

        Ok(())
    }

    /// Load a profile from the database.
    ///
    /// # Arguments
    /// * `db` - Database connection
    /// * `id` - Profile ID to load
    /// * `key` - Encryption key for decrypting the profile data
    ///
    /// # Errors
    /// Returns error if profile not found, decryption fails, or deserialization fails.
    pub async fn load(db: &Database, id: &ProfileId, key: &[u8; 32]) -> Result<Self> {
        // Query the database
        let row = sqlx::query_as::<_, (Vec<u8>, Vec<u8>)>(
            "SELECT data, nonce FROM profiles WHERE id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(db.pool())
        .await
        .map_err(spectral_db::DatabaseError::from)?
        .ok_or_else(|| VaultError::NotFound(format!("profile {id}")))?;

        // Reconstruct encrypted field
        let nonce: [u8; 12] = row
            .1
            .try_into()
            .map_err(|_| VaultError::InvalidData("invalid nonce length".to_string()))?;

        let encrypted = EncryptedField::<Vec<u8>>::from_raw(row.0, nonce);

        // Decrypt
        let profile_json = encrypted.decrypt(key)?;

        // Deserialize
        let profile = serde_json::from_slice(&profile_json).map_err(|e| {
            VaultError::Serialization(format!("failed to deserialize profile: {e}"))
        })?;

        Ok(profile)
    }

    /// Delete a profile from the database.
    ///
    /// # Arguments
    /// * `db` - Database connection
    /// * `id` - Profile ID to delete
    ///
    /// # Errors
    /// Returns error if database operation fails.
    pub async fn delete(db: &Database, id: &ProfileId) -> Result<()> {
        sqlx::query("DELETE FROM profiles WHERE id = ?")
            .bind(id.as_str())
            .execute(db.pool())
            .await
            .map_err(spectral_db::DatabaseError::from)?;

        Ok(())
    }

    /// List all profile IDs in the database.
    ///
    /// # Arguments
    /// * `db` - Database connection
    ///
    /// # Errors
    /// Returns error if database operation fails.
    pub async fn list_ids(db: &Database) -> Result<Vec<ProfileId>> {
        let rows = sqlx::query_scalar::<_, String>(
            "SELECT id FROM profiles WHERE id != '__vault_verification__' ORDER BY created_at",
        )
        .fetch_all(db.pool())
        .await
        .map_err(spectral_db::DatabaseError::from)?;

        rows.into_iter()
            .map(|id| {
                ProfileId::new(id)
                    .map_err(|e| VaultError::InvalidData(format!("invalid profile ID: {e}")))
            })
            .collect()
    }

    /// Update the profile's `updated_at` timestamp.
    pub fn touch(&mut self) {
        self.updated_at = Timestamp::now();
    }

    /// Calculate profile completeness score.
    ///
    /// Scoring breakdown:
    /// - Core identity (40 points): `first_name` (15), `last_name` (15), email (10)
    /// - Current location (30 points): address (10), city (10), state+zip (10)
    /// - Enhanced matching (30 points): phones (10), `prev_addresses` (10), dob (5), aliases (3), relatives (2)
    #[must_use]
    pub fn completeness_score(&self) -> ProfileCompleteness {
        let mut score = 0u32;

        // Core identity (40 points)
        if self.first_name.is_some() {
            score += 15;
        }
        if self.last_name.is_some() {
            score += 15;
        }
        if self.email.is_some() {
            score += 10;
        }

        // Current location (30 points)
        if self.address.is_some() {
            score += 10;
        }
        if self.city.is_some() {
            score += 10;
        }
        if self.state.is_some() && self.zip_code.is_some() {
            score += 10;
        }

        // Enhanced matching (30 points)
        if !self.phone_numbers.is_empty() {
            score += 10;
        }
        if !self.previous_addresses_v2.is_empty() {
            score += 10;
        }
        if self.date_of_birth.is_some() {
            score += 5;
        }
        if !self.aliases.is_empty() {
            score += 3;
        }
        if !self.relatives.is_empty() {
            score += 2;
        }

        let tier = Self::score_to_tier(score);

        ProfileCompleteness {
            score,
            max_score: 100,
            percentage: score,
            tier,
            message: Self::tier_message(tier),
        }
    }

    fn score_to_tier(score: u32) -> CompletenessTier {
        match score {
            0..=30 => CompletenessTier::Minimal,
            31..=60 => CompletenessTier::Basic,
            61..=85 => CompletenessTier::Good,
            _ => CompletenessTier::Excellent,
        }
    }

    fn tier_message(tier: CompletenessTier) -> String {
        match tier {
            CompletenessTier::Minimal => {
                "Limited removal coverage - consider adding more information".to_string()
            }
            CompletenessTier::Basic => {
                "Basic removal coverage - adding contact info and addresses will improve results"
                    .to_string()
            }
            CompletenessTier::Good => {
                "Good removal coverage - you've provided solid information for effective removal"
                    .to_string()
            }
            CompletenessTier::Excellent => {
                "Excellent removal coverage - comprehensive information enables maximum removal effectiveness"
                    .to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cipher::encrypt_string;

    fn test_key() -> [u8; 32] {
        [0x42; 32]
    }

    #[test]
    fn test_new_profile() {
        let id = ProfileId::generate();
        let profile = UserProfile::new(id.clone());

        assert_eq!(profile.id, id);
        assert!(profile.full_name.is_none());
        assert!(profile.email.is_none());
    }

    #[test]
    fn test_profile_with_encrypted_fields() {
        let key = test_key();
        let mut profile = UserProfile::new(ProfileId::generate());

        profile.email = Some(encrypt_string("test@example.com", &key).expect("encrypt email"));
        profile.full_name = Some(encrypt_string("John Doe", &key).expect("encrypt full name"));

        assert!(profile.email.is_some());
        assert!(profile.full_name.is_some());
    }

    #[tokio::test]
    async fn test_save_and_load_profile() {
        let key = test_key();
        let db = Database::new(":memory:", key.to_vec())
            .await
            .expect("create database");
        db.run_migrations().await.expect("run migrations");

        let id = ProfileId::generate();
        let mut profile = UserProfile::new(id.clone());
        profile.email = Some(encrypt_string("test@example.com", &key).expect("encrypt"));
        profile.full_name = Some(encrypt_string("Alice Smith", &key).expect("encrypt"));

        // Save
        profile.save(&db, &key).await.expect("save profile");

        // Load
        let loaded = UserProfile::load(&db, &id, &key)
            .await
            .expect("load profile");

        assert_eq!(loaded.id, id);
        assert!(loaded.email.is_some());
        assert!(loaded.full_name.is_some());

        // Verify encrypted fields decrypt correctly
        let email = loaded
            .email
            .as_ref()
            .expect("email should be present")
            .decrypt(&key)
            .expect("decrypt email");
        assert_eq!(email, "test@example.com");

        let name = loaded
            .full_name
            .as_ref()
            .expect("full name should be present")
            .decrypt(&key)
            .expect("decrypt name");
        assert_eq!(name, "Alice Smith");
    }

    #[tokio::test]
    async fn test_load_nonexistent_profile() {
        let key = test_key();
        let db = Database::new(":memory:", key.to_vec())
            .await
            .expect("create database");
        db.run_migrations().await.expect("run migrations");

        let id = ProfileId::generate();
        let result = UserProfile::load(&db, &id, &key).await;

        assert!(result.is_err());
        match result {
            Err(VaultError::NotFound(_)) => {}
            _ => panic!("expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_delete_profile() {
        let key = test_key();
        let db = Database::new(":memory:", key.to_vec())
            .await
            .expect("create database");
        db.run_migrations().await.expect("run migrations");

        let id = ProfileId::generate();
        let profile = UserProfile::new(id.clone());

        // Save
        profile.save(&db, &key).await.expect("save profile");

        // Verify it exists
        let loaded = UserProfile::load(&db, &id, &key).await;
        assert!(loaded.is_ok());

        // Delete
        UserProfile::delete(&db, &id).await.expect("delete profile");

        // Verify it's gone
        let result = UserProfile::load(&db, &id, &key).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_profile_ids() {
        let key = test_key();
        let db = Database::new(":memory:", key.to_vec())
            .await
            .expect("create database");
        db.run_migrations().await.expect("run migrations");

        // Create and save multiple profiles
        let id1 = ProfileId::generate();
        let id2 = ProfileId::generate();
        let id3 = ProfileId::generate();

        UserProfile::new(id1.clone())
            .save(&db, &key)
            .await
            .expect("save profile 1");
        UserProfile::new(id2.clone())
            .save(&db, &key)
            .await
            .expect("save profile 2");
        UserProfile::new(id3.clone())
            .save(&db, &key)
            .await
            .expect("save profile 3");

        // List IDs
        let ids = UserProfile::list_ids(&db).await.expect("list IDs");

        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
        assert!(ids.contains(&id3));
    }

    #[tokio::test]
    async fn test_wrong_key_fails() {
        let key1 = [0x42; 32];
        let key2 = [0x43; 32];
        let db = Database::new(":memory:", key1.to_vec())
            .await
            .expect("create database");
        db.run_migrations().await.expect("run migrations");

        let id = ProfileId::generate();
        let profile = UserProfile::new(id.clone());

        // Save with key1
        profile.save(&db, &key1).await.expect("save profile");

        // Try to load with key2
        let result = UserProfile::load(&db, &id, &key2).await;

        assert!(result.is_err());
        match result {
            Err(VaultError::Decryption(_)) => {}
            _ => panic!("expected Decryption error"),
        }
    }

    #[test]
    fn test_touch_updates_timestamp() {
        let mut profile = UserProfile::new(ProfileId::generate());
        let original_time = profile.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        profile.touch();

        assert!(profile.updated_at > original_time);
    }

    #[test]
    fn test_phone_number_serialization() {
        let key = test_key();
        let phone = PhoneNumber {
            number: encrypt_string("555-123-4567", &key).expect("encrypt"),
            phone_type: PhoneType::Mobile,
        };

        let json = serde_json::to_string(&phone).expect("serialize");
        let deserialized: PhoneNumber = serde_json::from_str(&json).expect("deserialize");

        let decrypted = deserialized.number.decrypt(&key).expect("decrypt");
        assert_eq!(decrypted, "555-123-4567");
        assert_eq!(deserialized.phone_type, PhoneType::Mobile);
    }

    #[test]
    fn test_previous_address_serialization() {
        let key = test_key();
        let address = PreviousAddress {
            address_line1: encrypt_string("123 Old St", &key).expect("encrypt"),
            address_line2: Some(encrypt_string("Apt 4B", &key).expect("encrypt")),
            city: encrypt_string("Boston", &key).expect("encrypt"),
            state: encrypt_string("MA", &key).expect("encrypt"),
            zip_code: encrypt_string("02101", &key).expect("encrypt"),
            lived_from: Some("2015-01-01".to_string()),
            lived_to: Some("2020-12-31".to_string()),
        };

        let json = serde_json::to_string(&address).expect("serialize");
        let deserialized: PreviousAddress = serde_json::from_str(&json).expect("deserialize");

        let decrypted_line1 = deserialized.address_line1.decrypt(&key).expect("decrypt");
        assert_eq!(decrypted_line1, "123 Old St");
        let decrypted_line2 = deserialized
            .address_line2
            .as_ref()
            .expect("address_line2 should be present")
            .decrypt(&key)
            .expect("decrypt");
        assert_eq!(decrypted_line2, "Apt 4B");
        let decrypted_city = deserialized.city.decrypt(&key).expect("decrypt");
        assert_eq!(decrypted_city, "Boston");
        assert_eq!(deserialized.lived_from, Some("2015-01-01".to_string()));
        assert_eq!(deserialized.lived_to, Some("2020-12-31".to_string()));
    }

    #[test]
    fn test_relative_serialization() {
        let key = test_key();
        let relative = Relative {
            name: encrypt_string("Jane Doe", &key).expect("encrypt"),
            relationship: RelationshipType::Spouse,
        };

        let json = serde_json::to_string(&relative).expect("serialize");
        let deserialized: Relative = serde_json::from_str(&json).expect("deserialize");

        let decrypted = deserialized.name.decrypt(&key).expect("decrypt");
        assert_eq!(decrypted, "Jane Doe");
        assert_eq!(deserialized.relationship, RelationshipType::Spouse);
    }

    #[tokio::test]
    async fn test_profile_with_phase2_fields() {
        let key = test_key();
        let db = Database::new(":memory:", key.to_vec())
            .await
            .expect("create database");
        db.run_migrations().await.expect("run migrations");

        let id = ProfileId::generate();
        let mut profile = UserProfile::new(id.clone());

        // Add Phase 2 fields
        profile.phone_numbers = vec![PhoneNumber {
            number: encrypt_string("555-1234", &key).expect("encrypt"),
            phone_type: PhoneType::Mobile,
        }];

        profile.aliases = vec![encrypt_string("Johnny", &key).expect("encrypt")];

        profile.relatives = vec![Relative {
            name: encrypt_string("Jane", &key).expect("encrypt"),
            relationship: RelationshipType::Spouse,
        }];

        // Save and reload
        profile.save(&db, &key).await.expect("save");
        let loaded = UserProfile::load(&db, &id, &key).await.expect("load");

        assert_eq!(loaded.phone_numbers.len(), 1);
        assert_eq!(loaded.aliases.len(), 1);
        assert_eq!(loaded.relatives.len(), 1);
    }

    #[test]
    fn test_completeness_tier_minimal() {
        let profile = UserProfile::new(ProfileId::generate());
        let completeness = profile.completeness_score();

        assert_eq!(completeness.tier, CompletenessTier::Minimal);
        assert_eq!(completeness.score, 0);
        assert_eq!(completeness.percentage, 0);
    }

    #[test]
    fn test_completeness_tier_basic() {
        let key = test_key();
        let mut profile = UserProfile::new(ProfileId::generate());

        profile.first_name = Some(encrypt_string("John", &key).expect("encrypt"));
        profile.last_name = Some(encrypt_string("Doe", &key).expect("encrypt"));
        profile.email = Some(encrypt_string("john@example.com", &key).expect("encrypt"));

        let completeness = profile.completeness_score();

        assert_eq!(completeness.tier, CompletenessTier::Basic);
        assert_eq!(completeness.score, 40); // 15+15+10
    }

    #[test]
    fn test_completeness_tier_excellent() {
        let key = test_key();
        let mut profile = UserProfile::new(ProfileId::generate());

        // Core identity (40 points)
        profile.first_name = Some(encrypt_string("John", &key).expect("encrypt"));
        profile.last_name = Some(encrypt_string("Doe", &key).expect("encrypt"));
        profile.email = Some(encrypt_string("john@example.com", &key).expect("encrypt"));

        // Current location (30 points)
        profile.address = Some(encrypt_string("123 Main", &key).expect("encrypt"));
        profile.city = Some(encrypt_string("Chicago", &key).expect("encrypt"));
        profile.state = Some(encrypt_string("IL", &key).expect("encrypt"));
        profile.zip_code = Some(encrypt_string("60601", &key).expect("encrypt"));

        // Enhanced matching (30 points)
        profile.phone_numbers = vec![PhoneNumber {
            number: encrypt_string("555-1234", &key).expect("encrypt"),
            phone_type: PhoneType::Mobile,
        }];
        profile.previous_addresses_v2 = vec![PreviousAddress {
            address_line1: encrypt_string("456 Oak", &key).expect("encrypt"),
            address_line2: None,
            city: encrypt_string("Seattle", &key).expect("encrypt"),
            state: encrypt_string("WA", &key).expect("encrypt"),
            zip_code: encrypt_string("98101", &key).expect("encrypt"),
            lived_from: Some("2020-01-01".to_string()),
            lived_to: Some("2022-12-31".to_string()),
        }];
        profile.date_of_birth = Some(encrypt_string("1990-01-01", &key).expect("encrypt"));
        profile.aliases = vec![encrypt_string("Johnny", &key).expect("encrypt")];
        profile.relatives = vec![Relative {
            name: encrypt_string("Jane", &key).expect("encrypt"),
            relationship: RelationshipType::Spouse,
        }];

        let completeness = profile.completeness_score();

        assert_eq!(completeness.tier, CompletenessTier::Excellent);
        assert_eq!(completeness.score, 100);
        assert_eq!(completeness.percentage, 100);
    }
}
