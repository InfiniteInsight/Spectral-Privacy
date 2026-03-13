-- Fix foreign key constraints in cookie tables
-- Migration 014 had FOREIGN KEY constraints referencing vaults(id) which doesn't exist
-- in per-vault databases. This migration removes those invalid constraints by recreating
-- the tables without them.

-- Only run this migration if the tables exist and have the bad foreign key
-- Check by attempting to recreate the tables

PRAGMA foreign_keys = OFF;

-- Recreate browser_cookies without vaults foreign key
DROP TABLE IF EXISTS browser_cookies_new;
CREATE TABLE browser_cookies_new (
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
    removed_at TEXT
);

INSERT INTO browser_cookies_new SELECT * FROM browser_cookies WHERE 1=0; -- Copy structure only
INSERT INTO browser_cookies_new SELECT * FROM browser_cookies; -- Copy data
DROP TABLE browser_cookies;
ALTER TABLE browser_cookies_new RENAME TO browser_cookies;

CREATE INDEX IF NOT EXISTS idx_cookies_vault ON browser_cookies(vault_id);
CREATE INDEX IF NOT EXISTS idx_cookies_browser ON browser_cookies(browser_type);
CREATE INDEX IF NOT EXISTS idx_cookies_broker ON browser_cookies(matched_broker_id);
CREATE INDEX IF NOT EXISTS idx_cookies_status ON browser_cookies(removal_status);
CREATE INDEX IF NOT EXISTS idx_cookies_domain ON browser_cookies(cookie_domain);

-- Recreate cookie_scans without vaults foreign key
DROP TABLE IF EXISTS cookie_scans_new;
CREATE TABLE cookie_scans_new (
    id TEXT PRIMARY KEY NOT NULL,
    vault_id TEXT NOT NULL,
    scan_timestamp TEXT NOT NULL,
    browsers_scanned TEXT NOT NULL,
    total_cookies_found INTEGER NOT NULL DEFAULT 0,
    matched_cookies INTEGER NOT NULL DEFAULT 0,
    brokers_matched TEXT,
    scan_status TEXT NOT NULL CHECK(scan_status IN ('InProgress', 'Completed', 'Failed')),
    error_message TEXT,
    completed_at TEXT
);

INSERT INTO cookie_scans_new SELECT * FROM cookie_scans WHERE 1=0; -- Copy structure only
INSERT INTO cookie_scans_new SELECT * FROM cookie_scans; -- Copy data
DROP TABLE cookie_scans;
ALTER TABLE cookie_scans_new RENAME TO cookie_scans;

CREATE INDEX IF NOT EXISTS idx_cookie_scans_vault ON cookie_scans(vault_id);
CREATE INDEX IF NOT EXISTS idx_cookie_scans_timestamp ON cookie_scans(scan_timestamp);

-- Recreate cookie_removals without vaults foreign key
DROP TABLE IF EXISTS cookie_removals_new;
CREATE TABLE cookie_removals_new (
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
    FOREIGN KEY (scan_id) REFERENCES cookie_scans(id) ON DELETE SET NULL
);

INSERT INTO cookie_removals_new SELECT * FROM cookie_removals WHERE 1=0; -- Copy structure only
INSERT INTO cookie_removals_new SELECT * FROM cookie_removals; -- Copy data
DROP TABLE cookie_removals;
ALTER TABLE cookie_removals_new RENAME TO cookie_removals;

CREATE INDEX IF NOT EXISTS idx_cookie_removals_vault ON cookie_removals(vault_id);
CREATE INDEX IF NOT EXISTS idx_cookie_removals_scan ON cookie_removals(scan_id);
CREATE INDEX IF NOT EXISTS idx_cookie_removals_status ON cookie_removals(status);

PRAGMA foreign_keys = ON;
