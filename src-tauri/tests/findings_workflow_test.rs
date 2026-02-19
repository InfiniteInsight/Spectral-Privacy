//! Comprehensive integration test for the complete findings workflow.
//!
//! Tests the end-to-end flow: scan → findings → get findings → verify → submit removals.
//! Validates database state, Tauri command responses, deduplication, and finding-to-removal linkage.

use spectral_app::commands::scan::*;
use spectral_app::state::AppState;
use spectral_db::findings::{create_finding, get_by_scan_job, VerificationStatus};
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
}

#[tokio::test]
async fn test_full_findings_workflow() {
    // ===== SETUP PHASE =====
    let (app, _temp_dir) = create_test_app();
    let state: State<AppState> = app.state();
    let vault_id = Uuid::new_v4().to_string();
    let profile_id = "test-profile-123";
    let scan_job_id = "scan-job-456";
    let broker_scan_id = "broker-scan-789";

    // Create and unlock vault
    create_test_vault(&state, &vault_id).await;

    // Get vault and database
    let vault = state.get_vault(&vault_id).expect("get vault");
    let db = vault.database().expect("get database");

    // Create test data structure
    create_test_scan_structure(&vault, profile_id, scan_job_id, broker_scan_id).await;

    // ===== FINDINGS CREATED (Simulated Scan Results) =====
    // Create 2 findings with realistic extracted data
    let finding1 = create_finding(
        db.pool(),
        broker_scan_id.to_string(),
        "test-broker".to_string(),
        profile_id.to_string(),
        "https://broker.example.com/person/1".to_string(),
        serde_json::json!({
            "name": "John Doe",
            "age": 42,
            "addresses": ["123 Main St, Anytown, CA"],
            "phone_numbers": ["(555) 123-4567"],
            "relatives": [],
            "emails": []
        }),
    )
    .await
    .expect("create finding 1");

    let finding2 = create_finding(
        db.pool(),
        broker_scan_id.to_string(),
        "test-broker".to_string(),
        profile_id.to_string(),
        "https://broker.example.com/person/2".to_string(),
        serde_json::json!({
            "name": "Jane Smith",
            "age": 35,
            "addresses": ["456 Oak Ave, Somewhere, TX"],
            "phone_numbers": ["(555) 987-6543"],
            "relatives": [],
            "emails": []
        }),
    )
    .await
    .expect("create finding 2");

    // Verify findings in database
    let all_findings = get_by_scan_job(db.pool(), scan_job_id)
        .await
        .expect("get findings from database");

    assert_eq!(all_findings.len(), 2, "Should have 2 findings in database");

    // Verify all findings start with PendingVerification
    assert_eq!(
        finding1.verification_status,
        VerificationStatus::PendingVerification
    );
    assert_eq!(
        finding2.verification_status,
        VerificationStatus::PendingVerification
    );

    // Verify no duplicates (both have different listing URLs)
    assert_ne!(finding1.listing_url, finding2.listing_url);

    // ===== GET FINDINGS (Tauri Command) =====
    // Test the get_findings command
    let findings_response = get_findings(
        state.clone(),
        vault_id.clone(),
        scan_job_id.to_string(),
        None,
    )
    .await
    .expect("get_findings command should succeed");

    assert_eq!(
        findings_response.len(),
        2,
        "get_findings should return 2 findings"
    );

    // Verify FindingResponse structure (check both findings)
    for response in &findings_response {
        assert!(!response.id.is_empty());
        assert_eq!(response.broker_id, "test-broker");
        assert!(!response.listing_url.is_empty());
        assert_eq!(response.verification_status, "PendingVerification");
        assert!(response.extracted_data.name.is_some());
        assert!(response.extracted_data.age.is_some());
        assert_eq!(response.extracted_data.addresses.len(), 1);
        assert_eq!(response.extracted_data.phone_numbers.len(), 1);
    }

    // Verify we have both expected names (order doesn't matter)
    let names: Vec<&str> = findings_response
        .iter()
        .filter_map(|f| f.extracted_data.name.as_deref())
        .collect();
    assert!(names.contains(&"John Doe"));
    assert!(names.contains(&"Jane Smith"));

    // Test filtering by status
    let pending_findings = get_findings(
        state.clone(),
        vault_id.clone(),
        scan_job_id.to_string(),
        Some("PendingVerification".to_string()),
    )
    .await
    .expect("get_findings with filter should succeed");

    assert_eq!(pending_findings.len(), 2, "Should have 2 pending findings");

    // ===== VERIFY FINDING (Tauri Command) =====
    // Confirm first finding, reject second finding
    verify_finding(
        state.clone(),
        vault_id.clone(),
        finding1.id.clone(),
        true, // is_match = true (Confirmed)
    )
    .await
    .expect("verify_finding (confirm) should succeed");

    verify_finding(
        state.clone(),
        vault_id.clone(),
        finding2.id.clone(),
        false, // is_match = false (Rejected)
    )
    .await
    .expect("verify_finding (reject) should succeed");

    // Query database to verify statuses updated
    let updated_findings = get_by_scan_job(db.pool(), scan_job_id)
        .await
        .expect("get updated findings");

    let updated_finding1 = updated_findings
        .iter()
        .find(|f| f.id == finding1.id)
        .expect("find updated finding1");
    let updated_finding2 = updated_findings
        .iter()
        .find(|f| f.id == finding2.id)
        .expect("find updated finding2");

    assert_eq!(
        updated_finding1.verification_status,
        VerificationStatus::Confirmed
    );
    assert_eq!(
        updated_finding2.verification_status,
        VerificationStatus::Rejected
    );

    // Test filtering works
    let confirmed_findings = get_findings(
        state.clone(),
        vault_id.clone(),
        scan_job_id.to_string(),
        Some("Confirmed".to_string()),
    )
    .await
    .expect("get confirmed findings");

    assert_eq!(
        confirmed_findings.len(),
        1,
        "Should have 1 confirmed finding"
    );
    assert_eq!(confirmed_findings[0].id, finding1.id);

    let rejected_findings = get_findings(
        state.clone(),
        vault_id.clone(),
        scan_job_id.to_string(),
        Some("Rejected".to_string()),
    )
    .await
    .expect("get rejected findings");

    assert_eq!(rejected_findings.len(), 1, "Should have 1 rejected finding");
    assert_eq!(rejected_findings[0].id, finding2.id);

    // ===== SUBMIT REMOVALS (Tauri Command) =====
    let removal_ids =
        submit_removals_for_confirmed(state.clone(), vault_id.clone(), scan_job_id.to_string())
            .await
            .expect("submit_removals_for_confirmed should succeed");

    // Should create 1 removal attempt (only for confirmed finding)
    assert_eq!(
        removal_ids.len(),
        1,
        "Should create 1 removal attempt for confirmed finding"
    );

    // Query removal_attempts table to verify record created
    let removal_attempt = spectral_db::removal_attempts::get_by_id(db.pool(), &removal_ids[0])
        .await
        .expect("query removal attempt")
        .expect("removal attempt should exist");

    // Verify removal_attempt has Pending status
    assert_eq!(
        removal_attempt.status,
        spectral_db::removal_attempts::RemovalStatus::Pending
    );

    // Verify finding has removal_attempt_id set
    let final_findings = get_by_scan_job(db.pool(), scan_job_id)
        .await
        .expect("get final findings");

    let final_finding1 = final_findings
        .iter()
        .find(|f| f.id == finding1.id)
        .expect("find final finding1");
    let final_finding2 = final_findings
        .iter()
        .find(|f| f.id == finding2.id)
        .expect("find final finding2");

    // Confirmed finding should have removal_attempt_id
    assert!(final_finding1.removal_attempt_id.is_some());
    assert_eq!(
        final_finding1.removal_attempt_id.as_ref().unwrap(),
        &removal_ids[0]
    );

    // Rejected finding should NOT have removal_attempt_id
    assert!(final_finding2.removal_attempt_id.is_none());

    // ===== DEDUPLICATION VERIFICATION =====
    // Test deduplication by trying to create the same finding again
    let dedup_result =
        spectral_db::findings::finding_exists_by_url(db.pool(), scan_job_id, &finding1.listing_url)
            .await
            .expect("check finding exists");

    assert!(
        dedup_result,
        "Deduplication check should return true for existing finding"
    );

    // Try with a different URL - should not exist
    let not_exists = spectral_db::findings::finding_exists_by_url(
        db.pool(),
        scan_job_id,
        "https://broker.example.com/person/999",
    )
    .await
    .expect("check finding not exists");

    assert!(
        !not_exists,
        "Deduplication check should return false for non-existent URL"
    );

    println!("✅ Full findings workflow test completed successfully!");
}

/// Test get_findings command with invalid vault.
#[tokio::test]
async fn test_get_findings_invalid_vault() {
    let (app, _temp_dir) = create_test_app();
    let state: State<AppState> = app.state();

    let result = get_findings(
        state,
        "non-existent-vault".to_string(),
        "scan-123".to_string(),
        None,
    )
    .await;

    assert!(result.is_err());
}

/// Test verify_finding command with invalid vault.
#[tokio::test]
async fn test_verify_finding_invalid_vault() {
    let (app, _temp_dir) = create_test_app();
    let state: State<AppState> = app.state();

    let result = verify_finding(
        state,
        "non-existent-vault".to_string(),
        "finding-123".to_string(),
        true,
    )
    .await;

    assert!(result.is_err());
}

/// Test that get_findings works correctly with no findings.
#[tokio::test]
async fn test_get_findings_empty() {
    let (app, _temp_dir) = create_test_app();
    let state: State<AppState> = app.state();
    let vault_id = Uuid::new_v4().to_string();
    let profile_id = "profile-123";
    let scan_job_id = "scan-job-456";

    // Create and unlock vault
    create_test_vault(&state, &vault_id).await;

    // Get vault to create test data
    let vault = state.get_vault(&vault_id).expect("get vault");
    let db = vault.database().expect("get database");

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
    .execute(db.pool())
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
    .execute(db.pool())
    .await
    .expect("create scan job");

    // Call get_findings - should return empty array
    let findings = get_findings(state, vault_id, scan_job_id.to_string(), None)
        .await
        .expect("get_findings should succeed");

    assert_eq!(findings.len(), 0, "Should return empty array");
}

/// Test that findings with multiple statuses are filtered correctly.
#[tokio::test]
async fn test_findings_status_filtering() {
    let (app, _temp_dir) = create_test_app();
    let state: State<AppState> = app.state();
    let vault_id = Uuid::new_v4().to_string();
    let profile_id = "profile-123";
    let scan_job_id = "scan-job-456";
    let broker_scan_id = "broker-scan-789";

    // Create and unlock vault
    create_test_vault(&state, &vault_id).await;

    // Get vault and database
    let vault = state.get_vault(&vault_id).expect("get vault");
    let db = vault.database().expect("get database");

    // Create test data structure
    create_test_scan_structure(&vault, profile_id, scan_job_id, broker_scan_id).await;

    // Create 5 findings with different statuses
    let mut finding_ids = Vec::new();
    for i in 1..=5 {
        let finding = create_finding(
            db.pool(),
            broker_scan_id.to_string(),
            "test-broker".to_string(),
            profile_id.to_string(),
            format!("https://broker.example.com/person/{}", i),
            serde_json::json!({"name": format!("Person {}", i)}),
        )
        .await
        .expect("create finding");
        finding_ids.push(finding.id);
    }

    // Confirm 2 findings
    spectral_db::findings::verify_finding(db.pool(), &finding_ids[0], true, true)
        .await
        .expect("confirm finding 1");
    spectral_db::findings::verify_finding(db.pool(), &finding_ids[1], true, true)
        .await
        .expect("confirm finding 2");

    // Reject 2 findings
    spectral_db::findings::verify_finding(db.pool(), &finding_ids[2], false, true)
        .await
        .expect("reject finding 3");
    spectral_db::findings::verify_finding(db.pool(), &finding_ids[3], false, true)
        .await
        .expect("reject finding 4");

    // Leave 1 finding pending (finding_ids[4])

    // Test filtering
    let all_findings = get_findings(
        state.clone(),
        vault_id.clone(),
        scan_job_id.to_string(),
        None,
    )
    .await
    .expect("get all findings");
    assert_eq!(all_findings.len(), 5);

    let confirmed = get_findings(
        state.clone(),
        vault_id.clone(),
        scan_job_id.to_string(),
        Some("Confirmed".to_string()),
    )
    .await
    .expect("get confirmed findings");
    assert_eq!(confirmed.len(), 2);

    let rejected = get_findings(
        state.clone(),
        vault_id.clone(),
        scan_job_id.to_string(),
        Some("Rejected".to_string()),
    )
    .await
    .expect("get rejected findings");
    assert_eq!(rejected.len(), 2);

    let pending = get_findings(
        state.clone(),
        vault_id.clone(),
        scan_job_id.to_string(),
        Some("PendingVerification".to_string()),
    )
    .await
    .expect("get pending findings");
    assert_eq!(pending.len(), 1);
}
