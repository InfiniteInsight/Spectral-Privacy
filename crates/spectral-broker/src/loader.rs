//! Broker definition loading from TOML files.
//!
//! This module handles loading broker definitions from the `broker-definitions/` directory.

use crate::{
    definition::BrokerDefinition,
    error::{BrokerError, Result},
};
use spectral_core::BrokerId;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Loader for broker definitions from TOML files.
pub struct BrokerLoader {
    /// Base directory containing broker definitions
    definitions_dir: PathBuf,
}

impl BrokerLoader {
    /// Create a new loader with the given definitions directory.
    ///
    /// # Errors
    /// Returns error if the directory doesn't exist.
    pub fn new(definitions_dir: impl Into<PathBuf>) -> Result<Self> {
        let definitions_dir = definitions_dir.into();

        if !definitions_dir.exists() {
            return Err(BrokerError::DirectoryNotFound {
                path: definitions_dir.display().to_string(),
            });
        }

        if !definitions_dir.is_dir() {
            return Err(BrokerError::DirectoryNotFound {
                path: definitions_dir.display().to_string(),
            });
        }

        Ok(Self { definitions_dir })
    }

    /// Create a loader using the default definitions directory.
    ///
    /// Looks for `broker-definitions/` relative to the workspace root.
    ///
    /// # Errors
    /// Returns error if the default directory doesn't exist.
    pub fn with_default_dir() -> Result<Self> {
        // Find workspace root by looking for Cargo.toml with [workspace]
        let mut current_dir = std::env::current_dir()?;

        loop {
            let cargo_toml = current_dir.join("Cargo.toml");
            if cargo_toml.exists() {
                // Check if it's a workspace
                if let Ok(contents) = std::fs::read_to_string(&cargo_toml) {
                    if contents.contains("[workspace]") {
                        let definitions_dir = current_dir.join("broker-definitions");
                        return Self::new(definitions_dir);
                    }
                }
            }

            // Move up one directory
            if let Some(parent) = current_dir.parent() {
                current_dir = parent.to_path_buf();
            } else {
                break;
            }
        }

        // Fallback: try relative path
        let definitions_dir = PathBuf::from("broker-definitions");
        Self::new(definitions_dir)
    }

    /// Load a single broker definition by ID.
    ///
    /// # Errors
    /// Returns error if the definition file doesn't exist, can't be read, or is invalid.
    pub fn load(&self, broker_id: &BrokerId) -> Result<BrokerDefinition> {
        let definition = self.find_and_load(broker_id)?;

        // Validate after loading
        definition.validate()?;

        debug!(
            broker_id = %broker_id,
            name = %definition.name(),
            "loaded broker definition"
        );

        Ok(definition)
    }

    /// Load all broker definitions from the definitions directory.
    ///
    /// Invalid definitions are logged as warnings and skipped.
    ///
    /// # Errors
    /// Returns error if the directory can't be read.
    pub fn load_all(&self) -> Result<Vec<BrokerDefinition>> {
        let mut definitions = Vec::new();

        Self::walk_and_load_recursive(&self.definitions_dir, &mut definitions)?;

        info!(
            count = definitions.len(),
            dir = %self.definitions_dir.display(),
            "loaded broker definitions"
        );

        Ok(definitions)
    }

    /// Recursively walk directory and load all TOML files.
    fn walk_and_load_recursive(dir: &Path, definitions: &mut Vec<BrokerDefinition>) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Recursively process subdirectories
                Self::walk_and_load_recursive(&path, definitions)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                // Skip README files
                if path.file_name().and_then(|s| s.to_str()) == Some("README.toml") {
                    continue;
                }

                // Load and parse TOML
                match Self::load_from_path(&path) {
                    Ok(definition) => {
                        // Validate before adding
                        if let Err(e) = definition.validate() {
                            warn!(
                                path = %path.display(),
                                error = %e,
                                "skipping invalid broker definition"
                            );
                            continue;
                        }
                        definitions.push(definition);
                    }
                    Err(e) => {
                        warn!(
                            path = %path.display(),
                            error = %e,
                            "failed to load broker definition"
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Find and load a broker definition file by ID.
    fn find_and_load(&self, broker_id: &BrokerId) -> Result<BrokerDefinition> {
        // Try to find the TOML file in any subdirectory
        let filename = format!("{}.toml", broker_id.as_str());

        if let Some(path) = Self::find_file(&self.definitions_dir, &filename)? {
            Self::load_from_path(&path)
        } else {
            Err(BrokerError::NotFound {
                broker_id: broker_id.to_string(),
            })
        }
    }

    /// Recursively search for a file by name.
    fn find_file(dir: &Path, filename: &str) -> Result<Option<PathBuf>> {
        Self::find_file_recursive(dir, filename)
    }

    /// Recursively search for a file by name (static helper).
    fn find_file_recursive(dir: &Path, filename: &str) -> Result<Option<PathBuf>> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                if let Some(found) = Self::find_file_recursive(&path, filename)? {
                    return Ok(Some(found));
                }
            } else if path.file_name().and_then(|s| s.to_str()) == Some(filename) {
                return Ok(Some(path));
            }
        }

        Ok(None)
    }

    /// Load a broker definition from a specific file path.
    fn load_from_path(path: &Path) -> Result<BrokerDefinition> {
        let contents = std::fs::read_to_string(path).map_err(|e| BrokerError::LoadError {
            path: path.display().to_string(),
            source: Box::new(e),
        })?;

        toml::from_str(&contents).map_err(|e| BrokerError::ParseError {
            path: path.display().to_string(),
            source: e,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::BrokerCategory;
    use tempfile::TempDir;

    fn create_test_definition_file(dir: &Path, broker_id: &str, category: &str) -> PathBuf {
        let category_dir = dir.join(category);
        std::fs::create_dir_all(&category_dir).expect("create category dir");

        let file_path = category_dir.join(format!("{broker_id}.toml"));

        let content = format!(
            r#"
[broker]
id = "{broker_id}"
name = "Test Broker"
url = "https://test.com"
domain = "test.com"
category = "people-search"
difficulty = "Easy"
typical_removal_days = 7
recheck_interval_days = 30
last_verified = "2025-05-01"

[search]
method = "url-template"
template = "https://test.com/{{first}}-{{last}}"
requires_fields = ["first_name", "last_name"]

[removal]
method = "web-form"
url = "https://test.com/optout"
confirmation = "email-verification"
notes = "Test notes"

[removal.fields]
email = "{{user_email}}"
"#
        );

        std::fs::write(&file_path, content).expect("write test file");
        file_path
    }

    #[test]
    fn test_loader_new_with_existing_dir() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let loader = BrokerLoader::new(temp_dir.path());
        assert!(loader.is_ok());
    }

    #[test]
    fn test_loader_new_with_nonexistent_dir() {
        let loader = BrokerLoader::new("/nonexistent/path/to/definitions");
        assert!(loader.is_err());
    }

    #[test]
    fn test_load_single_broker() {
        let temp_dir = TempDir::new().expect("create temp dir");
        create_test_definition_file(temp_dir.path(), "test-broker", "people-search");

        let loader = BrokerLoader::new(temp_dir.path()).expect("create loader");
        let broker_id = BrokerId::new("test-broker").expect("valid broker ID");
        let definition = loader.load(&broker_id).expect("load broker definition");

        assert_eq!(definition.id(), &broker_id);
        assert_eq!(definition.name(), "Test Broker");
        assert_eq!(definition.category(), BrokerCategory::PeopleSearch);
    }

    #[test]
    fn test_load_nonexistent_broker() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let loader = BrokerLoader::new(temp_dir.path()).expect("create loader");
        let broker_id = BrokerId::new("nonexistent").expect("valid broker ID");

        let result = loader.load(&broker_id);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BrokerError::NotFound { .. }));
    }

    #[test]
    fn test_load_all_brokers() {
        let temp_dir = TempDir::new().expect("create temp dir");

        // Create multiple broker definitions
        create_test_definition_file(temp_dir.path(), "broker-1", "people-search");
        create_test_definition_file(temp_dir.path(), "broker-2", "people-search");
        create_test_definition_file(temp_dir.path(), "broker-3", "background-check");

        let loader = BrokerLoader::new(temp_dir.path()).expect("create loader");
        let definitions = loader.load_all().expect("load all definitions");

        assert_eq!(definitions.len(), 3);

        // Verify all broker IDs are unique
        let ids: std::collections::HashSet<_> =
            definitions.iter().map(BrokerDefinition::id).collect();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn test_load_all_skips_invalid() {
        let temp_dir = TempDir::new().expect("create temp dir");

        // Create a valid definition
        create_test_definition_file(temp_dir.path(), "valid-broker", "people-search");

        // Create an invalid TOML file
        let invalid_path = temp_dir.path().join("invalid.toml");
        std::fs::write(&invalid_path, "invalid toml content [[[").expect("write invalid file");

        let loader = BrokerLoader::new(temp_dir.path()).expect("create loader");
        let definitions = loader.load_all().expect("load all definitions");

        // Should only load the valid one
        assert_eq!(definitions.len(), 1);
    }

    #[test]
    fn test_find_file_in_nested_directories() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let nested_dir = temp_dir.path().join("category").join("subcategory");
        std::fs::create_dir_all(&nested_dir).expect("create nested dir");

        let file_path = nested_dir.join("nested-broker.toml");
        std::fs::write(&file_path, "test").expect("write file");

        BrokerLoader::new(temp_dir.path()).expect("create loader");
        let found = BrokerLoader::find_file(temp_dir.path(), "nested-broker.toml")
            .expect("search for file");

        assert!(found.is_some());
        assert_eq!(found.unwrap(), file_path);
    }
}
