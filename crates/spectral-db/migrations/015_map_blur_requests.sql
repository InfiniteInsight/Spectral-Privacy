-- Map blur request tracking for Google Maps, Apple Maps, and Bing Maps
CREATE TABLE IF NOT EXISTS map_blur_requests (
    id TEXT PRIMARY KEY NOT NULL,
    profile_id TEXT NOT NULL,
    service TEXT NOT NULL CHECK(service IN ('GoogleMaps', 'AppleMaps', 'BingMaps')),
    status TEXT NOT NULL CHECK(status IN ('URLGenerated', 'Submitted', 'Completed', 'Failed')),
    request_url TEXT NOT NULL,
    street_address TEXT NOT NULL,
    latitude REAL,
    longitude REAL,
    generated_at TEXT NOT NULL,
    submitted_at TEXT,
    completed_at TEXT,
    notes TEXT,
    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE,
    UNIQUE(profile_id, service)
);

CREATE INDEX idx_map_blur_profile ON map_blur_requests(profile_id);
CREATE INDEX idx_map_blur_service ON map_blur_requests(service);
CREATE INDEX idx_map_blur_status ON map_blur_requests(status);
