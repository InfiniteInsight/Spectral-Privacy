-- Google Removal Requests
-- Tracks removal request URLs generated for Google search results

CREATE TABLE IF NOT EXISTS google_removal_requests (
    id TEXT PRIMARY KEY NOT NULL,
    finding_id TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL CHECK(status IN ('URLGenerated', 'Submitted', 'Completed', 'Failed')),
    google_removal_url TEXT NOT NULL,
    generated_at TEXT NOT NULL,
    submitted_at TEXT,
    completed_at TEXT,
    notes TEXT,
    FOREIGN KEY (finding_id) REFERENCES findings(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_google_removal_finding ON google_removal_requests(finding_id);
CREATE INDEX IF NOT EXISTS idx_google_removal_status ON google_removal_requests(status);
