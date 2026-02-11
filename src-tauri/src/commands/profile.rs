//! Profile management commands.

use crate::error::CommandError;
use crate::state::AppState;
use crate::types::profile::{ProfileInput, ProfileOutput, ProfileSummary};
use spectral_core::types::ProfileId;
use spectral_vault::cipher::encrypt_string;
use spectral_vault::UserProfile;
use tauri::State;
use tracing::info;

/// Create a new profile in the vault.
///
/// Creates a profile with validated input and returns the profile with generated ID.
#[tauri::command]
pub async fn profile_create(
    state: State<'_, AppState>,
    vault_id: String,
    input: ProfileInput,
) -> Result<ProfileOutput, CommandError> {
    info!("Creating profile in vault: {}", vault_id);

    // Validate input
    input.validate()?;

    // Get vault
    let vault = state.get_vault(&vault_id).ok_or_else(|| {
        CommandError::new(
            "VAULT_NOT_UNLOCKED",
            format!("Vault '{}' is not unlocked", vault_id),
        )
    })?;

    // Get the encryption key for field-level encryption
    let key = vault.encryption_key()?;

    // Create profile ID
    let profile_id = ProfileId::generate();

    // Build user profile with encrypted fields
    let mut profile = UserProfile::new(profile_id.clone());

    // Encrypt and store fields
    profile.first_name = Some(encrypt_string(&input.first_name, key)?);
    profile.middle_name = input
        .middle_name
        .as_ref()
        .map(|s| encrypt_string(s, key))
        .transpose()?;
    profile.last_name = Some(encrypt_string(&input.last_name, key)?);
    profile.email = Some(encrypt_string(&input.email, key)?);
    profile.date_of_birth = input
        .date_of_birth
        .map(|d| encrypt_string(&d.to_string(), key))
        .transpose()?;
    // Combine address lines if address_line2 exists
    let full_address = if let Some(ref line2) = input.address_line2 {
        format!("{}\n{}", input.address_line1, line2)
    } else {
        input.address_line1.clone()
    };
    profile.address = Some(encrypt_string(&full_address, key)?);
    profile.city = Some(encrypt_string(&input.city, key)?);
    profile.state = Some(encrypt_string(&input.state, key)?);
    profile.zip_code = Some(encrypt_string(&input.zip_code, key)?);

    // Save profile
    vault.save_profile(&profile).await?;

    info!("Profile created: {}", profile_id);

    // Return output
    Ok(ProfileOutput {
        id: profile_id.to_string(),
        first_name: input.first_name,
        middle_name: input.middle_name,
        last_name: input.last_name,
        email: input.email,
        date_of_birth: input.date_of_birth,
        address_line1: input.address_line1,
        address_line2: input.address_line2,
        city: input.city,
        state: input.state,
        zip_code: input.zip_code,
        created_at: profile.created_at.to_rfc3339(),
        updated_at: profile.updated_at.to_rfc3339(),
    })
}

/// Get a profile by ID.
///
/// Loads a profile from the vault and decrypts all fields.
#[tauri::command]
pub async fn profile_get(
    state: State<'_, AppState>,
    vault_id: String,
    profile_id: String,
) -> Result<ProfileOutput, CommandError> {
    info!("Getting profile {} from vault: {}", profile_id, vault_id);

    // Get vault
    let vault = state.get_vault(&vault_id).ok_or_else(|| {
        CommandError::new(
            "VAULT_NOT_UNLOCKED",
            format!("Vault '{}' is not unlocked", vault_id),
        )
    })?;

    // Parse profile ID
    let id = ProfileId::new(profile_id.clone())?;

    // Load profile
    let profile = vault.load_profile(&id).await?;

    // Get encryption key
    let key = vault.encryption_key()?;

    // Decrypt fields
    let first_name = profile
        .first_name
        .as_ref()
        .map(|f| f.decrypt(key))
        .transpose()?
        .unwrap_or_default();
    let middle_name = profile
        .middle_name
        .as_ref()
        .map(|f| f.decrypt(key))
        .transpose()?;
    let last_name = profile
        .last_name
        .as_ref()
        .map(|f| f.decrypt(key))
        .transpose()?
        .unwrap_or_default();
    let email = profile
        .email
        .as_ref()
        .map(|f| f.decrypt(key))
        .transpose()?
        .unwrap_or_default();
    let date_of_birth = profile
        .date_of_birth
        .as_ref()
        .map(|f| f.decrypt(key))
        .transpose()?
        .and_then(|s: String| s.parse().ok());
    // Decrypt and split address into two lines
    let (address_line1, address_line2) = profile
        .address
        .as_ref()
        .map(|f| f.decrypt(key))
        .transpose()?
        .map(|address_str: String| {
            let address_parts: Vec<&str> = address_str.split('\n').collect();
            let line1 = address_parts.first().unwrap_or(&"").to_string();
            let line2 = address_parts.get(1).map(|s| s.to_string());
            (line1, line2)
        })
        .unwrap_or((String::new(), None));
    let city = profile
        .city
        .as_ref()
        .map(|f| f.decrypt(key))
        .transpose()?
        .unwrap_or_default();
    let state_code = profile
        .state
        .as_ref()
        .map(|f| f.decrypt(key))
        .transpose()?
        .unwrap_or_default();
    let zip_code = profile
        .zip_code
        .as_ref()
        .map(|f| f.decrypt(key))
        .transpose()?
        .unwrap_or_default();

    Ok(ProfileOutput {
        id: profile_id,
        first_name,
        middle_name,
        last_name,
        email,
        date_of_birth,
        address_line1,
        address_line2,
        city,
        state: state_code,
        zip_code,
        created_at: profile.created_at.to_rfc3339(),
        updated_at: profile.updated_at.to_rfc3339(),
    })
}

/// Update an existing profile.
///
/// Updates all fields of a profile with validated input.
#[tauri::command]
pub async fn profile_update(
    state: State<'_, AppState>,
    vault_id: String,
    profile_id: String,
    input: ProfileInput,
) -> Result<ProfileOutput, CommandError> {
    info!("Updating profile {} in vault: {}", profile_id, vault_id);

    // Validate input
    input.validate()?;

    // Get vault
    let vault = state.get_vault(&vault_id).ok_or_else(|| {
        CommandError::new(
            "VAULT_NOT_UNLOCKED",
            format!("Vault '{}' is not unlocked", vault_id),
        )
    })?;

    // Parse profile ID
    let id = ProfileId::new(profile_id.clone())?;

    // Load existing profile
    let mut profile = vault.load_profile(&id).await?;

    // Get encryption key
    let key = vault.encryption_key()?;

    // Update encrypted fields
    profile.first_name = Some(encrypt_string(&input.first_name, key)?);
    profile.middle_name = input
        .middle_name
        .as_ref()
        .map(|s| encrypt_string(s, key))
        .transpose()?;
    profile.last_name = Some(encrypt_string(&input.last_name, key)?);
    profile.email = Some(encrypt_string(&input.email, key)?);
    profile.date_of_birth = input
        .date_of_birth
        .map(|d| encrypt_string(&d.to_string(), key))
        .transpose()?;
    // Combine address lines if address_line2 exists
    let full_address = if let Some(ref line2) = input.address_line2 {
        format!("{}\n{}", input.address_line1, line2)
    } else {
        input.address_line1.clone()
    };
    profile.address = Some(encrypt_string(&full_address, key)?);
    profile.city = Some(encrypt_string(&input.city, key)?);
    profile.state = Some(encrypt_string(&input.state, key)?);
    profile.zip_code = Some(encrypt_string(&input.zip_code, key)?);

    // Update timestamp
    profile.touch();

    // Save profile
    vault.save_profile(&profile).await?;

    info!("Profile updated: {}", profile_id);

    // Return output
    Ok(ProfileOutput {
        id: profile_id,
        first_name: input.first_name,
        middle_name: input.middle_name,
        last_name: input.last_name,
        email: input.email,
        date_of_birth: input.date_of_birth,
        address_line1: input.address_line1,
        address_line2: input.address_line2,
        city: input.city,
        state: input.state,
        zip_code: input.zip_code,
        created_at: profile.created_at.to_rfc3339(),
        updated_at: profile.updated_at.to_rfc3339(),
    })
}

/// List all profiles in the vault.
///
/// Returns a summary of all profiles with basic information.
#[tauri::command]
pub async fn profile_list(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<Vec<ProfileSummary>, CommandError> {
    info!("Listing profiles in vault: {}", vault_id);

    // Get vault
    let vault = state.get_vault(&vault_id).ok_or_else(|| {
        CommandError::new(
            "VAULT_NOT_UNLOCKED",
            format!("Vault '{}' is not unlocked", vault_id),
        )
    })?;

    // Get all profile IDs
    let profile_ids = vault.list_profiles().await?;

    // Get encryption key
    let key = vault.encryption_key()?;

    // Load and decrypt basic info for each profile
    let mut summaries = Vec::new();

    for id in profile_ids {
        let profile = vault.load_profile(&id).await?;

        // Decrypt first and last name for full name
        let first_name = profile
            .first_name
            .as_ref()
            .map(|f| f.decrypt(key))
            .transpose()?
            .unwrap_or_default();
        let last_name = profile
            .last_name
            .as_ref()
            .map(|f| f.decrypt(key))
            .transpose()?
            .unwrap_or_default();
        let full_name = format!("{} {}", first_name, last_name).trim().to_string();

        // Decrypt email
        let email = profile
            .email
            .as_ref()
            .map(|f| f.decrypt(key))
            .transpose()?
            .unwrap_or_default();

        summaries.push(ProfileSummary {
            id: id.to_string(),
            full_name,
            email,
            created_at: profile.created_at.to_rfc3339(),
        });
    }

    info!("Found {} profiles", summaries.len());
    Ok(summaries)
}

/// Get profile completeness score.
///
/// Returns completeness metrics for the first profile in the vault.
#[tauri::command]
pub async fn get_profile_completeness(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<spectral_vault::ProfileCompleteness, CommandError> {
    info!("Getting profile completeness for vault: {}", vault_id);

    // Get vault
    let vault = state.get_vault(&vault_id).ok_or_else(|| {
        CommandError::new(
            "VAULT_NOT_UNLOCKED",
            format!("Vault '{}' is not unlocked", vault_id),
        )
    })?;

    // Get all profile IDs
    let profile_ids = vault.list_profiles().await?;

    // Get the first profile (for now, assumes single profile)
    let profile_id = profile_ids
        .first()
        .ok_or_else(|| CommandError::new("NO_PROFILE", "No profile found in vault"))?;

    // Load profile
    let profile = vault.load_profile(profile_id).await?;

    // Calculate and return completeness
    Ok(profile.completeness_score())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These are compilation tests - actual functional tests would require
    // creating a vault, which is an integration test concern

    #[test]
    fn test_profile_commands_exist() {
        // This test just verifies the functions compile with correct signatures
        // We can't directly assign async functions to fn pointers, so we just
        // reference them to ensure they exist
        let _create = profile_create;
        let _get = profile_get;
        let _update = profile_update;
        let _list = profile_list;
    }

    #[test]
    fn test_profile_input_validation() {
        let invalid_input = ProfileInput {
            first_name: "".to_string(), // Invalid: empty name
            middle_name: None,
            last_name: "Doe".to_string(),
            email: "john@example.com".to_string(),
            date_of_birth: None,
            address_line1: "123 Main St".to_string(),
            address_line2: None,
            city: "San Francisco".to_string(),
            state: "CA".to_string(),
            zip_code: "94102".to_string(),
        };

        // Validation should fail
        assert!(invalid_input.validate().is_err());

        let valid_input = ProfileInput {
            first_name: "John".to_string(),
            middle_name: Some("A".to_string()),
            last_name: "Doe".to_string(),
            email: "john@example.com".to_string(),
            date_of_birth: Some(
                chrono::Local::now().date_naive() - chrono::Duration::days(365 * 30),
            ),
            address_line1: "123 Main St".to_string(),
            address_line2: Some("Apt 4B".to_string()),
            city: "San Francisco".to_string(),
            state: "CA".to_string(),
            zip_code: "94102".to_string(),
        };

        // Validation should pass
        assert!(valid_input.validate().is_ok());
    }
}
