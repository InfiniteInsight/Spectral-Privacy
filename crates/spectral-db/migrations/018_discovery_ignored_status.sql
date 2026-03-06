-- Add ignored status and still_present tracking for discovery findings
-- This allows users to distinguish between:
-- - Remediated: User claims to have fixed/removed the PII
-- - Ignored: User accepts the PII as a false positive or acceptable risk

ALTER TABLE discovery_findings ADD COLUMN ignored INTEGER NOT NULL DEFAULT 0;
ALTER TABLE discovery_findings ADD COLUMN still_present_after_remediation INTEGER NOT NULL DEFAULT 0;

-- Update the index to include ignored status for efficient filtering
CREATE INDEX IF NOT EXISTS idx_discovery_findings_status
ON discovery_findings(vault_id, remediated, ignored);
