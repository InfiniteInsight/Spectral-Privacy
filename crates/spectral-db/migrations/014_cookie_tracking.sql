-- Cookie tracking and removal system
-- Tracks browser cookies matched against broker definitions for removal

CREATE TABLE IF NOT EXISTS browser_cookies (
    id TEXT PRIMARY KEY NOT NULL,
    vault_id TEXT NOT NULL,
    browser_type TEXT NOT NULL CHECK(browser_type IN ('Chrome', 'Firefox', 'Safari', 'Edge', 'Brave', 'Other')),
    profile_name TEXT,
    cookie_name TEXT NOT NULL,
    cookie_domain TEXT NOT NULL,
    cookie_value TEXT,
    cookie_path TEXT NOT NULL DEFAULT '/',
    creation_time INTEGER,
    expiry_time INTEGER,
    last_access_time INTEGER,
    is_secure INTEGER NOT NULL DEFAULT 0,
    is_httponly INTEGER NOT NULL DEFAULT 0,
    same_site TEXT CHECK(same_site IN ('None', 'Lax', 'Strict', NULL)),
    matched_broker_id TEXT,
    scan_timestamp TEXT NOT NULL,
    removal_status TEXT NOT NULL DEFAULT 'Pending' CHECK(removal_status IN ('Pending', 'Removed', 'Failed', 'Protected')),
    removed_at TEXT,
    FOREIGN KEY (vault_id) REFERENCES vaults(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_cookies_vault ON browser_cookies(vault_id);
CREATE INDEX IF NOT EXISTS idx_cookies_browser ON browser_cookies(browser_type);
CREATE INDEX IF NOT EXISTS idx_cookies_broker ON browser_cookies(matched_broker_id);
CREATE INDEX IF NOT EXISTS idx_cookies_status ON browser_cookies(removal_status);
CREATE INDEX IF NOT EXISTS idx_cookies_domain ON browser_cookies(cookie_domain);

-- Cookie scan sessions to track scan history
CREATE TABLE IF NOT EXISTS cookie_scans (
    id TEXT PRIMARY KEY NOT NULL,
    vault_id TEXT NOT NULL,
    scan_timestamp TEXT NOT NULL,
    browsers_scanned TEXT NOT NULL, -- JSON array of browser types
    total_cookies_found INTEGER NOT NULL DEFAULT 0,
    matched_cookies INTEGER NOT NULL DEFAULT 0,
    brokers_matched TEXT, -- JSON array of broker IDs
    scan_status TEXT NOT NULL CHECK(scan_status IN ('InProgress', 'Completed', 'Failed')),
    error_message TEXT,
    completed_at TEXT,
    FOREIGN KEY (vault_id) REFERENCES vaults(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_cookie_scans_vault ON cookie_scans(vault_id);
CREATE INDEX IF NOT EXISTS idx_cookie_scans_timestamp ON cookie_scans(scan_timestamp);

-- Cookie removal operations
CREATE TABLE IF NOT EXISTS cookie_removals (
    id TEXT PRIMARY KEY NOT NULL,
    vault_id TEXT NOT NULL,
    scan_id TEXT,
    browser_type TEXT NOT NULL,
    profile_name TEXT,
    cookies_to_remove INTEGER NOT NULL DEFAULT 0,
    cookies_removed INTEGER NOT NULL DEFAULT 0,
    cookies_failed INTEGER NOT NULL DEFAULT 0,
    removal_timestamp TEXT NOT NULL,
    completion_timestamp TEXT,
    status TEXT NOT NULL CHECK(status IN ('Pending', 'InProgress', 'Completed', 'Failed', 'PartialSuccess')),
    error_message TEXT,
    backup_path TEXT,
    FOREIGN KEY (vault_id) REFERENCES vaults(id) ON DELETE CASCADE,
    FOREIGN KEY (scan_id) REFERENCES cookie_scans(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_cookie_removals_vault ON cookie_removals(vault_id);
CREATE INDEX IF NOT EXISTS idx_cookie_removals_scan ON cookie_removals(scan_id);
CREATE INDEX IF NOT EXISTS idx_cookie_removals_status ON cookie_removals(status);
