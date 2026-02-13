use spectral_broker::BrokerRegistry;
use spectral_browser::BrowserEngine;
use spectral_core::types::ProfileId;
use spectral_db::Database;
use spectral_scanner::{BrokerFilter, ScanOrchestrator};
use spectral_vault::{EncryptedField, UserProfile};
use std::sync::Arc;

#[tokio::test]
#[ignore = "Requires Chrome browser to be installed"]
async fn test_full_scan_flow() {
    // Setup
    let key = [0x42; 32];
    let db = Database::new(":memory:", key.to_vec())
        .await
        .expect("create db");
    db.run_migrations().await.expect("run migrations");

    let db = Arc::new(db);

    let broker_registry = Arc::new(BrokerRegistry::new());
    let browser_engine = Arc::new(BrowserEngine::new().await.expect("create browser"));

    let orchestrator = ScanOrchestrator::new(broker_registry, browser_engine, db.clone())
        .with_max_concurrent_scans(2);

    // Create test profile
    let mut profile = UserProfile::new(ProfileId::generate());
    profile.first_name = Some(EncryptedField::encrypt(&"John".to_string(), &key).unwrap());
    profile.last_name = Some(EncryptedField::encrypt(&"Doe".to_string(), &key).unwrap());
    profile.state = Some(EncryptedField::encrypt(&"CA".to_string(), &key).unwrap());

    // Start scan
    let job_id = orchestrator
        .start_scan(&profile, BrokerFilter::All, &key)
        .await
        .expect("start scan");

    // Verify job was created
    let job =
        sqlx::query_as::<_, (String, String)>("SELECT id, status FROM scan_jobs WHERE id = ?")
            .bind(&job_id)
            .fetch_one(db.pool())
            .await
            .expect("fetch job");

    assert_eq!(job.0, job_id);
    assert_eq!(job.1, "InProgress");

    // Wait briefly for background execution
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    println!("Integration test completed - scan job created: {}", job_id);
}
