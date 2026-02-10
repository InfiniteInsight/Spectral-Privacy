//! Field-level encryption using ChaCha20-Poly1305 AEAD.
//!
//! Provides encryption and decryption of individual PII fields using
//! ChaCha20-Poly1305, an authenticated encryption algorithm.
//!
//! # Security Properties
//!
//! - **Confidentiality**: `ChaCha20` stream cipher
//! - **Authenticity**: `Poly1305` MAC
//! - **Nonce**: 96-bit random nonce per encryption
//! - **Key**: 256-bit derived from master password
//!
//! Each encrypted field includes its own nonce and authentication tag,
//! allowing independent encryption/decryption of fields.

use crate::error::{Result, VaultError};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use serde::{Deserialize, Serialize};

/// Length of the nonce in bytes (96 bits for ChaCha20-Poly1305).
pub const NONCE_LENGTH: usize = 12;

/// Encrypted field with ciphertext and nonce.
///
/// This structure stores an encrypted value along with its nonce,
/// which is required for decryption. The generic type `T` represents
/// the plaintext type before encryption.
///
/// # Type Parameter
/// * `T` - The type of the plaintext value (for type safety, not stored)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedField<T> {
    /// Ciphertext + authentication tag (16 bytes)
    ciphertext: Vec<u8>,
    /// Random nonce used for this encryption
    nonce: [u8; NONCE_LENGTH],
    /// Phantom data to maintain type safety
    #[serde(skip)]
    _phantom: std::marker::PhantomData<T>,
}

impl<T> EncryptedField<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    /// Create an `EncryptedField` from raw ciphertext and nonce.
    ///
    /// This is used when loading encrypted data from storage.
    #[must_use]
    pub fn from_raw(ciphertext: Vec<u8>, nonce: [u8; NONCE_LENGTH]) -> Self {
        Self {
            ciphertext,
            nonce,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Encrypt a value using the provided key.
    ///
    /// # Arguments
    /// * `value` - The value to encrypt (must be serializable)
    /// * `key` - The 256-bit encryption key
    ///
    /// # Returns
    /// An `EncryptedField` containing the ciphertext and nonce.
    ///
    /// # Errors
    /// Returns `VaultError::Encryption` if encryption or serialization fails.
    pub fn encrypt(value: &T, key: &[u8; 32]) -> Result<Self> {
        // Serialize the value to JSON
        let plaintext = serde_json::to_vec(value)
            .map_err(|e| VaultError::Encryption(format!("serialization failed: {e}")))?;

        // Generate random nonce
        let nonce_bytes = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let nonce_array: [u8; NONCE_LENGTH] = nonce_bytes
            .as_slice()
            .try_into()
            .expect("nonce has correct length");

        // Create cipher
        let cipher = ChaCha20Poly1305::new(key.into());

        // Encrypt
        let ciphertext = cipher
            .encrypt(&nonce_bytes, plaintext.as_ref())
            .map_err(|e| VaultError::Encryption(format!("encryption failed: {e}")))?;

        Ok(Self {
            ciphertext,
            nonce: nonce_array,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Decrypt the field using the provided key.
    ///
    /// # Arguments
    /// * `key` - The 256-bit encryption key (must match the key used for encryption)
    ///
    /// # Returns
    /// The decrypted value of type `T`.
    ///
    /// # Errors
    /// Returns `VaultError::Decryption` if:
    /// - The key is incorrect
    /// - The ciphertext has been tampered with
    /// - Deserialization fails
    pub fn decrypt(&self, key: &[u8; 32]) -> Result<T> {
        // Create cipher
        let cipher = ChaCha20Poly1305::new(key.into());

        // Decrypt
        let nonce = Nonce::from_slice(&self.nonce);
        let plaintext = cipher
            .decrypt(nonce, self.ciphertext.as_ref())
            .map_err(|e| VaultError::Decryption(format!("decryption failed: {e}")))?;

        // Deserialize
        let value = serde_json::from_slice(&plaintext)
            .map_err(|e| VaultError::Decryption(format!("deserialization failed: {e}")))?;

        Ok(value)
    }

    /// Get the size of the ciphertext in bytes.
    #[must_use]
    pub fn ciphertext_len(&self) -> usize {
        self.ciphertext.len()
    }

    /// Get the nonce as a byte slice.
    #[must_use]
    pub fn nonce(&self) -> &[u8; NONCE_LENGTH] {
        &self.nonce
    }

    /// Get the ciphertext as a byte slice.
    #[must_use]
    pub fn ciphertext(&self) -> &[u8] {
        &self.ciphertext
    }
}

/// Encrypt a string value.
///
/// Convenience function for encrypting strings without needing to specify the type.
pub fn encrypt_string(value: &str, key: &[u8; 32]) -> Result<EncryptedField<String>> {
    EncryptedField::encrypt(&value.to_string(), key)
}

/// Decrypt a string value.
///
/// Convenience function for decrypting strings.
pub fn decrypt_string(field: &EncryptedField<String>, key: &[u8; 32]) -> Result<String> {
    field.decrypt(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        [0x42; 32] // Fixed key for testing
    }

    #[test]
    fn test_encrypt_decrypt_string() {
        let key = test_key();
        let original = "test@example.com";

        let encrypted = encrypt_string(original, &key).expect("encrypt");
        let decrypted = decrypt_string(&encrypted, &key).expect("decrypt");

        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_encrypt_decrypt_generic() {
        let key = test_key();

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct TestData {
            name: String,
            age: u32,
        }

        let original = TestData {
            name: "Alice".to_string(),
            age: 30,
        };

        let encrypted = EncryptedField::encrypt(&original, &key).expect("encrypt");
        let decrypted: TestData = encrypted.decrypt(&key).expect("decrypt");

        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_different_nonces() {
        let key = test_key();
        let value = "test";

        let encrypted1 = encrypt_string(value, &key).expect("encrypt 1");
        let encrypted2 = encrypt_string(value, &key).expect("encrypt 2");

        // Same plaintext should produce different ciphertexts due to different nonces
        assert_ne!(encrypted1.nonce(), encrypted2.nonce());
        assert_ne!(encrypted1.ciphertext, encrypted2.ciphertext);

        // Both should decrypt correctly
        assert_eq!(decrypt_string(&encrypted1, &key).expect("decrypt 1"), value);
        assert_eq!(decrypt_string(&encrypted2, &key).expect("decrypt 2"), value);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = [0x42; 32];
        let key2 = [0x43; 32];
        let value = "secret";

        let encrypted = encrypt_string(value, &key1).expect("encrypt");
        let result = decrypt_string(&encrypted, &key2);

        assert!(result.is_err());
        match result {
            Err(VaultError::Decryption(_)) => {}
            _ => panic!("expected Decryption error"),
        }
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let key = test_key();
        let value = "secret";

        let mut encrypted = encrypt_string(value, &key).expect("encrypt");

        // Tamper with the ciphertext
        if let Some(byte) = encrypted.ciphertext.first_mut() {
            *byte ^= 0xFF;
        }

        let result = decrypt_string(&encrypted, &key);

        assert!(result.is_err());
        match result {
            Err(VaultError::Decryption(_)) => {}
            _ => panic!("expected Decryption error"),
        }
    }

    #[test]
    fn test_tampered_nonce_fails() {
        let key = test_key();
        let value = "secret";

        let mut encrypted = encrypt_string(value, &key).expect("encrypt");

        // Tamper with the nonce
        encrypted.nonce[0] ^= 0xFF;

        let result = decrypt_string(&encrypted, &key);

        assert!(result.is_err());
        match result {
            Err(VaultError::Decryption(_)) => {}
            _ => panic!("expected Decryption error"),
        }
    }

    #[test]
    fn test_serialization_roundtrip() {
        let key = test_key();
        let value = "test@example.com";

        let encrypted = encrypt_string(value, &key).expect("encrypt");

        // Serialize to JSON
        let json = serde_json::to_string(&encrypted).expect("serialize");

        // Deserialize from JSON
        let deserialized: EncryptedField<String> =
            serde_json::from_str(&json).expect("deserialize");

        // Should still decrypt correctly
        let decrypted = decrypt_string(&deserialized, &key).expect("decrypt");
        assert_eq!(decrypted, value);
    }

    #[test]
    fn test_empty_string() {
        let key = test_key();
        let value = "";

        let encrypted = encrypt_string(value, &key).expect("encrypt");
        let decrypted = decrypt_string(&encrypted, &key).expect("decrypt");

        assert_eq!(decrypted, value);
    }

    #[test]
    fn test_unicode() {
        let key = test_key();
        let value = "Hello 世界 🌍";

        let encrypted = encrypt_string(value, &key).expect("encrypt");
        let decrypted = decrypt_string(&encrypted, &key).expect("decrypt");

        assert_eq!(decrypted, value);
    }

    #[test]
    fn test_ciphertext_length() {
        let key = test_key();
        let value = "test";

        let encrypted = encrypt_string(value, &key).expect("encrypt");

        // Ciphertext should be longer than plaintext due to authentication tag (16 bytes)
        assert!(encrypted.ciphertext_len() > value.len());
    }
}
