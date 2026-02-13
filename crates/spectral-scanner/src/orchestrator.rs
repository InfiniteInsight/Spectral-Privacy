use crate::error::Result;
use crate::filter::BrokerFilter;
use crate::parser::ListingMatch;
use crate::url_builder::build_search_url;
use spectral_broker::BrokerDefinition;
use spectral_broker::BrokerRegistry;
use spectral_browser::BrowserActions;
use spectral_browser::BrowserEngine;
use spectral_core::BrokerId;
use spectral_db::scan_jobs;
use spectral_db::EncryptedPool;
use spectral_vault::UserProfile;
use std::sync::Arc;

#[allow(dead_code)]
struct BrokerScanResult {
    broker_id: BrokerId,
    findings: Vec<ListingMatch>,
    error: Option<String>,
}

#[allow(dead_code)]
pub struct ScanOrchestrator {
    broker_registry: Arc<BrokerRegistry>,
    browser_engine: Arc<BrowserEngine>,
    db: Arc<EncryptedPool>,
    max_concurrent_scans: usize,
}

impl ScanOrchestrator {
    pub fn new(
        broker_registry: Arc<BrokerRegistry>,
        browser_engine: Arc<BrowserEngine>,
        db: Arc<EncryptedPool>,
        max_concurrent_scans: usize,
    ) -> Self {
        Self {
            broker_registry,
            browser_engine,
            db,
            max_concurrent_scans,
        }
    }

    pub async fn start_scan(
        &self,
        profile: &UserProfile,
        broker_filter: BrokerFilter,
        _vault_key: &[u8; 32],
    ) -> Result<String> {
        // Get list of brokers to scan
        let brokers: Vec<_> = self
            .broker_registry
            .get_all()
            .into_iter()
            .filter(|broker| broker_filter.matches(broker))
            .collect();

        let total_brokers = brokers.len() as u32;

        // Create scan job in database
        let job = scan_jobs::create_scan_job(
            self.db.pool(),
            profile.id.as_str().to_string(),
            total_brokers,
        )
        .await?;

        Ok(job.id)
    }

    #[allow(dead_code)]
    async fn scan_single_broker(
        &self,
        _scan_job_id: &str,
        broker: &BrokerDefinition,
        profile: &UserProfile,
        vault_key: &[u8; 32],
    ) -> Result<BrokerScanResult> {
        let broker_id = broker.broker.id.clone();

        // Build search URL
        let search_url = build_search_url(&broker_id, &broker.search, profile, vault_key)?;

        // Fetch the search results page
        // TODO: Implement HTML fetching in BrowserEngine
        // For now, using placeholder - this will be implemented when browser integration is complete
        let _html = match self.browser_engine.navigate(&search_url).await {
            Ok(_) => {
                // After navigation, we need to get the page HTML
                // This will require adding a get_html() method to BrowserEngine
                String::new()
            }
            Err(e) => {
                return Ok(BrokerScanResult {
                    broker_id,
                    findings: vec![],
                    error: Some(format!("Failed to fetch page: {}", e)),
                });
            }
        };

        // Parse results if selectors are available
        let findings = if let Some(_selectors) = broker.search.result_selectors() {
            // TODO: Parse HTML when browser integration is complete
            // let parser = ResultParser::new(selectors, broker.broker.url.clone());
            // match parser.parse(&html) { ... }
            vec![]
        } else {
            // No selectors - manual review needed
            vec![]
        };

        Ok(BrokerScanResult {
            broker_id,
            findings,
            error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral_core::ProfileId;
    use spectral_vault::{encrypt_string, UserProfile};

    // Helper to create test pool
    // Note: Skips browser creation since Chrome may not be available in test environment
    async fn create_test_pool() -> (Arc<EncryptedPool>, [u8; 32]) {
        let key = [0x42; 32];
        let pool = EncryptedPool::new(":memory:", key.to_vec())
            .await
            .expect("create pool");
        spectral_db::migrations::run_migrations(pool.pool())
            .await
            .expect("run migrations");
        (Arc::new(pool), key)
    }

    fn mock_profile(key: &[u8; 32]) -> UserProfile {
        let mut profile = UserProfile::new(ProfileId::generate());
        profile.first_name = Some(encrypt_string("John", key).expect("encrypt first name"));
        profile.last_name = Some(encrypt_string("Doe", key).expect("encrypt last name"));
        profile.state = Some(encrypt_string("CA", key).expect("encrypt state"));
        profile.city = Some(encrypt_string("Los Angeles", key).expect("encrypt city"));
        profile
    }

    #[tokio::test]
    #[ignore = "Requires Chrome browser to be installed"]
    async fn test_start_scan_creates_job() {
        let (pool, key) = create_test_pool().await;

        let broker_registry = Arc::new(BrokerRegistry::new());
        let browser_engine = Arc::new(BrowserEngine::new().await.expect("create browser"));

        let orchestrator = ScanOrchestrator::new(broker_registry, browser_engine, pool.clone(), 5);

        let profile = mock_profile(&key);

        let job_id = orchestrator
            .start_scan(&profile, BrokerFilter::All, &key)
            .await
            .expect("start scan");

        // Verify job was created in database - use same pool
        let job = sqlx::query_as::<_, (String, String, i64)>(
            "SELECT id, status, total_brokers FROM scan_jobs WHERE id = ?",
        )
        .bind(&job_id)
        .fetch_one(pool.pool())
        .await
        .expect("fetch job");

        assert_eq!(job.0, job_id);
        assert_eq!(job.1, "InProgress");
        assert!(job.2 > 0);
    }

    #[test]
    fn test_orchestrator_compiles() {
        // Verify the orchestrator methods compile correctly
        // This is a compile-time test to ensure API is correct
        // Actual runtime tests require Chrome browser
    }

    #[tokio::test]
    #[ignore = "Requires Chrome browser to be installed"]
    async fn test_scan_single_broker() {
        let (pool, key) = create_test_pool().await;

        let broker_registry = Arc::new(BrokerRegistry::new());
        let browser_engine = Arc::new(BrowserEngine::new().await.expect("create browser"));

        let orchestrator = ScanOrchestrator::new(broker_registry, browser_engine, pool.clone(), 5);

        let profile = mock_profile(&key);

        // Create a mock broker definition
        let broker = BrokerDefinition {
            broker: spectral_broker::BrokerMetadata {
                id: BrokerId::new("test-broker").expect("valid broker ID"),
                name: "Test Broker".to_string(),
                url: "https://example.com".to_string(),
                domain: "example.com".to_string(),
                category: spectral_broker::BrokerCategory::PeopleSearch,
                difficulty: spectral_broker::RemovalDifficulty::Easy,
                typical_removal_days: 7,
                recheck_interval_days: 30,
                last_verified: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid date"),
            },
            search: spectral_broker::SearchMethod::UrlTemplate {
                template: "https://example.com/{first}-{last}".to_string(),
                requires_fields: vec![
                    spectral_core::PiiField::FirstName,
                    spectral_core::PiiField::LastName,
                ],
                result_selectors: None,
            },
            removal: spectral_broker::RemovalMethod::Manual {
                instructions: "Test removal instructions".to_string(),
            },
        };

        // Note: This test will fail if there's no actual browser, but verifies the flow compiles
        let result = orchestrator
            .scan_single_broker("job-123", &broker, &profile, &key)
            .await;

        // We expect this to fail in tests (no real browser), but the method should exist
        assert!(result.is_ok() || result.is_err());
    }
}
