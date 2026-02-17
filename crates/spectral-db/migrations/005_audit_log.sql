-- Migration: Rebuild audit_log with privacy-aware schema
--
-- The original audit_log from migration 001 had a generic schema.
-- This migration replaces it with a structured schema that captures
-- vault context, PII field names (never values), data destination,
-- and access outcome for privacy auditing.

-- Drop old audit_log and its indexes
DROP INDEX IF EXISTS idx_audit_log_timestamp;
DROP INDEX IF EXISTS idx_audit_log_event_type;
DROP TABLE IF EXISTS audit_log;

-- Recreate with privacy-aware schema
CREATE TABLE IF NOT EXISTS audit_log (
    id TEXT PRIMARY KEY NOT NULL,
    vault_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL,
    subject TEXT NOT NULL,
    pii_fields TEXT,                -- JSON array of field names, never values
    data_destination TEXT NOT NULL, -- 'LocalOnly' | 'ExternalSite:domain' | 'CloudLlm:provider'
    outcome TEXT NOT NULL           -- 'Allowed' | 'Denied'
);

CREATE INDEX IF NOT EXISTS idx_audit_log_vault_id ON audit_log (vault_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_timestamp ON audit_log (timestamp);
