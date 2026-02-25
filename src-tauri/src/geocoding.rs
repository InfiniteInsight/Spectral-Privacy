use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;

// In-memory cache: Address → (lat, lon)
static GEOCODE_CACHE: Lazy<Mutex<HashMap<String, (f64, f64)>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Deserialize)]
struct NominatimResponse {
    #[serde(rename = "type")]
    _type: String,
    features: Vec<NominatimFeature>,
}

#[derive(Debug, Deserialize)]
struct NominatimFeature {
    geometry: NominatimGeometry,
}

#[derive(Debug, Deserialize)]
struct NominatimGeometry {
    coordinates: (f64, f64), // [lon, lat]
}

pub async fn geocode_address(
    street: &str,
    city: &str,
    state: &str,
    zip: &str,
) -> Option<(f64, f64)> {
    let full_address = format!("{}, {}, {} {}", street, city, state, zip);

    // Check cache first
    if let Ok(cache) = GEOCODE_CACHE.lock() {
        if let Some(&coords) = cache.get(&full_address) {
            return Some(coords);
        }
    }

    // Geocode using OpenStreetMap Nominatim API
    let client = reqwest::Client::new();
    let response = client
        .get("https://nominatim.openstreetmap.org/search")
        .query(&[("q", full_address.as_str()), ("format", "geojson")])
        .header("User-Agent", "Spectral-Privacy-App/0.1.0")
        .send()
        .await;

    match response {
        Ok(resp) => {
            if let Ok(data) = resp.json::<NominatimResponse>().await {
                if let Some(feature) = data.features.first() {
                    // Nominatim returns [lon, lat], we want (lat, lon)
                    let coords = (
                        feature.geometry.coordinates.1,
                        feature.geometry.coordinates.0,
                    );

                    // Cache result
                    if let Ok(mut cache) = GEOCODE_CACHE.lock() {
                        cache.insert(full_address.clone(), coords);
                    }

                    return Some(coords);
                }
            }
            None
        }
        Err(e) => {
            tracing::warn!("Geocoding failed for {}: {}", full_address, e);
            None
        }
    }
}
