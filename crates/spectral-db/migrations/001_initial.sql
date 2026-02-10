-- Initial schema for Spectral database
--
-- This migration creates the core tables for profile management,
-- broker scanning results, and audit logging.

-- Profiles table: stores encrypted user profile data
CREATE TABLE profiles (
    id TEXT PRIMARY KEY,
    data BLOB NOT NULL,           -- Encrypted profile data (ChaCha20-Poly1305)
    nonce BLOB NOT NULL,           -- 12-byte nonce for AEAD encryption
    created_at TEXT NOT NULL,      -- RFC3339 timestamp
    updated_at TEXT NOT NULL       -- RFC3339 timestamp
);

-- Broker results table: tracks data broker scan results
CREATE TABLE broker_results (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    broker_id TEXT NOT NULL,       -- Broker identifier (e.g., "spokeo", "whitepages")
    status TEXT NOT NULL,          -- Status: "found", "not_found", "error", "pending"
    found_data_hash TEXT,          -- SHA-256 hash of found PII (for change detection)
    first_seen TEXT NOT NULL,      -- RFC3339 timestamp of first detection
    last_checked TEXT NOT NULL,    -- RFC3339 timestamp of last scan
    removal_requested_at TEXT,     -- RFC3339 timestamp when removal was requested
    removal_confirmed_at TEXT      -- RFC3339 timestamp when removal was confirmed
);

-- Audit log table: immutable log of security events
CREATE TABLE audit_log (
    id TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL,       -- RFC3339 timestamp
    event_type TEXT NOT NULL,      -- Event type: "vault_unlock", "profile_created", etc.
    detail TEXT,                   -- JSON-encoded event details
    source TEXT NOT NULL           -- Component that generated the event (e.g., "vault", "scanner")
);

-- Indexes for common query patterns
CREATE INDEX idx_broker_results_profile ON broker_results(profile_id);
CREATE INDEX idx_broker_results_broker ON broker_results(broker_id);
CREATE INDEX idx_broker_results_status ON broker_results(status);
CREATE INDEX idx_broker_results_last_checked ON broker_results(last_checked);
CREATE INDEX idx_audit_log_timestamp ON audit_log(timestamp);
CREATE INDEX idx_audit_log_event_type ON audit_log(event_type);
