-- Add index for discovery findings deduplication
-- This index speeds up the check for existing findings when preventing duplicates
-- during PII scans. It's used to find findings matching (vault_id, source_detail, pii_type).

CREATE INDEX IF NOT EXISTS idx_discovery_findings_dedup
ON discovery_findings(vault_id, source_detail, pii_type);
