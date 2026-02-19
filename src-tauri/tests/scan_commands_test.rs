//! Integration tests for scan commands.

use spectral_app::commands::scan::*;
use spectral_app::state::AppState;
use spectral_db::findings::{create_finding, verify_finding};
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

/// Helper to create test data structure: profile, scan job, broker scan.
async fn create_test_scan_structure(
    vault: &Vault,
    profile_id: &str,
    scan_job_id: &str,
    broker_scan_id: &str,
) {
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
    .bind(3)
    .bind(3)
    .execute(pool)
    .await
    .expect("create scan job");

    // Create test broker scan
    sqlx::query(
        "INSERT INTO broker_scans (id, scan_job_id, broker_id, status, started_at) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(broker_scan_id)
    .bind(scan_job_id)
    .bind("spokeo")
    .bind("Success")
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(pool)
    .await
    .expect("create broker scan");
}

#[tokio::test]
async fn test_submit_removals_for_confirmed() {
    let (app, _temp_dir) = create_test_app();
    let state: State<AppState> = app.state();
    let vault_id = Uuid::new_v4().to_string();
    let profile_id = "profile-123";
    let scan_job_id = "scan-job-456";
    let broker_scan_id = "broker-scan-789";

    // Create and unlock vault
    create_test_vault(&state, &vault_id).await;

    // Get vault to create test data
    let vault = state.get_vault(&vault_id).expect("get vault");
    let db = vault.database().expect("get database");

    // Create test data structure
    create_test_scan_structure(&vault, profile_id, scan_job_id, broker_scan_id).await;

    // Create 3 findings with different verification statuses
    let finding1 = create_finding(
        db.pool(),
        broker_scan_id.to_string(),
        "spokeo".to_string(),
        profile_id.to_string(),
        "https://example.com/finding1".to_string(),
        serde_json::json!({"name": "John Doe"}),
    )
    .await
    .expect("create finding 1");

    let finding2 = create_finding(
        db.pool(),
        broker_scan_id.to_string(),
        "spokeo".to_string(),
        profile_id.to_string(),
        "https://example.com/finding2".to_string(),
        serde_json::json!({"name": "Jane Doe"}),
    )
    .await
    .expect("create finding 2");

    let finding3 = create_finding(
        db.pool(),
        broker_scan_id.to_string(),
        "spokeo".to_string(),
        profile_id.to_string(),
        "https://example.com/finding3".to_string(),
        serde_json::json!({"name": "Bob Smith"}),
    )
    .await
    .expect("create finding 3");

    // Verify finding 1 and 2 as Confirmed, finding 3 as Rejected
    verify_finding(db.pool(), &finding1.id, true, true)
        .await
        .expect("verify finding 1 as confirmed");
    verify_finding(db.pool(), &finding2.id, true, true)
        .await
        .expect("verify finding 2 as confirmed");
    verify_finding(db.pool(), &finding3.id, false, true)
        .await
        .expect("verify finding 3 as rejected");

    // Call submit_removals_for_confirmed
    let removal_ids =
        submit_removals_for_confirmed(state.clone(), vault_id.clone(), scan_job_id.to_string())
            .await
            .expect("submit removals");

    // Verify we got exactly 2 removal attempt IDs (not 3)
    assert_eq!(removal_ids.len(), 2);

    // Verify removal_attempts were created in database
    for removal_id in &removal_ids {
        let removal = spectral_db::removal_attempts::get_by_id(db.pool(), removal_id)
            .await
            .expect("query removal")
            .expect("removal exists");

        // Verify status is Pending
        assert_eq!(
            removal.status,
            spectral_db::removal_attempts::RemovalStatus::Pending
        );
    }

    // Verify finding linkage - confirmed findings should have removal_attempt_id set
    let findings = spectral_db::findings::get_by_scan_job(db.pool(), scan_job_id)
        .await
        .expect("get findings");

    let finding1_updated = findings
        .iter()
        .find(|f| f.id == finding1.id)
        .expect("find finding1");
    let finding2_updated = findings
        .iter()
        .find(|f| f.id == finding2.id)
        .expect("find finding2");
    let finding3_updated = findings
        .iter()
        .find(|f| f.id == finding3.id)
        .expect("find finding3");

    // Confirmed findings should have removal_attempt_id
    assert!(finding1_updated.removal_attempt_id.is_some());
    assert!(finding2_updated.removal_attempt_id.is_some());

    // Rejected finding should NOT have removal_attempt_id
    assert!(finding3_updated.removal_attempt_id.is_none());
}

#[tokio::test]
async fn test_submit_with_no_confirmed_findings() {
    let (app, _temp_dir) = create_test_app();
    let state: State<AppState> = app.state();
    let vault_id = Uuid::new_v4().to_string();
    let profile_id = "profile-123";
    let scan_job_id = "scan-job-456";
    let broker_scan_id = "broker-scan-789";

    // Create and unlock vault
    create_test_vault(&state, &vault_id).await;

    // Get vault to create test data
    let vault = state.get_vault(&vault_id).expect("get vault");
    let db = vault.database().expect("get database");

    // Create test data structure
    create_test_scan_structure(&vault, profile_id, scan_job_id, broker_scan_id).await;

    // Create findings - all rejected
    let finding1 = create_finding(
        db.pool(),
        broker_scan_id.to_string(),
        "spokeo".to_string(),
        profile_id.to_string(),
        "https://example.com/finding1".to_string(),
        serde_json::json!({"name": "Not Me"}),
    )
    .await
    .expect("create finding 1");

    verify_finding(db.pool(), &finding1.id, false, true)
        .await
        .expect("verify finding as rejected");

    // Call submit_removals_for_confirmed - should return empty vector
    let removal_ids =
        submit_removals_for_confirmed(state.clone(), vault_id.clone(), scan_job_id.to_string())
            .await
            .expect("submit removals");

    // Should return empty vector (not error)
    assert_eq!(removal_ids.len(), 0);
}

#[tokio::test]
async fn test_submit_for_invalid_vault() {
    let (app, _temp_dir) = create_test_app();
    let state: State<AppState> = app.state();
    let vault_id = "non-existent-vault";
    let scan_job_id = "scan-job-456";

    // Call submit_removals_for_confirmed with non-existent vault
    let result =
        submit_removals_for_confirmed(state.clone(), vault_id.to_string(), scan_job_id.to_string())
            .await;

    // Should return error
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, "Vault not found or locked");
}

#[tokio::test]
async fn test_verify_finding_linkage() {
    let (app, _temp_dir) = create_test_app();
    let state: State<AppState> = app.state();
    let vault_id = Uuid::new_v4().to_string();
    let profile_id = "profile-123";
    let scan_job_id = "scan-job-456";
    let broker_scan_id = "broker-scan-789";

    // Create and unlock vault
    create_test_vault(&state, &vault_id).await;

    // Get vault to create test data
    let vault = state.get_vault(&vault_id).expect("get vault");
    let db = vault.database().expect("get database");

    // Create test data structure
    create_test_scan_structure(&vault, profile_id, scan_job_id, broker_scan_id).await;

    // Create confirmed finding
    let finding = create_finding(
        db.pool(),
        broker_scan_id.to_string(),
        "spokeo".to_string(),
        profile_id.to_string(),
        "https://example.com/finding".to_string(),
        serde_json::json!({"name": "Test User"}),
    )
    .await
    .expect("create finding");

    verify_finding(db.pool(), &finding.id, true, true)
        .await
        .expect("verify finding as confirmed");

    // Call submit_removals_for_confirmed
    let removal_ids =
        submit_removals_for_confirmed(state.clone(), vault_id.clone(), scan_job_id.to_string())
            .await
            .expect("submit removals");

    assert_eq!(removal_ids.len(), 1);

    // Query finding from database
    let findings = spectral_db::findings::get_by_scan_job(db.pool(), scan_job_id)
        .await
        .expect("get findings");

    assert_eq!(findings.len(), 1);
    let updated_finding = &findings[0];

    // Verify finding.removal_attempt_id is set and matches
    assert!(updated_finding.removal_attempt_id.is_some());
    assert_eq!(
        updated_finding.removal_attempt_id.as_ref().unwrap(),
        &removal_ids[0]
    );
}

#[tokio::test]
async fn test_multiple_scans_isolation() {
    let (app, _temp_dir) = create_test_app();
    let state: State<AppState> = app.state();
    let vault_id = Uuid::new_v4().to_string();
    let profile_id = "profile-123";
    let scan1_job_id = "scan-job-1";
    let scan1_broker_scan_id = "broker-scan-1";
    let scan2_job_id = "scan-job-2";
    let scan2_broker_scan_id = "broker-scan-2";

    // Create and unlock vault
    create_test_vault(&state, &vault_id).await;

    // Get vault to create test data
    let vault = state.get_vault(&vault_id).expect("get vault");
    let db = vault.database().expect("get database");

    // Create test data structure for scan 1
    create_test_scan_structure(&vault, profile_id, scan1_job_id, scan1_broker_scan_id).await;

    // Create test data structure for scan 2 (need separate broker scan)
    sqlx::query(
        "INSERT INTO scan_jobs (id, profile_id, started_at, status, total_brokers, completed_brokers) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(scan2_job_id)
    .bind(profile_id)
    .bind(chrono::Utc::now().to_rfc3339())
    .bind("Completed")
    .bind(3)
    .bind(3)
    .execute(db.pool())
    .await
    .expect("create scan job 2");

    sqlx::query(
        "INSERT INTO broker_scans (id, scan_job_id, broker_id, status, started_at) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(scan2_broker_scan_id)
    .bind(scan2_job_id)
    .bind("spokeo")
    .bind("Success")
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(db.pool())
    .await
    .expect("create broker scan 2");

    // Create confirmed finding in scan 1
    let finding1 = create_finding(
        db.pool(),
        scan1_broker_scan_id.to_string(),
        "spokeo".to_string(),
        profile_id.to_string(),
        "https://example.com/scan1/finding".to_string(),
        serde_json::json!({"name": "Scan 1 User"}),
    )
    .await
    .expect("create finding in scan 1");

    verify_finding(db.pool(), &finding1.id, true, true)
        .await
        .expect("verify finding in scan 1 as confirmed");

    // Create confirmed finding in scan 2
    let finding2 = create_finding(
        db.pool(),
        scan2_broker_scan_id.to_string(),
        "spokeo".to_string(),
        profile_id.to_string(),
        "https://example.com/scan2/finding".to_string(),
        serde_json::json!({"name": "Scan 2 User"}),
    )
    .await
    .expect("create finding in scan 2");

    verify_finding(db.pool(), &finding2.id, true, true)
        .await
        .expect("verify finding in scan 2 as confirmed");

    // Call submit_removals_for_confirmed for scan 1 only
    let removal_ids =
        submit_removals_for_confirmed(state.clone(), vault_id.clone(), scan1_job_id.to_string())
            .await
            .expect("submit removals for scan 1");

    // Should return only 1 removal attempt (for scan 1)
    assert_eq!(removal_ids.len(), 1);

    // Verify only finding1 has removal_attempt_id set
    let scan1_findings = spectral_db::findings::get_by_scan_job(db.pool(), scan1_job_id)
        .await
        .expect("get scan 1 findings");
    let scan2_findings = spectral_db::findings::get_by_scan_job(db.pool(), scan2_job_id)
        .await
        .expect("get scan 2 findings");

    assert_eq!(scan1_findings.len(), 1);
    assert_eq!(scan2_findings.len(), 1);

    // Finding from scan 1 should have removal_attempt_id
    assert!(scan1_findings[0].removal_attempt_id.is_some());

    // Finding from scan 2 should NOT have removal_attempt_id (we didn't submit for it)
    assert!(scan2_findings[0].removal_attempt_id.is_none());
}
