//! Integration tests for removal batch processing
//!
//! Tests the process_removal_batch command and queue query commands
//! to validate task spawning, database state, and queue filtering logic.

use spectral_app::commands::scan::{get_captcha_queue, get_failed_queue};
use spectral_app::state::AppState;
use spectral_db::findings::create_finding;
use spectral_db::removal_attempts::{
    create_removal_attempt, get_by_id, update_status, RemovalStatus,
};
use spectral_vault::Vault;
use std::sync::Arc;
use tauri::{Manager, State};
use tempfile::TempDir;
use uuid::Uuid;

/// Helper to create test app with AppState and temporary directory.
fn create_test_app() -> (tauri::App<tauri::test::MockRuntime>, TempDir) {
    let temp_dir = TempDir::new().expect("create temp dir");
    let vaults_dir = temp_dir.path().join("vaults");
    std::fs::create_dir_all(&vaults_dir).expect("create vaults dir");

    let app_state = AppState {
        vaults_dir,
        unlocked_vaults: std::sync::RwLock::new(std::collections::HashMap::new()),
        browser_engine: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
        broker_registry: std::sync::Arc::new(spectral_broker::BrokerRegistry::new()),
    };

    let app = tauri::test::mock_app();
    app.manage(app_state);

    (app, temp_dir)
}

/// Helper to create and unlock a vault for testing.
async fn create_test_vault(state: &AppState, vault_id: &str) -> String {
    let vault_dir = state.vaults_dir.join(vault_id);
    std::fs::create_dir_all(&vault_dir).expect("create vault directory");
    let vault_path = vault_dir.join("vault.db");
    let password = "test-password-123"; // pragma: allowlist secret

    // Create vault
    let vault = Vault::create(password, &vault_path)
        .await
        .expect("create vault");

    // Store in state (wrapped in Arc)
    state
        .unlocked_vaults
        .write()
        .unwrap()
        .insert(vault_id.to_string(), Arc::new(vault));

    vault_id.to_string()
}

/// Helper to create test data: profile, scan job, broker scan, findings, removal attempts.
async fn setup_test_removal_structure(
    vault: &Vault,
    profile_id: &str,
    scan_job_id: &str,
    broker_scan_id: &str,
    num_attempts: usize,
) -> Vec<String> {
    let db = vault.database().expect("get database");
    let pool = db.pool();

    // Create test profile
    let dummy_data = [0u8; 32];
    let dummy_nonce = [0u8; 12];
    sqlx::query(
        "INSERT INTO profiles (id, data, nonce, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(profile_id)
    .bind(&dummy_data[..])
    .bind(&dummy_nonce[..])
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(pool)
    .await
    .expect("create profile");

    // Create test scan job
    sqlx::query(
        "INSERT INTO scan_jobs (id, profile_id, started_at, status, total_brokers, completed_brokers) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(scan_job_id)
    .bind(profile_id)
    .bind(chrono::Utc::now().to_rfc3339())
    .bind("Completed")
    .bind(1)
    .bind(1)
    .execute(pool)
    .await
    .expect("create scan job");

    // Create test broker scan
    sqlx::query(
        "INSERT INTO broker_scans (id, scan_job_id, broker_id, status, started_at) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(broker_scan_id)
    .bind(scan_job_id)
    .bind("test-broker")
    .bind("Success")
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(pool)
    .await
    .expect("create broker scan");

    // Create findings and removal attempts
    let mut removal_attempt_ids = Vec::new();
    for i in 0..num_attempts {
        // Create finding
        let finding = create_finding(
            pool,
            broker_scan_id.to_string(),
            "test-broker".to_string(),
            profile_id.to_string(),
            format!("https://broker.example.com/person/{}", i),
            serde_json::json!({"name": "Test User"}),
        )
        .await
        .expect("create finding");

        // Create removal attempt for the finding
        let removal_attempt =
            create_removal_attempt(pool, finding.id.clone(), "test-broker".to_string())
                .await
                .expect("create removal attempt");

        removal_attempt_ids.push(removal_attempt.id);
    }

    removal_attempt_ids
}

#[tokio::test]
async fn test_batch_processing_creates_worker_tasks() {
    // Setup
    let (app, _temp_dir) = create_test_app();
    let state: State<AppState> = app.state();
    let vault_id = Uuid::new_v4().to_string();
    let profile_id = "profile-123";
    let scan_job_id = "scan-job-456";
    let broker_scan_id = "broker-scan-789";

    // Create and unlock vault
    create_test_vault(&state, &vault_id).await;
    let vault = state.get_vault(&vault_id).expect("get vault");

    // Create 3 removal attempts
    let removal_attempt_ids =
        setup_test_removal_structure(&vault, profile_id, scan_job_id, broker_scan_id, 3).await;

    // Verify removal attempts exist in database
    let db = vault.database().expect("get database");
    for removal_id in &removal_attempt_ids {
        let attempt = get_by_id(db.pool(), removal_id)
            .await
            .expect("Failed to get removal attempt")
            .expect("Removal attempt should exist");
        assert_eq!(attempt.id, *removal_id);
        assert_eq!(attempt.status, RemovalStatus::Pending);
    }

    // Process batch
    let result = spectral_app::commands::scan::process_removal_batch(
        state,
        app.handle().clone(),
        vault_id.to_string(),
        removal_attempt_ids.clone(),
    )
    .await;

    // Verify command succeeds
    assert!(result.is_ok());
    let batch_result = result.unwrap();
    assert_eq!(batch_result.total_count, 3);
    assert_eq!(batch_result.queued_count, 3);
    assert!(!batch_result.job_id.is_empty());

    // Wait for worker tasks to spawn
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_queue_queries_return_correct_attempts() {
    // Setup
    let (app, _temp_dir) = create_test_app();
    let state: State<AppState> = app.state();
    let vault_id = Uuid::new_v4().to_string();
    let profile_id = "profile-123";
    let scan_job_id = "scan-job-456";
    let broker_scan_id = "broker-scan-789";

    // Create and unlock vault
    create_test_vault(&state, &vault_id).await;
    let vault = state.get_vault(&vault_id).expect("get vault");

    // Create 3 removal attempts
    let removal_attempt_ids =
        setup_test_removal_structure(&vault, profile_id, scan_job_id, broker_scan_id, 3).await;

    // Update removal attempts to different statuses
    let db = vault.database().expect("get database");

    // Set first to Pending with CAPTCHA error message
    update_status(
        db.pool(),
        &removal_attempt_ids[0],
        RemovalStatus::Pending,
        None,
        None,
        Some("CAPTCHA_REQUIRED: reCAPTCHA v2 detected".to_string()),
    )
    .await
    .expect("Failed to update status");

    // Set second to Failed
    update_status(
        db.pool(),
        &removal_attempt_ids[1],
        RemovalStatus::Failed,
        None,
        None,
        Some("Form submission failed".to_string()),
    )
    .await
    .expect("Failed to update status");

    // Third remains Pending (no CAPTCHA error)

    // Query captcha queue
    let captcha_result = get_captcha_queue(state.clone(), vault_id.clone()).await;

    assert!(captcha_result.is_ok(), "Captcha queue query should succeed");
    let captcha_attempts = captcha_result.unwrap();
    assert_eq!(captcha_attempts.len(), 1, "Should have 1 captcha attempt");
    assert_eq!(
        captcha_attempts[0].id, removal_attempt_ids[0],
        "Should return the CAPTCHA attempt"
    );
    assert_eq!(captcha_attempts[0].status, RemovalStatus::Pending);
    assert!(captcha_attempts[0]
        .error_message
        .as_ref()
        .unwrap()
        .starts_with("CAPTCHA_REQUIRED"));

    // Query failed queue
    let failed_result = get_failed_queue(state.clone(), vault_id.clone()).await;

    assert!(failed_result.is_ok(), "Failed queue query should succeed");
    let failed_attempts = failed_result.unwrap();
    assert_eq!(failed_attempts.len(), 1, "Should have 1 failed attempt");
    assert_eq!(
        failed_attempts[0].id, removal_attempt_ids[1],
        "Should return the Failed attempt"
    );
    assert_eq!(failed_attempts[0].status, RemovalStatus::Failed);
}
