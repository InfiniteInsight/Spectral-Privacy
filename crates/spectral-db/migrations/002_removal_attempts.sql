-- Migration: Add removal_attempts table
-- Tracks all removal submission attempts for audit and status

CREATE TABLE removal_attempts (
    id TEXT PRIMARY KEY,
    broker_result_id TEXT NOT NULL,
    broker_id TEXT NOT NULL,
    attempted_at TEXT NOT NULL,
    outcome_type TEXT NOT NULL,
    outcome_data TEXT,
    verification_email TEXT,
    notes TEXT,
    FOREIGN KEY (broker_result_id) REFERENCES broker_results(id) ON DELETE CASCADE
);

CREATE INDEX idx_removal_attempts_broker_result
ON removal_attempts(broker_result_id);

CREATE INDEX idx_removal_attempts_attempted_at
ON removal_attempts(attempted_at DESC);
