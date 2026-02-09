//! Spectral Authentication Layer
//!
//! Handles app-level authentication including PIN, biometrics, and session management.
//! This layer controls access to the application, separate from vault encryption.
//!
//! # Authentication Layers
//!
//! 1. **App Launch**: PIN/Biometrics to open the app
//! 2. **Vault Unlock**: Master password to decrypt PII (handled by spectral-vault)
//! 3. **External Services**: API keys, OAuth tokens (encrypted in vault)
//!
//! # Platform Support
//!
//! - **Windows**: Windows Hello (face, fingerprint, PIN)
//! - **macOS**: Touch ID via LocalAuthentication framework
//! - **Linux**: fprintd (fingerprint), polkit, PAM
//!
//! # Session Management
//!
//! - Auto-lock after configurable inactivity (default: 15 minutes)
//! - Rate limiting: 5 failed attempts → 5 minute lockout
//! - Session tokens zeroized from memory on lock

use std::time::Duration;
use thiserror::Error;
use zeroize::Zeroizing;

/// Authentication errors
#[derive(Debug, Error)]
pub enum AuthError {
    /// Invalid PIN provided
    #[error("invalid PIN")]
    InvalidPin,

    /// Biometric authentication failed
    #[error("biometric authentication failed")]
    BiometricFailed,

    /// Biometrics not available on this platform
    #[error("biometrics not available")]
    BiometricsUnavailable,

    /// Too many failed attempts, locked out
    #[error("too many failed attempts, locked for {0:?}")]
    RateLimited(Duration),

    /// Session has expired
    #[error("session expired")]
    SessionExpired,

    /// Not authenticated
    #[error("not authenticated")]
    NotAuthenticated,
}

/// Result type for authentication operations
pub type Result<T> = std::result::Result<T, AuthError>;

/// Authentication state
#[derive(Debug, Default)]
pub struct AuthState {
    /// Whether the user is authenticated
    authenticated: bool,
    /// Session token (zeroized on logout)
    session_token: Option<Zeroizing<[u8; 32]>>,
    /// Failed attempt count
    failed_attempts: u32,
}

impl AuthState {
    /// Create a new unauthenticated state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if user is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    /// Get the number of failed attempts
    pub fn failed_attempts(&self) -> u32 {
        self.failed_attempts
    }

    /// Lock the session
    pub fn lock(&mut self) {
        self.authenticated = false;
        self.session_token = None;
        tracing::info!("Session locked");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_initial_state() {
        let auth = AuthState::new();
        assert!(!auth.is_authenticated());
        assert_eq!(auth.failed_attempts(), 0);
    }

    #[test]
    fn test_auth_lock() {
        let mut auth = AuthState::new();
        auth.lock();
        assert!(!auth.is_authenticated());
    }
}
