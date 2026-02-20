//! Spectral Privacy - Centralized privacy controls and settings management

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

/// Error types for privacy operations.
pub mod error;

pub use error::{PrivacyError, Result};
