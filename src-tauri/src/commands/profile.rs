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
#[allow(deprecated)]
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
            format!("Vault '{vault_id}' is not unlocked"),
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

    // Add Phase 2 fields
    // Add phone numbers
    if let Some(ref phones) = input.phone_numbers {
        for phone_input in phones {
            let phone_type = match phone_input.phone_type {
                crate::types::profile::PhoneTypeInput::Mobile => {
                    spectral_vault::profile::PhoneType::Mobile
                }
                crate::types::profile::PhoneTypeInput::Home => {
                    spectral_vault::profile::PhoneType::Home
                }
                crate::types::profile::PhoneTypeInput::Work => {
                    spectral_vault::profile::PhoneType::Work
                }
            };
            profile
                .phone_numbers
                .push(spectral_vault::profile::PhoneNumber::new(
                    &phone_input.number,
                    phone_type,
                    key,
                )?);
        }
    }

    // Add email addresses
    if let Some(ref emails) = input.email_addresses {
        for email_input in emails {
            let email_type = match email_input.email_type {
                crate::types::profile::EmailTypeInput::Personal => {
                    spectral_vault::profile::EmailType::Personal
                }
                crate::types::profile::EmailTypeInput::Work => {
                    spectral_vault::profile::EmailType::Work
                }
                crate::types::profile::EmailTypeInput::Other => {
                    spectral_vault::profile::EmailType::Other
                }
            };
            profile
                .email_addresses
                .push(spectral_vault::profile::EmailAddress::new(
                    &email_input.email,
                    email_type,
                    key,
                )?);
        }
    }

    // Add previous addresses
    if let Some(ref addrs) = input.previous_addresses {
        for addr_input in addrs {
            profile
                .previous_addresses_v2
                .push(spectral_vault::profile::PreviousAddress {
                    address_line1: encrypt_string(&addr_input.address_line1, key)?,
                    address_line2: addr_input
                        .address_line2
                        .as_ref()
                        .map(|s| encrypt_string(s, key))
                        .transpose()?,
                    city: encrypt_string(&addr_input.city, key)?,
                    state: encrypt_string(&addr_input.state, key)?,
                    zip_code: encrypt_string(&addr_input.zip_code, key)?,
                    lived_from: addr_input.lived_from.clone(),
                    lived_to: addr_input.lived_to.clone(),
                });
        }
    }

    // Add aliases
    if let Some(ref alias_list) = input.aliases {
        for alias_input in alias_list {
            profile.aliases.push(spectral_vault::profile::Alias {
                first_name: alias_input
                    .first_name
                    .as_ref()
                    .map(|s| encrypt_string(s, key))
                    .transpose()?,
                middle_name: alias_input
                    .middle_name
                    .as_ref()
                    .map(|s| encrypt_string(s, key))
                    .transpose()?,
                last_name: alias_input
                    .last_name
                    .as_ref()
                    .map(|s| encrypt_string(s, key))
                    .transpose()?,
                nickname: alias_input
                    .nickname
                    .as_ref()
                    .map(|s| encrypt_string(s, key))
                    .transpose()?,
            });
        }
    }

    // Add relatives
    if let Some(ref relative_list) = input.relatives {
        for relative_input in relative_list {
            let relationship = match relative_input.relationship {
                crate::types::profile::RelationshipTypeInput::Spouse => {
                    spectral_vault::profile::RelationshipType::Spouse
                }
                crate::types::profile::RelationshipTypeInput::Partner => {
                    spectral_vault::profile::RelationshipType::Partner
                }
                crate::types::profile::RelationshipTypeInput::Child => {
                    spectral_vault::profile::RelationshipType::Child
                }
                crate::types::profile::RelationshipTypeInput::Parent => {
                    spectral_vault::profile::RelationshipType::Parent
                }
                crate::types::profile::RelationshipTypeInput::Sibling => {
                    spectral_vault::profile::RelationshipType::Sibling
                }
                crate::types::profile::RelationshipTypeInput::Other => {
                    spectral_vault::profile::RelationshipType::Other
                }
            };
            profile.relatives.push(spectral_vault::profile::Relative {
                first_name: relative_input
                    .first_name
                    .as_ref()
                    .map(|s| encrypt_string(s, key))
                    .transpose()?,
                middle_name: relative_input
                    .middle_name
                    .as_ref()
                    .map(|s| encrypt_string(s, key))
                    .transpose()?,
                last_name: relative_input
                    .last_name
                    .as_ref()
                    .map(|s| encrypt_string(s, key))
                    .transpose()?,
                maiden_name: relative_input
                    .maiden_name
                    .as_ref()
                    .map(|s| encrypt_string(s, key))
                    .transpose()?,
                relationship,
            });
        }
    }

    // Save profile
    vault.save_profile(&profile).await?;

    info!("Profile created: {}", profile_id);

    // Call profile_get to return complete output with Phase 2 fields
    profile_get(state, vault_id, profile_id.to_string()).await
}

/// Get a profile by ID.
///
/// Loads a profile from the vault and decrypts all fields.
#[allow(deprecated)]
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
            format!("Vault '{vault_id}' is not unlocked"),
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

    // Debug: Log old field presence
    info!(
        "Profile {}: phone_numbers.len={}, phone field present={}",
        profile_id,
        profile.phone_numbers.len(),
        profile.phone.is_some()
    );

    // Decrypt Phase 2 fields with migration from old fields
    let phone_numbers = if !profile.phone_numbers.is_empty() {
        let mut phones = Vec::new();
        for phone in &profile.phone_numbers {
            let number = phone.number.decrypt(key)?;
            let phone_type = match phone.phone_type {
                spectral_vault::profile::PhoneType::Mobile => "Mobile",
                spectral_vault::profile::PhoneType::Home => "Home",
                spectral_vault::profile::PhoneType::Work => "Work",
            };
            phones.push(crate::types::profile::PhoneNumberOutput {
                number,
                phone_type: phone_type.to_string(),
            });
        }
        Some(phones)
    } else if let Some(ref old_phone) = profile.phone {
        // Migration: convert old phone field to new format
        let number = old_phone.decrypt(key)?;
        if !number.is_empty() {
            Some(vec![crate::types::profile::PhoneNumberOutput {
                number,
                phone_type: "Mobile".to_string(), // Default to Mobile for migrated data
            }])
        } else {
            None
        }
    } else {
        None
    };

    let email_addresses = if !profile.email_addresses.is_empty() {
        let mut emails = Vec::new();
        for email_addr in &profile.email_addresses {
            let email = email_addr.email.decrypt(key)?;
            let email_type = match email_addr.email_type {
                spectral_vault::profile::EmailType::Personal => "Personal",
                spectral_vault::profile::EmailType::Work => "Work",
                spectral_vault::profile::EmailType::Other => "Other",
            };
            emails.push(crate::types::profile::EmailAddressOutput {
                email,
                email_type: email_type.to_string(),
            });
        }
        Some(emails)
    } else {
        // Migration: The old 'email' field is always populated from Phase 1 data above
        // We already have it in the 'email' variable, so create an email_addresses array from it
        if !email.is_empty() {
            Some(vec![crate::types::profile::EmailAddressOutput {
                email: email.clone(),
                email_type: "Personal".to_string(), // Default to Personal for migrated data
            }])
        } else {
            None
        }
    };

    let previous_addresses = if profile.previous_addresses_v2.is_empty() {
        None
    } else {
        let mut addrs = Vec::new();
        for addr in &profile.previous_addresses_v2 {
            let address_line1 = addr.address_line1.decrypt(key)?;
            let address_line2 = addr
                .address_line2
                .as_ref()
                .map(|f| f.decrypt(key))
                .transpose()?;
            let city = addr.city.decrypt(key)?;
            let state = addr.state.decrypt(key)?;
            let zip_code = addr.zip_code.decrypt(key)?;
            addrs.push(crate::types::profile::PreviousAddressOutput {
                address_line1,
                address_line2,
                city,
                state,
                zip_code,
                lived_from: addr.lived_from.clone(),
                lived_to: addr.lived_to.clone(),
            });
        }
        Some(addrs)
    };

    let aliases = if profile.aliases.is_empty() {
        None
    } else {
        let mut alias_list = Vec::new();
        for alias in &profile.aliases {
            let first_name = alias
                .first_name
                .as_ref()
                .map(|f| f.decrypt(key))
                .transpose()?;
            let middle_name = alias
                .middle_name
                .as_ref()
                .map(|f| f.decrypt(key))
                .transpose()?;
            let last_name = alias
                .last_name
                .as_ref()
                .map(|f| f.decrypt(key))
                .transpose()?;
            let nickname = alias
                .nickname
                .as_ref()
                .map(|f| f.decrypt(key))
                .transpose()?;
            alias_list.push(crate::types::profile::AliasOutput {
                first_name,
                middle_name,
                last_name,
                nickname,
            });
        }
        Some(alias_list)
    };

    let relatives = if profile.relatives.is_empty() {
        None
    } else {
        let mut relative_list = Vec::new();
        for relative in &profile.relatives {
            let first_name = relative
                .first_name
                .as_ref()
                .map(|f| f.decrypt(key))
                .transpose()?;
            let middle_name = relative
                .middle_name
                .as_ref()
                .map(|f| f.decrypt(key))
                .transpose()?;
            let last_name = relative
                .last_name
                .as_ref()
                .map(|f| f.decrypt(key))
                .transpose()?;
            let maiden_name = relative
                .maiden_name
                .as_ref()
                .map(|f| f.decrypt(key))
                .transpose()?;
            let relationship = match relative.relationship {
                spectral_vault::profile::RelationshipType::Spouse => "Spouse",
                spectral_vault::profile::RelationshipType::Partner => "Partner",
                spectral_vault::profile::RelationshipType::Parent => "Parent",
                spectral_vault::profile::RelationshipType::Child => "Child",
                spectral_vault::profile::RelationshipType::Sibling => "Sibling",
                spectral_vault::profile::RelationshipType::Other => "Other",
            };
            relative_list.push(crate::types::profile::RelativeOutput {
                first_name,
                middle_name,
                last_name,
                maiden_name,
                relationship: relationship.to_string(),
            });
        }
        Some(relative_list)
    };

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
        // Phase 2 fields
        phone_numbers,
        email_addresses,
        previous_addresses,
        aliases,
        relatives,
    })
}

/// Update an existing profile.
///
/// Updates all fields of a profile with validated input.
#[allow(deprecated)]
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
            format!("Vault '{vault_id}' is not unlocked"),
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

    // Update Phase 2 fields
    // Clear existing arrays
    profile.phone_numbers.clear();
    profile.email_addresses.clear();
    profile.previous_addresses_v2.clear();
    profile.aliases.clear();
    profile.relatives.clear();

    // Add phone numbers
    if let Some(ref phones) = input.phone_numbers {
        for phone_input in phones {
            let phone_type = match phone_input.phone_type {
                crate::types::profile::PhoneTypeInput::Mobile => {
                    spectral_vault::profile::PhoneType::Mobile
                }
                crate::types::profile::PhoneTypeInput::Home => {
                    spectral_vault::profile::PhoneType::Home
                }
                crate::types::profile::PhoneTypeInput::Work => {
                    spectral_vault::profile::PhoneType::Work
                }
            };
            profile
                .phone_numbers
                .push(spectral_vault::profile::PhoneNumber::new(
                    &phone_input.number,
                    phone_type,
                    key,
                )?);
        }
    }

    // Add email addresses
    if let Some(ref emails) = input.email_addresses {
        for email_input in emails {
            let email_type = match email_input.email_type {
                crate::types::profile::EmailTypeInput::Personal => {
                    spectral_vault::profile::EmailType::Personal
                }
                crate::types::profile::EmailTypeInput::Work => {
                    spectral_vault::profile::EmailType::Work
                }
                crate::types::profile::EmailTypeInput::Other => {
                    spectral_vault::profile::EmailType::Other
                }
            };
            profile
                .email_addresses
                .push(spectral_vault::profile::EmailAddress::new(
                    &email_input.email,
                    email_type,
                    key,
                )?);
        }
    }

    // Add previous addresses
    if let Some(ref addrs) = input.previous_addresses {
        for addr_input in addrs {
            profile
                .previous_addresses_v2
                .push(spectral_vault::profile::PreviousAddress {
                    address_line1: encrypt_string(&addr_input.address_line1, key)?,
                    address_line2: addr_input
                        .address_line2
                        .as_ref()
                        .map(|s| encrypt_string(s, key))
                        .transpose()?,
                    city: encrypt_string(&addr_input.city, key)?,
                    state: encrypt_string(&addr_input.state, key)?,
                    zip_code: encrypt_string(&addr_input.zip_code, key)?,
                    lived_from: addr_input.lived_from.clone(),
                    lived_to: addr_input.lived_to.clone(),
                });
        }
    }

    // Add aliases
    if let Some(ref alias_list) = input.aliases {
        for alias_input in alias_list {
            profile.aliases.push(spectral_vault::profile::Alias {
                first_name: alias_input
                    .first_name
                    .as_ref()
                    .map(|s| encrypt_string(s, key))
                    .transpose()?,
                middle_name: alias_input
                    .middle_name
                    .as_ref()
                    .map(|s| encrypt_string(s, key))
                    .transpose()?,
                last_name: alias_input
                    .last_name
                    .as_ref()
                    .map(|s| encrypt_string(s, key))
                    .transpose()?,
                nickname: alias_input
                    .nickname
                    .as_ref()
                    .map(|s| encrypt_string(s, key))
                    .transpose()?,
            });
        }
    }

    // Add relatives
    if let Some(ref relative_list) = input.relatives {
        for relative_input in relative_list {
            let relationship = match relative_input.relationship {
                crate::types::profile::RelationshipTypeInput::Spouse => {
                    spectral_vault::profile::RelationshipType::Spouse
                }
                crate::types::profile::RelationshipTypeInput::Partner => {
                    spectral_vault::profile::RelationshipType::Partner
                }
                crate::types::profile::RelationshipTypeInput::Parent => {
                    spectral_vault::profile::RelationshipType::Parent
                }
                crate::types::profile::RelationshipTypeInput::Child => {
                    spectral_vault::profile::RelationshipType::Child
                }
                crate::types::profile::RelationshipTypeInput::Sibling => {
                    spectral_vault::profile::RelationshipType::Sibling
                }
                crate::types::profile::RelationshipTypeInput::Other => {
                    spectral_vault::profile::RelationshipType::Other
                }
            };
            profile.relatives.push(spectral_vault::profile::Relative {
                first_name: relative_input
                    .first_name
                    .as_ref()
                    .map(|s| encrypt_string(s, key))
                    .transpose()?,
                middle_name: relative_input
                    .middle_name
                    .as_ref()
                    .map(|s| encrypt_string(s, key))
                    .transpose()?,
                last_name: relative_input
                    .last_name
                    .as_ref()
                    .map(|s| encrypt_string(s, key))
                    .transpose()?,
                maiden_name: relative_input
                    .maiden_name
                    .as_ref()
                    .map(|s| encrypt_string(s, key))
                    .transpose()?,
                relationship,
            });
        }
    }

    // Update timestamp
    profile.touch();

    // Save profile
    vault.save_profile(&profile).await?;

    info!("Profile updated: {}", profile_id);

    // Call profile_get to return complete output with Phase 2 fields
    profile_get(state, vault_id, profile_id).await
}

/// List all profiles in the vault.
///
/// Returns a summary of all profiles with basic information.
#[allow(deprecated)]
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
            format!("Vault '{vault_id}' is not unlocked"),
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
        let full_name = format!("{first_name} {last_name}").trim().to_string();

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
            format!("Vault '{vault_id}' is not unlocked"),
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
            phone_numbers: None,
            email_addresses: None,
            previous_addresses: None,
            aliases: None,
            relatives: None,
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
            phone_numbers: None,
            email_addresses: None,
            previous_addresses: None,
            aliases: None,
            relatives: None,
        };

        // Validation should pass
        assert!(valid_input.validate().is_ok());
    }
}
