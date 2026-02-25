//! Map blurring request tracking for Google Maps, Apple Maps, and Bing Maps.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};
use std::str::FromStr;
use uuid::Uuid;

/// Map service provider for address blurring requests.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MapService {
    /// Google Maps Street View
    GoogleMaps,
    /// Apple Maps Look Around
    AppleMaps,
    /// Bing Maps Streetside
    BingMaps,
}

impl MapService {
    /// Get the string representation of the map service.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            MapService::GoogleMaps => "GoogleMaps",
            MapService::AppleMaps => "AppleMaps",
            MapService::BingMaps => "BingMaps",
        }
    }
}

impl FromStr for MapService {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GoogleMaps" => Ok(MapService::GoogleMaps),
            "AppleMaps" => Ok(MapService::AppleMaps),
            "BingMaps" => Ok(MapService::BingMaps),
            _ => Err(()),
        }
    }
}

/// Status of a map blur request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MapBlurStatus {
    /// URL has been generated but not yet submitted
    URLGenerated,
    /// User has submitted the request to the service
    Submitted,
    /// Service has completed the blurring
    Completed,
    /// Request failed or was rejected
    Failed,
}

/// A map blur request for a specific service and profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapBlurRequest {
    /// Unique request identifier
    pub id: String,
    /// Profile ID this request belongs to
    pub profile_id: String,
    /// Map service provider
    pub service: MapService,
    /// Current status of the request
    pub status: MapBlurStatus,
    /// Generated URL or mailto link for submission
    pub request_url: String,
    /// Full street address being blurred
    pub street_address: String,
    /// Geocoded latitude (if available)
    pub latitude: Option<f64>,
    /// Geocoded longitude (if available)
    pub longitude: Option<f64>,
    /// Timestamp when the request URL was generated
    pub generated_at: DateTime<Utc>,
    /// Timestamp when the user submitted the request
    pub submitted_at: Option<DateTime<Utc>>,
    /// Timestamp when the blurring was completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Optional notes about the request
    pub notes: Option<String>,
}

/// Generate request URL for a map service.
#[must_use]
pub fn generate_request_url(
    service: MapService,
    address: &str,
    lat: Option<f64>,
    lon: Option<f64>,
) -> String {
    match service {
        MapService::GoogleMaps => {
            if let (Some(lat), Some(lon)) = (lat, lon) {
                format!("https://www.google.com/maps/@?api=1&map_action=pano&viewpoint={lat},{lon}")
            } else {
                let encoded = urlencoding::encode(address);
                format!("https://www.google.com/maps/search/{encoded}")
            }
        }
        MapService::AppleMaps => {
            let subject = urlencoding::encode("Request to Blur Address in Apple Maps Look Around");
            let coords_text = if let (Some(lat), Some(lon)) = (lat, lon) {
                format!("Coordinates: {lat}, {lon}\n")
            } else {
                String::new()
            };
            let body_text = format!(
                "Hello Apple Maps Team,\n\n\
                I would like to request that my home address be blurred in Apple Maps Look Around imagery.\n\n\
                Address: {address}\n\
                {coords_text}\
                Thank you"
            );
            let body = urlencoding::encode(&body_text);
            format!("mailto:MapsImageCollection@apple.com?subject={subject}&body={body}")
        }
        MapService::BingMaps => {
            if let (Some(lat), Some(lon)) = (lat, lon) {
                format!("https://www.bing.com/maps?cp={lat}~{lon}&style=x")
            } else {
                let encoded = urlencoding::encode(address);
                format!("https://www.bing.com/maps?q={encoded}")
            }
        }
    }
}

/// Create a new map blur request.
pub async fn create_request(
    pool: &Pool<Sqlite>,
    profile_id: String,
    service: MapService,
    request_url: String,
    street_address: String,
    latitude: Option<f64>,
    longitude: Option<f64>,
) -> Result<MapBlurRequest, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    sqlx::query(
        "INSERT OR REPLACE INTO map_blur_requests
         (id, profile_id, service, status, request_url, street_address, latitude, longitude, generated_at)
         VALUES (?, ?, ?, 'URLGenerated', ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&profile_id)
    .bind(service.as_str())
    .bind(&request_url)
    .bind(&street_address)
    .bind(latitude)
    .bind(longitude)
    .bind(now.to_rfc3339())
    .execute(pool)
    .await?;

    Ok(MapBlurRequest {
        id,
        profile_id,
        service,
        status: MapBlurStatus::URLGenerated,
        request_url,
        street_address,
        latitude,
        longitude,
        generated_at: now,
        submitted_at: None,
        completed_at: None,
        notes: None,
    })
}

/// Get all map blur requests for a profile.
pub async fn get_by_profile_id(
    pool: &Pool<Sqlite>,
    profile_id: &str,
) -> Result<Vec<MapBlurRequest>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, profile_id, service, status, request_url, street_address,
                latitude, longitude, generated_at, submitted_at, completed_at, notes
         FROM map_blur_requests
         WHERE profile_id = ?
         ORDER BY service",
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await?;

    let requests = rows
        .into_iter()
        .filter_map(|row| {
            // nosemgrep: use-zeroize-for-secrets - These are non-sensitive database enum strings
            let service_str: String = row.get("service");
            let service = service_str.parse::<MapService>().ok()?;

            // nosemgrep: use-zeroize-for-secrets - These are non-sensitive database enum strings
            let status_str: String = row.get("status");
            let status = match status_str.as_str() {
                "URLGenerated" => MapBlurStatus::URLGenerated,
                "Submitted" => MapBlurStatus::Submitted,
                "Completed" => MapBlurStatus::Completed,
                "Failed" => MapBlurStatus::Failed,
                _ => return None,
            };

            // nosemgrep: use-zeroize-for-secrets - These are non-sensitive timestamp strings
            let generated_at_str: String = row.get("generated_at");
            let generated_at = DateTime::parse_from_rfc3339(&generated_at_str)
                .ok()?
                .with_timezone(&Utc);

            let submitted_at_str: Option<String> = row.get("submitted_at");
            let submitted_at = submitted_at_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            });

            let completed_at_str: Option<String> = row.get("completed_at");
            let completed_at = completed_at_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            });

            Some(MapBlurRequest {
                id: row.get("id"),
                profile_id: row.get("profile_id"),
                service,
                status,
                request_url: row.get("request_url"),
                street_address: row.get("street_address"),
                latitude: row.get("latitude"),
                longitude: row.get("longitude"),
                generated_at,
                submitted_at,
                completed_at,
                notes: row.get("notes"),
            })
        })
        .collect();

    Ok(requests)
}

/// Update request status.
pub async fn update_status(
    pool: &Pool<Sqlite>,
    request_id: &str,
    status: MapBlurStatus,
    notes: Option<String>,
) -> Result<(), sqlx::Error> {
    let timestamp = Utc::now().to_rfc3339();

    match status {
        MapBlurStatus::Submitted => {
            sqlx::query(
                "UPDATE map_blur_requests
                 SET status = 'Submitted', submitted_at = ?, notes = ?
                 WHERE id = ?",
            )
            .bind(&timestamp)
            .bind(&notes)
            .bind(request_id)
            .execute(pool)
            .await?;
        }
        MapBlurStatus::Completed => {
            sqlx::query(
                "UPDATE map_blur_requests
                 SET status = 'Completed', completed_at = ?, notes = ?
                 WHERE id = ?",
            )
            .bind(&timestamp)
            .bind(&notes)
            .bind(request_id)
            .execute(pool)
            .await?;
        }
        _ => {}
    }

    Ok(())
}
