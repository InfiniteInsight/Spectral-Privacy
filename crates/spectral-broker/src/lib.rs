//! Spectral Broker - Broker definition system for data broker scanning and removal.
//!
//! This crate provides the core types and functionality for managing data broker
//! definitions. It handles loading TOML definition files, caching them in memory,
//! and providing query capabilities.
//!
//! # Architecture
//!
//! - **Definition Types** ([`definition`]): Strongly-typed broker metadata and configuration
//! - **Loader** ([`loader`]): TOML file loading from `broker-definitions/` directory
//! - **Registry** ([`registry`]): In-memory cache with query support
//! - **Errors** ([`error`]): Broker-specific error types
//!
//! # Example
//!
//! ```rust
//! use spectral_broker::{BrokerLoader, BrokerRegistry};
//! use spectral_core::BrokerId;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Load broker definitions from the default directory
//! let loader = BrokerLoader::with_default_dir()?;
//! let registry = BrokerRegistry::load_from(&loader)?;
//!
//! // Query a specific broker
//! let broker_id = BrokerId::new("spokeo")?;
//! let definition = registry.get(&broker_id)?;
//!
//! println!("Broker: {}", definition.name());
//! println!("Category: {:?}", definition.category());
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod definition;
pub mod error;
pub mod loader;
pub mod registry;

// Re-export commonly used types
pub use definition::{
    BrokerCategory, BrokerDefinition, BrokerMetadata, ConfirmationType, RemovalDifficulty,
    RemovalMethod, SearchMethod,
};
pub use error::{BrokerError, Result};
pub use loader::BrokerLoader;
pub use registry::BrokerRegistry;
