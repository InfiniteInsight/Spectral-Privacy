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
    pub previous_addresses: Option<EncryptedField<Vec<String>>>,
    /// Profile creation timestamp
    pub created_at: Timestamp,
    /// Profile last update timestamp
    pub updated_at: Timestamp,
}

impl UserProfile {
    /// Create a new empty user profile.
    #[must_use]
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
            previous_addresses: None,
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
            .unwrap()
            .decrypt(&key)
            .expect("decrypt email");
        assert_eq!(email, "test@example.com");

        let name = loaded
            .full_name
            .as_ref()
            .unwrap()
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
}
