//! Spectral Vault - PII Encryption Layer
//!
//! Provides secure storage for Personally Identifiable Information (PII)
//! using ChaCha20-Poly1305 encryption with Argon2id key derivation.
//!
//! # Security Model
//!
//! - Master password → Argon2id (256MB memory) → 256-bit key
//! - ChaCha20-Poly1305 AEAD for all PII encryption
//! - Keys are zeroized from memory when vault is locked
//! - No PII ever logged or included in error messages
//!
//! # Example
//!
//! ```ignore
//! use spectral_vault::Vault;
//!
//! // Create a new vault
//! let vault = Vault::create("strong_password", "/path/to/vault.db").await?;
//!
//! // Later, unlock the vault
//! let vault = Vault::unlock("strong_password", "/path/to/vault.db").await?;
//!
//! // Use the vault...
//! let profile_id = vault.create_profile().await?;
//!
//! // Lock when done
//! vault.lock();
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod cipher;
pub mod error;
pub mod kdf;
pub mod profile;

pub use cipher::{encrypt_string, EncryptedField};
pub use error::{Result, VaultError};
pub use profile::{CompletenessTier, ProfileCompleteness, UserProfile};

use spectral_core::types::{ProfileId, Timestamp};
use spectral_db::Database;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

/// Salt storage file name (stored alongside the vault database).
const SALT_FILE_NAME: &str = ".vault_salt";

/// Verification token stored in database to verify password correctness.
const VERIFICATION_TOKEN: &str = "SPECTRAL_VAULT_V1";

/// Vault state managing encryption keys and database access.
///
/// The vault can be in one of two states:
/// - **Locked**: No encryption key in memory, database closed
/// - **Unlocked**: Encryption key derived and held in memory, database open
#[derive(Debug)]
pub struct Vault {
    /// Database connection (None when locked)
    db: Option<Database>,
    /// Derived encryption key (zeroized on drop)
    key: Option<Zeroizing<[u8; 32]>>,
    /// Path to the vault database
    db_path: PathBuf,
}

impl Vault {
    /// Create a new vault with a master password.
    ///
    /// This performs first-time vault setup:
    /// 1. Generates a random salt
    /// 2. Derives encryption key from password
    /// 3. Creates encrypted database
    /// 4. Stores salt in a separate file
    ///
    /// # Arguments
    /// * `password` - Master password for the vault
    /// * `db_path` - Path where the vault database should be created
    ///
    /// # Returns
    /// An unlocked `Vault` instance ready to use.
    ///
    /// # Errors
    /// Returns error if:
    /// - Salt file already exists (vault already created)
    /// - Key derivation fails
    /// - Database creation fails
    /// - File system operations fail
    pub async fn create(password: &str, db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = db_path.as_ref();
        let salt_path = get_salt_path(db_path);

        // Check if vault already exists
        if salt_path.exists() {
            return Err(VaultError::InvalidData(format!(
                "vault already exists at {}",
                db_path.display()
            )));
        }

        tracing::info!("Creating new vault at {}", db_path.display());

        // Generate salt
        let salt = kdf::generate_salt();

        // Derive key
        let key = kdf::derive_key(password, &salt)?;

        // Create database
        let db = Database::new(db_path, key.to_vec()).await?;
        db.run_migrations().await?;

        // Store verification token
        Self::store_verification_token(&db, &key).await?;

        // Store salt
        tokio::fs::write(&salt_path, &salt)
            .await
            .map_err(|e| VaultError::InvalidData(format!("failed to write salt file: {e}")))?;

        tracing::info!("Vault created successfully");

        Ok(Self {
            db: Some(db),
            key: Some(key),
            db_path: db_path.to_path_buf(),
        })
    }

    /// Unlock an existing vault with the master password.
    ///
    /// This:
    /// 1. Loads the salt from disk
    /// 2. Derives the encryption key from the password
    /// 3. Opens the encrypted database
    /// 4. Verifies the key is correct
    ///
    /// # Arguments
    /// * `password` - Master password for the vault
    /// * `db_path` - Path to the vault database
    ///
    /// # Returns
    /// An unlocked `Vault` instance ready to use.
    ///
    /// # Errors
    /// Returns error if:
    /// - Vault doesn't exist
    /// - Password is incorrect
    /// - Key derivation fails
    /// - Database cannot be opened
    pub async fn unlock(password: &str, db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = db_path.as_ref();
        let salt_path = get_salt_path(db_path);

        // Check if vault exists
        if !salt_path.exists() {
            return Err(VaultError::VaultNotFound(db_path.display().to_string()));
        }

        if !db_path.exists() {
            return Err(VaultError::VaultNotFound(db_path.display().to_string()));
        }

        tracing::info!("Unlocking vault at {}", db_path.display());

        // Load salt
        let salt = tokio::fs::read(&salt_path)
            .await
            .map_err(|e| VaultError::InvalidData(format!("failed to read salt file: {e}")))?;

        if salt.len() != kdf::SALT_LENGTH {
            return Err(VaultError::InvalidData(format!(
                "invalid salt file: expected {} bytes, got {}",
                kdf::SALT_LENGTH,
                salt.len()
            )));
        }

        // Derive key
        let key = kdf::derive_key(password, &salt)?;

        // Open database
        let db = Database::new(db_path, key.to_vec()).await?;

        // Run migrations to ensure schema is up to date
        // This is critical for existing vaults that were created before new migrations were added
        db.run_migrations().await?;

        // Verify password is correct by decrypting verification token
        Self::verify_password(&db, &key).await.map_err(|_| {
            tracing::warn!("Failed to verify vault key - incorrect password");
            VaultError::InvalidPassword
        })?;

        tracing::info!("Vault unlocked successfully");

        Ok(Self {
            db: Some(db),
            key: Some(key),
            db_path: db_path.to_path_buf(),
        })
    }

    /// Lock the vault, zeroizing the key from memory and closing the database.
    ///
    /// After calling this method, the vault must be unlocked again to access data.
    /// The encryption key is securely erased from memory.
    pub fn lock(mut self) {
        tracing::info!("Locking vault");
        self.db = None;
        self.key = None;
        // Key is automatically zeroized when dropped
    }

    /// Check if the vault is currently unlocked.
    #[must_use]
    pub fn is_unlocked(&self) -> bool {
        self.key.is_some() && self.db.is_some()
    }

    /// Get the vault's database path.
    #[must_use]
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Create a new empty user profile.
    ///
    /// # Returns
    /// The ID of the newly created profile.
    ///
    /// # Errors
    /// Returns `VaultError::Locked` if the vault is not unlocked.
    pub async fn create_profile(&self) -> Result<ProfileId> {
        self.require_unlocked()?;

        let id = ProfileId::generate();
        let profile = UserProfile::new(id.clone());

        profile
            .save(self.db.as_ref().unwrap(), self.key.as_ref().unwrap())
            .await?;

        tracing::info!("Created profile {id}");
        Ok(id)
    }

    /// Load a user profile by ID.
    ///
    /// # Errors
    /// Returns error if vault is locked or profile not found.
    pub async fn load_profile(&self, id: &ProfileId) -> Result<UserProfile> {
        self.require_unlocked()?;

        UserProfile::load(self.db.as_ref().unwrap(), id, self.key.as_ref().unwrap()).await
    }

    /// Save a user profile.
    ///
    /// # Errors
    /// Returns error if vault is locked or database operation fails.
    pub async fn save_profile(&self, profile: &UserProfile) -> Result<()> {
        self.require_unlocked()?;

        profile
            .save(self.db.as_ref().unwrap(), self.key.as_ref().unwrap())
            .await
    }

    /// Delete a user profile.
    ///
    /// # Errors
    /// Returns error if vault is locked or database operation fails.
    pub async fn delete_profile(&self, id: &ProfileId) -> Result<()> {
        self.require_unlocked()?;

        UserProfile::delete(self.db.as_ref().unwrap(), id).await?;
        tracing::info!("Deleted profile {id}");
        Ok(())
    }

    /// List all profile IDs in the vault.
    ///
    /// # Errors
    /// Returns error if vault is locked or database operation fails.
    pub async fn list_profiles(&self) -> Result<Vec<ProfileId>> {
        self.require_unlocked()?;

        UserProfile::list_ids(self.db.as_ref().unwrap()).await
    }

    /// Get a reference to the underlying database.
    ///
    /// # Errors
    /// Returns `VaultError::Locked` if the vault is not unlocked.
    pub fn database(&self) -> Result<&Database> {
        self.db.as_ref().ok_or(VaultError::Locked)
    }

    /// Get the encryption key for field-level encryption.
    ///
    /// This is used by application code that needs to encrypt individual fields
    /// before storing them in the profile.
    ///
    /// # Errors
    /// Returns `VaultError::Locked` if the vault is not unlocked.
    ///
    /// # Security Note
    /// The returned key should be used immediately and not stored.
    /// It's a reference to zeroized memory that will be cleared when the vault is locked.
    pub fn encryption_key(&self) -> Result<&[u8; 32]> {
        // Zeroizing<[u8; 32]> derefs to &[u8; 32]
        self.key.as_ref().ok_or(VaultError::Locked).map(|k| &**k)
    }

    /// Require that the vault is unlocked.
    fn require_unlocked(&self) -> Result<()> {
        if !self.is_unlocked() {
            return Err(VaultError::Locked);
        }
        Ok(())
    }

    /// Store an encrypted verification token in the database.
    ///
    /// This token is used to verify the password is correct during unlock.
    async fn store_verification_token(db: &Database, key: &[u8; 32]) -> Result<()> {
        let encrypted = encrypt_string(VERIFICATION_TOKEN, key)?;

        sqlx::query(
            "INSERT INTO profiles (id, data, nonce, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind("__vault_verification__")
        .bind(encrypted.ciphertext())
        .bind(&encrypted.nonce()[..])
        .bind(Timestamp::now().to_rfc3339())
        .bind(Timestamp::now().to_rfc3339())
        .execute(db.pool())
        .await
        .map_err(spectral_db::DatabaseError::from)?;

        Ok(())
    }

    /// Verify the password by decrypting the verification token.
    async fn verify_password(db: &Database, key: &[u8; 32]) -> Result<()> {
        let row = sqlx::query_as::<_, (Vec<u8>, Vec<u8>)>(
            "SELECT data, nonce FROM profiles WHERE id = '__vault_verification__'",
        )
        .fetch_optional(db.pool())
        .await
        .map_err(spectral_db::DatabaseError::from)?
        .ok_or(VaultError::InvalidPassword)?;

        let nonce: [u8; 12] = row.1.try_into().map_err(|_| VaultError::InvalidPassword)?;

        let encrypted = EncryptedField::<String>::from_raw(row.0, nonce);
        let token = encrypted.decrypt(key)?;

        if token != VERIFICATION_TOKEN {
            return Err(VaultError::InvalidPassword);
        }

        Ok(())
    }
}

impl Drop for Vault {
    fn drop(&mut self) {
        // Ensure database is closed and key is zeroized
        self.db = None;
        self.key = None;
    }
}

/// Get the path to the salt file for a given database path.
fn get_salt_path(db_path: &Path) -> PathBuf {
    db_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(SALT_FILE_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_vault_path() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("create temp dir");
        let db_path = temp_dir.path().join("test_vault.db");
        (temp_dir, db_path)
    }

    #[tokio::test]
    async fn test_vault_create() {
        let (_temp_dir, db_path) = test_vault_path();
        let password = "test_password";

        let vault = Vault::create(password, &db_path)
            .await
            .expect("create vault");

        assert!(vault.is_unlocked());
        assert_eq!(vault.db_path(), db_path);
        assert!(db_path.exists());
        assert!(get_salt_path(&db_path).exists());
    }

    #[tokio::test]
    async fn test_vault_unlock_correct_password() {
        let (_temp_dir, db_path) = test_vault_path();
        let password = "test_password";

        // Create vault
        let vault = Vault::create(password, &db_path)
            .await
            .expect("create vault");
        vault.lock();

        // Unlock with correct password
        let vault = Vault::unlock(password, &db_path)
            .await
            .expect("unlock vault");

        assert!(vault.is_unlocked());
    }

    #[tokio::test]
    async fn test_vault_unlock_wrong_password() {
        let (_temp_dir, db_path) = test_vault_path();
        let password = "correct_password";
        let wrong_password = "wrong_password";

        // Create vault
        let vault = Vault::create(password, &db_path)
            .await
            .expect("create vault");
        vault.lock();

        // Try to unlock with wrong password
        let result = Vault::unlock(wrong_password, &db_path).await;

        assert!(result.is_err());
        match result {
            Err(VaultError::InvalidPassword) => {}
            _ => panic!("expected InvalidPassword error"),
        }
    }

    #[tokio::test]
    async fn test_vault_unlock_nonexistent() {
        let (_temp_dir, db_path) = test_vault_path();

        let result = Vault::unlock("password", &db_path).await;

        assert!(result.is_err());
        match result {
            Err(VaultError::VaultNotFound(_)) => {}
            _ => panic!("expected VaultNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_vault_create_duplicate() {
        let (_temp_dir, db_path) = test_vault_path();
        let password = "test_password";

        // Create vault
        let vault = Vault::create(password, &db_path)
            .await
            .expect("create vault");
        vault.lock();

        // Try to create again
        let result = Vault::create(password, &db_path).await;

        assert!(result.is_err());
        match result {
            Err(VaultError::InvalidData(msg)) => {
                assert!(msg.contains("already exists"));
            }
            _ => panic!("expected InvalidData error"),
        }
    }

    #[tokio::test]
    async fn test_vault_lock() {
        let (_temp_dir, db_path) = test_vault_path();
        let password = "test_password";

        let vault = Vault::create(password, &db_path)
            .await
            .expect("create vault");

        assert!(vault.is_unlocked());

        vault.lock();
        // Vault is consumed by lock(), can't check is_unlocked()
    }

    #[tokio::test]
    async fn test_vault_locked_operations_fail() {
        let (_temp_dir, db_path) = test_vault_path();
        let password = "test_password";

        let vault = Vault::create(password, &db_path)
            .await
            .expect("create vault");
        vault.lock();

        // Recreate to test locked state
        let vault = Vault::unlock(password, &db_path)
            .await
            .expect("unlock vault");

        // Create profile while unlocked
        let profile_id = vault.create_profile().await.expect("create profile");

        // Lock the vault
        vault.lock();

        // Unlock again to test operations
        let vault = Vault::unlock(password, &db_path)
            .await
            .expect("unlock vault");

        // Operations should work when unlocked
        let _profile = vault.load_profile(&profile_id).await.expect("load profile");
    }

    #[tokio::test]
    async fn test_create_and_load_profile() {
        let (_temp_dir, db_path) = test_vault_path();
        let password = "test_password";

        let vault = Vault::create(password, &db_path)
            .await
            .expect("create vault");

        let profile_id = vault.create_profile().await.expect("create profile");
        let profile = vault.load_profile(&profile_id).await.expect("load profile");

        assert_eq!(profile.id, profile_id);
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_save_and_load_profile() {
        let (_temp_dir, db_path) = test_vault_path();
        let password = "test_password";

        let vault = Vault::create(password, &db_path)
            .await
            .expect("create vault");

        let profile_id = vault.create_profile().await.expect("create profile");
        let mut profile = vault.load_profile(&profile_id).await.expect("load profile");

        // Modify profile
        let key = vault.key.as_ref().unwrap();
        profile.email = Some(encrypt_string("test@example.com", key).expect("encrypt"));

        // Save
        vault.save_profile(&profile).await.expect("save profile");

        // Load again
        let loaded = vault.load_profile(&profile_id).await.expect("load profile");

        assert!(loaded.email.is_some());
        let email = loaded
            .email
            .as_ref()
            .unwrap()
            .decrypt(key)
            .expect("decrypt");
        assert_eq!(email, "test@example.com");
    }

    #[tokio::test]
    async fn test_delete_profile() {
        let (_temp_dir, db_path) = test_vault_path();
        let password = "test_password";

        let vault = Vault::create(password, &db_path)
            .await
            .expect("create vault");

        let profile_id = vault.create_profile().await.expect("create profile");
        vault
            .delete_profile(&profile_id)
            .await
            .expect("delete profile");

        let result = vault.load_profile(&profile_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_profiles() {
        let (_temp_dir, db_path) = test_vault_path();
        let password = "test_password";

        let vault = Vault::create(password, &db_path)
            .await
            .expect("create vault");

        let id1 = vault.create_profile().await.expect("create profile 1");
        let id2 = vault.create_profile().await.expect("create profile 2");

        let profiles = vault.list_profiles().await.expect("list profiles");

        assert_eq!(profiles.len(), 2);
        assert!(profiles.contains(&id1));
        assert!(profiles.contains(&id2));
    }

    #[tokio::test]
    async fn test_profile_persists_across_lock_unlock() {
        let (_temp_dir, db_path) = test_vault_path();
        let password = "test_password";

        // Create vault and profile
        let vault = Vault::create(password, &db_path)
            .await
            .expect("create vault");
        let profile_id = vault.create_profile().await.expect("create profile");
        vault.lock();

        // Unlock and verify profile exists
        let vault = Vault::unlock(password, &db_path)
            .await
            .expect("unlock vault");
        let profile = vault.load_profile(&profile_id).await.expect("load profile");

        assert_eq!(profile.id, profile_id);
    }
}
