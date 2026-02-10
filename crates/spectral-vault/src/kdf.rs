//! Key Derivation Function (KDF) using Argon2id.
//!
//! Provides secure key derivation from user passwords using Argon2id,
//! the recommended algorithm for password hashing and key derivation.
//!
//! # Security Parameters
//!
//! - Algorithm: Argon2id (hybrid mode)
//! - Memory cost: 256 MB (262,144 KB)
//! - Time cost: 2 iterations
//! - Parallelism: 1 thread
//! - Output: 32 bytes (256 bits)
//!
//! These parameters balance security and usability for desktop applications.

use crate::error::{Result, VaultError};
use argon2::{Algorithm, Argon2, ParamsBuilder, Version};
use rand::{rngs::OsRng, RngCore};
use zeroize::Zeroizing;

/// Length of the derived key in bytes (256 bits).
pub const KEY_LENGTH: usize = 32;

/// Length of the salt in bytes.
pub const SALT_LENGTH: usize = 32;

/// Argon2id memory cost in KB (256 MB).
const MEMORY_COST_KB: u32 = 262_144;

/// Argon2id time cost (iterations).
const TIME_COST: u32 = 2;

/// Argon2id parallelism (threads).
const PARALLELISM: u32 = 1;

/// Generate a random salt for key derivation.
///
/// Returns a cryptographically secure random 32-byte salt.
#[must_use]
pub fn generate_salt() -> [u8; SALT_LENGTH] {
    let mut salt = [0u8; SALT_LENGTH];
    OsRng.fill_bytes(&mut salt);
    salt
}

/// Derive a 256-bit encryption key from a password using Argon2id.
///
/// # Arguments
/// * `password` - The user's master password
/// * `salt` - A 32-byte salt (should be generated once and stored)
///
/// # Returns
/// A zeroizing wrapper around the derived 32-byte key.
///
/// # Errors
/// Returns `VaultError::KeyDerivation` if the derivation fails.
///
/// # Example
/// ```ignore
/// use spectral_vault::kdf::{generate_salt, derive_key};
///
/// let salt = generate_salt();
/// let key = derive_key("my_strong_password", &salt)?;
/// // Use key for encryption...
/// // Key is automatically zeroized when dropped
/// ```
pub fn derive_key(password: &str, salt: &[u8]) -> Result<Zeroizing<[u8; KEY_LENGTH]>> {
    // Validate salt length
    if salt.len() != SALT_LENGTH {
        return Err(VaultError::KeyDerivation(format!(
            "invalid salt length: expected {SALT_LENGTH} bytes, got {}",
            salt.len()
        )));
    }

    // Build Argon2id parameters
    let params = ParamsBuilder::new()
        .m_cost(MEMORY_COST_KB)
        .t_cost(TIME_COST)
        .p_cost(PARALLELISM)
        .output_len(KEY_LENGTH)
        .build()
        .map_err(|e| VaultError::KeyDerivation(format!("failed to build parameters: {e}")))?;

    // Create Argon2 instance
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    // Derive key
    let mut key = Zeroizing::new([0u8; KEY_LENGTH]);
    argon2
        .hash_password_into(password.as_bytes(), salt, key.as_mut())
        .map_err(|e| VaultError::KeyDerivation(format!("key derivation failed: {e}")))?;

    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_salt() {
        let salt1 = generate_salt();
        let salt2 = generate_salt();

        // Salts should be unique
        assert_ne!(salt1, salt2);
        assert_eq!(salt1.len(), SALT_LENGTH);
        assert_eq!(salt2.len(), SALT_LENGTH);
    }

    #[test]
    fn test_derive_key_deterministic() {
        let salt = generate_salt();
        let password = "test_password";

        let key1 = derive_key(password, &salt).expect("derive key 1");
        let key2 = derive_key(password, &salt).expect("derive key 2");

        // Same password and salt should produce same key
        assert_eq!(key1.as_ref(), key2.as_ref());
    }

    #[test]
    fn test_derive_key_different_passwords() {
        let salt = generate_salt();

        let key1 = derive_key("password1", &salt).expect("derive key 1");
        let key2 = derive_key("password2", &salt).expect("derive key 2");

        // Different passwords should produce different keys
        assert_ne!(key1.as_ref(), key2.as_ref());
    }

    #[test]
    fn test_derive_key_different_salts() {
        let salt1 = generate_salt();
        let salt2 = generate_salt();
        let password = "test_password";

        let key1 = derive_key(password, &salt1).expect("derive key 1");
        let key2 = derive_key(password, &salt2).expect("derive key 2");

        // Different salts should produce different keys
        assert_ne!(key1.as_ref(), key2.as_ref());
    }

    #[test]
    fn test_derive_key_invalid_salt_length() {
        let invalid_salt = [0u8; 16]; // Wrong length
        let result = derive_key("password", &invalid_salt);

        assert!(result.is_err());
        match result {
            Err(VaultError::KeyDerivation(msg)) => {
                assert!(msg.contains("invalid salt length"));
            }
            _ => panic!("expected KeyDerivation error"),
        }
    }

    #[test]
    fn test_key_length() {
        let salt = generate_salt();
        let key = derive_key("password", &salt).expect("derive key");

        assert_eq!(key.len(), KEY_LENGTH);
    }

    #[test]
    fn test_empty_password() {
        let salt = generate_salt();
        let key = derive_key("", &salt).expect("derive key from empty password");

        // Should still work, just not recommended
        assert_eq!(key.len(), KEY_LENGTH);
    }
}
