//! Spectral Scanner - Broker scanning orchestration.
//!
//! This crate provides the scanning infrastructure for discovering PII on data broker sites.
//! It coordinates browser automation, result parsing, and findings storage with robust
//! error handling including retry logic, CAPTCHA detection, and rate limiting.
//!
//! # Features
//!
//! - Concurrent scanning of multiple brokers with configurable parallelism
//! - Retry logic with exponential backoff for transient failures
//! - CAPTCHA detection and reporting
//! - Rate limit handling with extended backoff
//! - Automatic findings storage in encrypted database
//!
//! # Example
//!
//! ```rust,ignore
//! use spectral_scanner::ScanOrchestrator;
//! use std::sync::Arc;
//!
//! let orchestrator = ScanOrchestrator::new(
//!     Arc::new(broker_registry),
//!     Arc::new(browser_engine),
//!     Arc::new(database),
//! );
//!
//! let results = orchestrator.execute_scan_job(
//!     scan_job_id,
//!     broker_ids,
//!     profile_id,
//!     vault_key,
//! ).await?;
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod error;
#[allow(missing_docs)]
pub mod filter;
pub mod orchestrator;
#[allow(missing_docs)]
pub mod parser;
#[allow(missing_docs)]
pub mod url_builder;

// Re-export commonly used types
pub use error::{Result, ScanError};
pub use filter::{check_profile_completeness, BrokerFilter};
pub use orchestrator::{BrokerScanResult, ScanOrchestrator};
pub use parser::{ExtractedData, ListingMatch, ResultParser};
pub use url_builder::build_search_url;
