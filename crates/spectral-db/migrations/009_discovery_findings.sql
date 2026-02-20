-- Discovery Findings Table
-- Stores local PII discovery findings from filesystem, browser, and email scans

CREATE TABLE IF NOT EXISTS discovery_findings (
    id TEXT PRIMARY KEY,
    vault_id TEXT NOT NULL,
    source TEXT NOT NULL,           -- 'filesystem' | 'browser' | 'email'
    source_detail TEXT NOT NULL,    -- file path, browser name, or email folder
    finding_type TEXT NOT NULL,     -- 'pii_exposure' | 'broker_contact' | 'broker_account'
    risk_level TEXT NOT NULL,       -- 'critical' | 'medium' | 'informational'
    description TEXT NOT NULL,
    recommended_action TEXT,
    remediated INTEGER NOT NULL DEFAULT 0,
    found_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_discovery_findings_vault_id ON discovery_findings(vault_id);
CREATE INDEX IF NOT EXISTS idx_discovery_findings_risk_level ON discovery_findings(risk_level, remediated);
