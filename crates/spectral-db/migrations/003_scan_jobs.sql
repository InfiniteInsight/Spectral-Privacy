-- Scan jobs track overall scan operations
CREATE TABLE scan_jobs (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    status TEXT NOT NULL CHECK(status IN ('InProgress', 'Completed', 'Failed', 'Cancelled')),
    total_brokers INTEGER NOT NULL,
    completed_brokers INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    FOREIGN KEY (profile_id) REFERENCES profiles(id)
);

CREATE INDEX idx_scan_jobs_profile ON scan_jobs(profile_id, started_at DESC);

-- Individual broker scans within a job
CREATE TABLE broker_scans (
    id TEXT PRIMARY KEY,
    scan_job_id TEXT NOT NULL,
    broker_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('Pending', 'InProgress', 'Success', 'Failed', 'Skipped')),
    started_at TEXT,
    completed_at TEXT,
    error_message TEXT,
    findings_count INTEGER DEFAULT 0,
    FOREIGN KEY (scan_job_id) REFERENCES scan_jobs(id) ON DELETE CASCADE
);

CREATE INDEX idx_broker_scans_job ON broker_scans(scan_job_id);
CREATE INDEX idx_broker_scans_broker ON broker_scans(broker_id);

-- Findings are potential matches found on broker sites
CREATE TABLE findings (
    id TEXT PRIMARY KEY,
    broker_scan_id TEXT NOT NULL,
    broker_id TEXT NOT NULL,
    profile_id TEXT NOT NULL,
    listing_url TEXT NOT NULL,
    verification_status TEXT NOT NULL CHECK(verification_status IN ('PendingVerification', 'Confirmed', 'Rejected')),

    -- Extracted data from listing (encrypted)
    extracted_data TEXT NOT NULL,

    -- Metadata
    discovered_at TEXT NOT NULL,
    verified_at TEXT,
    verified_by_user BOOLEAN,

    -- Removal tracking
    removal_attempt_id TEXT,

    FOREIGN KEY (broker_scan_id) REFERENCES broker_scans(id) ON DELETE CASCADE,
    FOREIGN KEY (profile_id) REFERENCES profiles(id),
    FOREIGN KEY (removal_attempt_id) REFERENCES removal_attempts(id)
);

CREATE INDEX idx_findings_broker_scan ON findings(broker_scan_id);
CREATE INDEX idx_findings_profile ON findings(profile_id, discovered_at DESC);
CREATE INDEX idx_findings_verification_status ON findings(verification_status);
