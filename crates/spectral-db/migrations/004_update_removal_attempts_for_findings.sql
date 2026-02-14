-- Migration: Update removal_attempts schema for findings workflow
-- Changes removal_attempts to reference findings instead of broker_results

-- Drop the old removal_attempts table
DROP TABLE IF EXISTS removal_attempts;

-- Recreate with new schema
CREATE TABLE removal_attempts (
    id TEXT PRIMARY KEY,
    finding_id TEXT NOT NULL,
    broker_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('Pending', 'Submitted', 'Completed', 'Failed')),
    created_at TEXT NOT NULL,
    submitted_at TEXT,
    completed_at TEXT,
    error_message TEXT,
    FOREIGN KEY (finding_id) REFERENCES findings(id) ON DELETE CASCADE
);

CREATE INDEX idx_removal_attempts_finding ON removal_attempts(finding_id);
CREATE INDEX idx_removal_attempts_status ON removal_attempts(status);
CREATE INDEX idx_removal_attempts_created_at ON removal_attempts(created_at DESC);
