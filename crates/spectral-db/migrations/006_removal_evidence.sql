-- Migration: Add removal_evidence table for screenshot evidence of browser-form removals
--
-- Stores screenshot evidence captured during browser-automated form submissions.
-- Each row captures a screenshot taken after a browser-form removal attempt,
-- providing audit evidence that the form was submitted successfully.

CREATE TABLE IF NOT EXISTS removal_evidence (
    id TEXT PRIMARY KEY NOT NULL,
    attempt_id TEXT NOT NULL REFERENCES removal_attempts(id),
    screenshot_bytes BLOB NOT NULL,
    captured_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_removal_evidence_attempt_id ON removal_evidence (attempt_id);
