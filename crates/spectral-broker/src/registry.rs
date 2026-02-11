//! In-memory broker definition registry with query support.

use crate::{
    definition::{BrokerCategory, BrokerDefinition, RemovalDifficulty},
    error::{BrokerError, Result},
    loader::BrokerLoader,
};
use spectral_core::BrokerId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, info};

/// In-memory cache of broker definitions with query capabilities.
///
/// The registry loads definitions from disk and caches them in memory
/// for fast lookups. It supports queries by ID, category, and difficulty.
#[derive(Clone)]
pub struct BrokerRegistry {
    /// Cached broker definitions, indexed by broker ID
    definitions: Arc<RwLock<HashMap<BrokerId, BrokerDefinition>>>,
}

impl BrokerRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            definitions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a registry and load all definitions from the given loader.
    ///
    /// # Errors
    /// Returns error if loading fails.
    pub fn load_from(loader: &BrokerLoader) -> Result<Self> {
        let registry = Self::new();
        registry.reload(loader)?;
        Ok(registry)
    }

    /// Reload all broker definitions from the loader.
    ///
    /// This replaces the current cache with freshly loaded definitions.
    ///
    /// # Errors
    /// Returns error if loading fails.
    pub fn reload(&self, loader: &BrokerLoader) -> Result<()> {
        let definitions = loader.load_all()?;

        let mut cache = self
            .definitions
            .write()
            .expect("acquire write lock on definitions");

        cache.clear();

        for definition in definitions {
            let broker_id = definition.id().clone();
            cache.insert(broker_id, definition);
        }

        info!(count = cache.len(), "reloaded broker definitions");

        Ok(())
    }

    /// Get a broker definition by ID.
    ///
    /// # Errors
    /// Returns error if the broker is not found.
    pub fn get(&self, broker_id: &BrokerId) -> Result<BrokerDefinition> {
        let cache = self
            .definitions
            .read()
            .expect("acquire read lock on definitions");

        cache
            .get(broker_id)
            .cloned()
            .ok_or_else(|| BrokerError::NotFound {
                broker_id: broker_id.to_string(),
            })
    }

    /// Get all broker definitions.
    #[must_use]
    pub fn get_all(&self) -> Vec<BrokerDefinition> {
        let cache = self
            .definitions
            .read()
            .expect("acquire read lock on definitions");

        cache.values().cloned().collect()
    }

    /// Query brokers by category.
    #[must_use]
    pub fn get_by_category(&self, category: BrokerCategory) -> Vec<BrokerDefinition> {
        let cache = self
            .definitions
            .read()
            .expect("acquire read lock on definitions");

        cache
            .values()
            .filter(|def| def.category() == category)
            .cloned()
            .collect()
    }

    /// Query brokers by difficulty level.
    #[must_use]
    pub fn get_by_difficulty(&self, difficulty: RemovalDifficulty) -> Vec<BrokerDefinition> {
        let cache = self
            .definitions
            .read()
            .expect("acquire read lock on definitions");

        cache
            .values()
            .filter(|def| def.broker.difficulty == difficulty)
            .cloned()
            .collect()
    }

    /// Query brokers by category and difficulty.
    #[must_use]
    pub fn get_by_category_and_difficulty(
        &self,
        category: BrokerCategory,
        difficulty: RemovalDifficulty,
    ) -> Vec<BrokerDefinition> {
        let cache = self
            .definitions
            .read()
            .expect("acquire read lock on definitions");

        cache
            .values()
            .filter(|def| def.category() == category && def.broker.difficulty == difficulty)
            .cloned()
            .collect()
    }

    /// Get the total number of brokers in the registry.
    #[must_use]
    pub fn count(&self) -> usize {
        let cache = self
            .definitions
            .read()
            .expect("acquire read lock on definitions");

        cache.len()
    }

    /// Check if a broker exists in the registry.
    #[must_use]
    pub fn contains(&self, broker_id: &BrokerId) -> bool {
        let cache = self
            .definitions
            .read()
            .expect("acquire read lock on definitions");

        cache.contains_key(broker_id)
    }

    /// Get all broker IDs in the registry.
    #[must_use]
    pub fn get_all_ids(&self) -> Vec<BrokerId> {
        let cache = self
            .definitions
            .read()
            .expect("acquire read lock on definitions");

        cache.keys().cloned().collect()
    }

    /// Get broker count by category.
    #[must_use]
    pub fn count_by_category(&self) -> HashMap<BrokerCategory, usize> {
        let cache = self
            .definitions
            .read()
            .expect("acquire read lock on definitions");

        let mut counts: HashMap<BrokerCategory, usize> = HashMap::new();

        for definition in cache.values() {
            *counts.entry(definition.category()).or_insert(0) += 1;
        }

        counts
    }

    /// Add or update a broker definition in the registry.
    ///
    /// This is useful for testing or dynamic updates.
    pub fn insert(&self, definition: BrokerDefinition) -> Result<()> {
        // Validate before inserting
        definition.validate()?;

        let mut cache = self
            .definitions
            .write()
            .expect("acquire write lock on definitions");

        let broker_id = definition.id().clone();
        cache.insert(broker_id.clone(), definition);

        debug!(broker_id = %broker_id, "inserted broker definition");

        Ok(())
    }

    /// Remove a broker definition from the registry.
    ///
    /// Returns `true` if the broker was present, `false` otherwise.
    pub fn remove(&self, broker_id: &BrokerId) -> bool {
        let mut cache = self
            .definitions
            .write()
            .expect("acquire write lock on definitions");

        let removed = cache.remove(broker_id).is_some();

        if removed {
            debug!(broker_id = %broker_id, "removed broker definition");
        }

        removed
    }
}

impl Default for BrokerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::{
        BrokerMetadata, ConfirmationType, FormSelectors, RemovalMethod, SearchMethod,
    };
    use chrono::NaiveDate;
    use spectral_core::PiiField;
    use std::collections::HashMap;

    fn create_test_definition(
        id: &str,
        category: BrokerCategory,
        difficulty: RemovalDifficulty,
    ) -> BrokerDefinition {
        let mut fields = HashMap::new();
        fields.insert("email".to_string(), "{user_email}".to_string());

        BrokerDefinition {
            broker: BrokerMetadata {
                id: BrokerId::new(id).expect("valid broker ID"),
                name: format!("Test {id}"),
                url: "https://test.com".to_string(),
                domain: "test.com".to_string(),
                category,
                difficulty,
                typical_removal_days: 7,
                recheck_interval_days: 30,
                last_verified: NaiveDate::from_ymd_opt(2025, 5, 1).expect("valid date"),
            },
            search: SearchMethod::UrlTemplate {
                template: "https://test.com/{first}-{last}".to_string(),
                requires_fields: vec![PiiField::FirstName, PiiField::LastName],
            },
            removal: RemovalMethod::WebForm {
                url: "https://test.com/optout".to_string(),
                fields,
                form_selectors: FormSelectors {
                    listing_url_input: Some("#listing-url".to_string()),
                    email_input: Some("input[name='email']".to_string()),
                    first_name_input: None,
                    last_name_input: None,
                    submit_button: "button[type='submit']".to_string(),
                    captcha_frame: None,
                    success_indicator: Some(".success".to_string()),
                },
                confirmation: ConfirmationType::EmailVerification,
                notes: String::new(),
            },
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = BrokerRegistry::new();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_registry_insert_and_get() {
        let registry = BrokerRegistry::new();
        let definition = create_test_definition(
            "test-broker",
            BrokerCategory::PeopleSearch,
            RemovalDifficulty::Easy,
        );
        let broker_id = definition.id().clone();

        registry.insert(definition).expect("insert definition");

        let retrieved = registry.get(&broker_id).expect("get definition");
        assert_eq!(retrieved.id(), &broker_id);
        assert_eq!(retrieved.name(), "Test test-broker");
    }

    #[test]
    fn test_registry_get_nonexistent() {
        let registry = BrokerRegistry::new();
        let broker_id = BrokerId::new("nonexistent").expect("valid broker ID");

        let result = registry.get(&broker_id);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BrokerError::NotFound { .. }));
    }

    #[test]
    fn test_registry_contains() {
        let registry = BrokerRegistry::new();
        let definition = create_test_definition(
            "test-broker",
            BrokerCategory::PeopleSearch,
            RemovalDifficulty::Easy,
        );
        let broker_id = definition.id().clone();

        assert!(!registry.contains(&broker_id));

        registry.insert(definition).expect("insert definition");

        assert!(registry.contains(&broker_id));
    }

    #[test]
    fn test_registry_remove() {
        let registry = BrokerRegistry::new();
        let definition = create_test_definition(
            "test-broker",
            BrokerCategory::PeopleSearch,
            RemovalDifficulty::Easy,
        );
        let broker_id = definition.id().clone();

        registry.insert(definition).expect("insert definition");
        assert!(registry.contains(&broker_id));

        let removed = registry.remove(&broker_id);
        assert!(removed);
        assert!(!registry.contains(&broker_id));

        // Removing again should return false
        let removed = registry.remove(&broker_id);
        assert!(!removed);
    }

    #[test]
    fn test_registry_get_all() {
        let registry = BrokerRegistry::new();

        registry
            .insert(create_test_definition(
                "broker-1",
                BrokerCategory::PeopleSearch,
                RemovalDifficulty::Easy,
            ))
            .expect("insert broker 1");

        registry
            .insert(create_test_definition(
                "broker-2",
                BrokerCategory::BackgroundCheck,
                RemovalDifficulty::Medium,
            ))
            .expect("insert broker 2");

        registry
            .insert(create_test_definition(
                "broker-3",
                BrokerCategory::PeopleSearch,
                RemovalDifficulty::Hard,
            ))
            .expect("insert broker 3");

        let all = registry.get_all();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_registry_get_by_category() {
        let registry = BrokerRegistry::new();

        registry
            .insert(create_test_definition(
                "broker-1",
                BrokerCategory::PeopleSearch,
                RemovalDifficulty::Easy,
            ))
            .expect("insert broker 1");

        registry
            .insert(create_test_definition(
                "broker-2",
                BrokerCategory::BackgroundCheck,
                RemovalDifficulty::Medium,
            ))
            .expect("insert broker 2");

        registry
            .insert(create_test_definition(
                "broker-3",
                BrokerCategory::PeopleSearch,
                RemovalDifficulty::Hard,
            ))
            .expect("insert broker 3");

        let people_search = registry.get_by_category(BrokerCategory::PeopleSearch);
        assert_eq!(people_search.len(), 2);

        let background_check = registry.get_by_category(BrokerCategory::BackgroundCheck);
        assert_eq!(background_check.len(), 1);
    }

    #[test]
    fn test_registry_get_by_difficulty() {
        let registry = BrokerRegistry::new();

        registry
            .insert(create_test_definition(
                "broker-1",
                BrokerCategory::PeopleSearch,
                RemovalDifficulty::Easy,
            ))
            .expect("insert broker 1");

        registry
            .insert(create_test_definition(
                "broker-2",
                BrokerCategory::BackgroundCheck,
                RemovalDifficulty::Easy,
            ))
            .expect("insert broker 2");

        registry
            .insert(create_test_definition(
                "broker-3",
                BrokerCategory::PeopleSearch,
                RemovalDifficulty::Hard,
            ))
            .expect("insert broker 3");

        let easy = registry.get_by_difficulty(RemovalDifficulty::Easy);
        assert_eq!(easy.len(), 2);

        let hard = registry.get_by_difficulty(RemovalDifficulty::Hard);
        assert_eq!(hard.len(), 1);

        let medium = registry.get_by_difficulty(RemovalDifficulty::Medium);
        assert_eq!(medium.len(), 0);
    }

    #[test]
    fn test_registry_get_by_category_and_difficulty() {
        let registry = BrokerRegistry::new();

        registry
            .insert(create_test_definition(
                "broker-1",
                BrokerCategory::PeopleSearch,
                RemovalDifficulty::Easy,
            ))
            .expect("insert broker 1");

        registry
            .insert(create_test_definition(
                "broker-2",
                BrokerCategory::PeopleSearch,
                RemovalDifficulty::Hard,
            ))
            .expect("insert broker 2");

        registry
            .insert(create_test_definition(
                "broker-3",
                BrokerCategory::BackgroundCheck,
                RemovalDifficulty::Easy,
            ))
            .expect("insert broker 3");

        let results = registry
            .get_by_category_and_difficulty(BrokerCategory::PeopleSearch, RemovalDifficulty::Easy);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id().as_str(), "broker-1");
    }

    #[test]
    fn test_registry_count_by_category() {
        let registry = BrokerRegistry::new();

        registry
            .insert(create_test_definition(
                "broker-1",
                BrokerCategory::PeopleSearch,
                RemovalDifficulty::Easy,
            ))
            .expect("insert broker 1");

        registry
            .insert(create_test_definition(
                "broker-2",
                BrokerCategory::PeopleSearch,
                RemovalDifficulty::Hard,
            ))
            .expect("insert broker 2");

        registry
            .insert(create_test_definition(
                "broker-3",
                BrokerCategory::BackgroundCheck,
                RemovalDifficulty::Easy,
            ))
            .expect("insert broker 3");

        let counts = registry.count_by_category();
        assert_eq!(counts.get(&BrokerCategory::PeopleSearch), Some(&2));
        assert_eq!(counts.get(&BrokerCategory::BackgroundCheck), Some(&1));
        assert_eq!(counts.get(&BrokerCategory::Marketing), None);
    }

    #[test]
    fn test_registry_get_all_ids() {
        let registry = BrokerRegistry::new();

        registry
            .insert(create_test_definition(
                "broker-1",
                BrokerCategory::PeopleSearch,
                RemovalDifficulty::Easy,
            ))
            .expect("insert broker 1");

        registry
            .insert(create_test_definition(
                "broker-2",
                BrokerCategory::BackgroundCheck,
                RemovalDifficulty::Medium,
            ))
            .expect("insert broker 2");

        let ids = registry.get_all_ids();
        assert_eq!(ids.len(), 2);

        let id_strings: Vec<String> = ids.iter().map(ToString::to_string).collect();
        assert!(id_strings.contains(&"broker-1".to_string()));
        assert!(id_strings.contains(&"broker-2".to_string()));
    }
}
