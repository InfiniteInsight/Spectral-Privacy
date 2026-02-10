//! Integration tests for vault commands.

use spectral_app::commands::vault::*;
use spectral_app::state::AppState;
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
    };

    let app = tauri::test::mock_app();
    app.manage(app_state);

    (app, temp_dir)
}

#[tokio::test]
async fn test_full_vault_lifecycle() {
    let (app, _temp_dir) = create_test_app();
    let state: State<AppState> = app.state();
    let vault_id = Uuid::new_v4().to_string();
    let password = "test-password-123"; // pragma: allowlist secret

    // 1. Create vault
    vault_create(
        state.clone(),
        vault_id.clone(),
        "Test Vault".to_string(),
        password.to_string(),
    )
    .await
    .expect("create vault");

    // 2. Verify exists and unlocked
    let status = vault_status(state.clone(), vault_id.clone())
        .await
        .expect("get status");
    assert!(status.exists);
    assert!(status.unlocked);
    assert_eq!(status.display_name, Some("Test Vault".to_string()));

    // 3. Lock vault
    vault_lock(state.clone(), vault_id.clone())
        .await
        .expect("lock vault");

    // 4. Verify exists but locked
    let status = vault_status(state.clone(), vault_id.clone())
        .await
        .expect("get status");
    assert!(status.exists);
    assert!(!status.unlocked);

    // 5. Unlock with correct password
    vault_unlock(state.clone(), vault_id.clone(), password.to_string())
        .await
        .expect("unlock vault");

    let status = vault_status(state.clone(), vault_id.clone())
        .await
        .expect("get status");
    assert!(status.unlocked);

    // 6. Lock vault first before testing wrong password
    vault_lock(state.clone(), vault_id.clone())
        .await
        .expect("lock vault");

    // 7. Verify wrong password fails
    let result = vault_unlock(
        state.clone(),
        vault_id.clone(),
        "wrong-password".to_string(),
    )
    .await;
    assert!(result.is_err());

    // 8. Unlock with correct password again
    vault_unlock(state.clone(), vault_id.clone(), password.to_string())
        .await
        .expect("unlock vault");

    // 9. Test list_vaults sees it
    let vaults = list_vaults(state.clone()).await.expect("list vaults");
    assert_eq!(vaults.len(), 1);
    assert_eq!(vaults[0].vault_id, vault_id);
    assert_eq!(vaults[0].display_name, "Test Vault");
    assert!(vaults[0].unlocked);
}

#[tokio::test]
async fn test_multiple_vaults() {
    let (app, _temp_dir) = create_test_app();
    let state: State<AppState> = app.state();
    let vault1_id = Uuid::new_v4().to_string();
    let vault2_id = Uuid::new_v4().to_string();
    let password1 = "password-1"; // pragma: allowlist secret
    let password2 = "password-2"; // pragma: allowlist secret

    // 1. Create vault1 and vault2
    vault_create(
        state.clone(),
        vault1_id.clone(),
        "Vault 1".to_string(),
        password1.to_string(),
    )
    .await
    .expect("create vault1");

    vault_create(
        state.clone(),
        vault2_id.clone(),
        "Vault 2".to_string(),
        password2.to_string(),
    )
    .await
    .expect("create vault2");

    // 2. Verify both unlocked
    let vaults = list_vaults(state.clone()).await.expect("list vaults");
    assert_eq!(vaults.len(), 2);
    assert!(vaults.iter().all(|v| v.unlocked));

    // 3. Lock vault1
    vault_lock(state.clone(), vault1_id.clone())
        .await
        .expect("lock vault1");

    // 4. Verify vault2 still unlocked
    let status1 = vault_status(state.clone(), vault1_id.clone())
        .await
        .expect("get status1");
    let status2 = vault_status(state.clone(), vault2_id.clone())
        .await
        .expect("get status2");

    assert!(!status1.unlocked);
    assert!(status2.unlocked);

    // 5. Verify list shows correct states
    let vaults = list_vaults(state.clone()).await.expect("list vaults");
    assert_eq!(vaults.len(), 2);

    let vault1_info = vaults.iter().find(|v| v.vault_id == vault1_id).unwrap();
    let vault2_info = vaults.iter().find(|v| v.vault_id == vault2_id).unwrap();

    assert!(!vault1_info.unlocked);
    assert!(vault2_info.unlocked);
}
