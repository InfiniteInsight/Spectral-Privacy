use crate::error::Result;
use crate::filter::BrokerFilter;
use spectral_broker::BrokerRegistry;
use spectral_browser::BrowserEngine;
use spectral_db::scan_jobs;
use spectral_db::EncryptedPool;
use spectral_vault::UserProfile;
use std::sync::Arc;

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral_core::ProfileId;
    use spectral_db::Database;
    use spectral_vault::{encrypt_string, UserProfile};

    // Helper to create test orchestrator
    // Note: Skips browser creation since Chrome may not be available in test environment
    async fn create_test_pool_and_db() -> (Arc<EncryptedPool>, Database, [u8; 32]) {
        let key = [0x42; 32];
        let pool = Arc::new(
            EncryptedPool::new(":memory:", key.to_vec())
                .await
                .expect("create pool"),
        );
        let db = Database::new(":memory:", key.to_vec())
            .await
            .expect("create db");
        db.run_migrations().await.expect("run migrations");
        (pool, db, key)
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
        let (pool, db, key) = create_test_pool_and_db().await;

        let broker_registry = Arc::new(BrokerRegistry::new());
        let browser_engine = Arc::new(BrowserEngine::new().await.expect("create browser"));

        let orchestrator = ScanOrchestrator::new(broker_registry, browser_engine, pool, 5);

        let profile = mock_profile(&key);

        let job_id = orchestrator
            .start_scan(&profile, BrokerFilter::All, &key)
            .await
            .expect("start scan");

        // Verify job was created in database
        let job = sqlx::query_as::<_, (String, String, i64)>(
            "SELECT id, status, total_brokers FROM scan_jobs WHERE id = ?",
        )
        .bind(&job_id)
        .fetch_one(db.pool())
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
}
