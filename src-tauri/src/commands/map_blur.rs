//! Map blurring Tauri commands.

use crate::geocoding;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use spectral_core::types::ProfileId;
use spectral_db::map_blur::{MapBlurStatus, MapService};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapBlurRequestResponse {
    pub id: String,
    pub profile_id: String,
    pub service: String,
    pub status: String,
    pub request_url: String,
    pub street_address: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub generated_at: String,
    pub submitted_at: Option<String>,
    pub completed_at: Option<String>,
}

/// Generate map blur requests for all services for a profile's current address.
#[tauri::command]
pub async fn generate_map_blur_requests(
    state: State<'_, AppState>,
    vault_id: String,
    profile_id: String,
) -> Result<Vec<MapBlurRequestResponse>, String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| "Vault not unlocked".to_string())?;

    let db = vault.database().map_err(|e| e.to_string())?;
    let key = vault.encryption_key().map_err(|e| e.to_string())?;

    // Parse profile ID and load profile
    let id = ProfileId::new(profile_id.clone()).map_err(|e| e.to_string())?;
    let profile = vault
        .load_profile(&id)
        .await
        .map_err(|e| format!("Failed to load profile: {}", e))?;

    // Extract address components
    let address_line1 = profile
        .address
        .as_ref()
        .map(|f| f.decrypt(key))
        .transpose()
        .map_err(|e| format!("Failed to decrypt address: {}", e))?
        .ok_or_else(|| "No address found in profile".to_string())?;

    let city = profile
        .city
        .as_ref()
        .map(|f| f.decrypt(key))
        .transpose()
        .map_err(|e| format!("Failed to decrypt city: {}", e))?
        .ok_or_else(|| "No city found in profile".to_string())?;

    let state_code = profile
        .state
        .as_ref()
        .map(|f| f.decrypt(key))
        .transpose()
        .map_err(|e| format!("Failed to decrypt state: {}", e))?
        .ok_or_else(|| "No state found in profile".to_string())?;

    let zip = profile
        .zip_code
        .as_ref()
        .map(|f| f.decrypt(key))
        .transpose()
        .map_err(|e| format!("Failed to decrypt zip: {}", e))?
        .ok_or_else(|| "No ZIP code found in profile".to_string())?;

    let full_address = format!("{}, {}, {} {}", address_line1, city, state_code, zip);

    // Geocode address to get coordinates
    let coords = geocoding::geocode_address(&address_line1, &city, &state_code, &zip).await;

    if coords.is_none() {
        tracing::warn!("Geocoding failed for address, using address-only URLs");
    }

    // Generate requests for all 3 services
    let services = vec![
        MapService::GoogleMaps,
        MapService::AppleMaps,
        MapService::BingMaps,
    ];

    let mut responses = Vec::new();

    for service in services {
        let request_url = spectral_db::map_blur::generate_request_url(
            service,
            &full_address,
            coords.map(|(lat, _)| lat),
            coords.map(|(_, lon)| lon),
        );

        let request = spectral_db::map_blur::create_request(
            db.pool(),
            profile_id.clone(),
            service,
            request_url,
            full_address.clone(),
            coords.map(|(lat, _)| lat),
            coords.map(|(_, lon)| lon),
        )
        .await
        .map_err(|e| format!("Failed to create request: {}", e))?;

        // Log to audit log
        let _ = spectral_db::audit_log::insert_audit_entry(
            db.pool(),
            vault_id.clone(),
            "MapBlurURLGenerated".to_string(),
            format!("Generated {} blur request for profile", service.as_str()),
            None,
            "LocalOnly".to_string(),
            "Allowed".to_string(),
        )
        .await;

        responses.push(MapBlurRequestResponse {
            id: request.id,
            profile_id: request.profile_id,
            service: service.as_str().to_string(),
            status: "URLGenerated".to_string(),
            request_url: request.request_url,
            street_address: request.street_address,
            latitude: request.latitude,
            longitude: request.longitude,
            generated_at: request.generated_at.to_rfc3339(),
            submitted_at: None,
            completed_at: None,
        });
    }

    Ok(responses)
}

/// Get map blur requests for a profile.
#[tauri::command]
pub async fn get_map_blur_requests(
    state: State<'_, AppState>,
    vault_id: String,
    profile_id: String,
) -> Result<Vec<MapBlurRequestResponse>, String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| "Vault not unlocked".to_string())?;

    let db = vault.database().map_err(|e| e.to_string())?;

    let requests = spectral_db::map_blur::get_by_profile_id(db.pool(), &profile_id)
        .await
        .map_err(|e| format!("Failed to get requests: {}", e))?;

    Ok(requests
        .into_iter()
        .map(|r| MapBlurRequestResponse {
            id: r.id,
            profile_id: r.profile_id,
            service: r.service.as_str().to_string(),
            status: format!("{:?}", r.status),
            request_url: r.request_url,
            street_address: r.street_address,
            latitude: r.latitude,
            longitude: r.longitude,
            generated_at: r.generated_at.to_rfc3339(),
            submitted_at: r.submitted_at.map(|t| t.to_rfc3339()),
            completed_at: r.completed_at.map(|t| t.to_rfc3339()),
        })
        .collect())
}

/// Mark a map blur request as submitted.
#[tauri::command]
pub async fn mark_map_blur_submitted(
    state: State<'_, AppState>,
    vault_id: String,
    request_id: String,
    service: String,
) -> Result<(), String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| "Vault not unlocked".to_string())?;

    let db = vault.database().map_err(|e| e.to_string())?;

    spectral_db::map_blur::update_status(db.pool(), &request_id, MapBlurStatus::Submitted, None)
        .await
        .map_err(|e| format!("Failed to update status: {}", e))?;

    // Log to audit log
    let _ = spectral_db::audit_log::insert_audit_entry(
        db.pool(),
        vault_id,
        "MapBlurSubmitted".to_string(),
        format!("User marked {} blur request as submitted", service),
        None,
        format!("ExternalSite:{}.com", service.to_lowercase()),
        "Allowed".to_string(),
    )
    .await;

    Ok(())
}
