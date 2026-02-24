-- Add pii_type column to discovery_findings table
-- This allows efficient filtering by PII type (email, phone, ssn)

-- Add nullable column first
ALTER TABLE discovery_findings ADD COLUMN pii_type TEXT;

-- Backfill existing findings by parsing description
UPDATE discovery_findings
SET pii_type = 'email'
WHERE description LIKE 'Email address%';

UPDATE discovery_findings
SET pii_type = 'phone'
WHERE description LIKE 'Phone number%';

UPDATE discovery_findings
SET pii_type = 'ssn'
WHERE description LIKE 'Social Security Number%';

-- Make column NOT NULL (safe after backfill)
-- SQLite doesn't support ADD COLUMN ... NOT NULL directly
-- so we use this workaround: create new table, copy data, swap
CREATE TABLE discovery_findings_new (
    id TEXT PRIMARY KEY,
    vault_id TEXT NOT NULL,
    source TEXT NOT NULL,
    source_detail TEXT NOT NULL,
    finding_type TEXT NOT NULL,
    risk_level TEXT NOT NULL,
    description TEXT NOT NULL,
    recommended_action TEXT,
    pii_type TEXT NOT NULL,
    remediated INTEGER NOT NULL DEFAULT 0,
    found_at TEXT NOT NULL
);

INSERT INTO discovery_findings_new
SELECT id, vault_id, source, source_detail, finding_type, risk_level,
       description, recommended_action, pii_type, remediated, found_at
FROM discovery_findings;

DROP TABLE discovery_findings;
ALTER TABLE discovery_findings_new RENAME TO discovery_findings;

-- Recreate indexes
CREATE INDEX IF NOT EXISTS idx_discovery_findings_vault_id ON discovery_findings(vault_id);
CREATE INDEX IF NOT EXISTS idx_discovery_findings_risk_level ON discovery_findings(risk_level, remediated);
