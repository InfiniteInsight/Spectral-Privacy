-- Recreate removal_attempts with nullable finding_id and new profile_id column.
-- Required so removal attempts can be created without a scan finding (e.g. email-method brokers).

-- Step 1: Create new table with desired schema
CREATE TABLE removal_attempts_new (
    id TEXT PRIMARY KEY,
    finding_id TEXT,
    profile_id TEXT,
    broker_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('Pending', 'Submitted', 'Completed', 'Failed')),
    created_at TEXT NOT NULL,
    submitted_at TEXT,
    completed_at TEXT,
    error_message TEXT,
    FOREIGN KEY (finding_id) REFERENCES findings(id) ON DELETE CASCADE
);

-- Step 2: Migrate existing rows — populate profile_id from the linked finding
INSERT INTO removal_attempts_new
    (id, finding_id, profile_id, broker_id, status, created_at, submitted_at, completed_at, error_message)
SELECT
    ra.id,
    ra.finding_id,
    f.profile_id,
    ra.broker_id,
    ra.status,
    ra.created_at,
    ra.submitted_at,
    ra.completed_at,
    ra.error_message
FROM removal_attempts ra
LEFT JOIN findings f ON f.id = ra.finding_id;

-- Step 3: Swap tables
DROP TABLE removal_attempts;
ALTER TABLE removal_attempts_new RENAME TO removal_attempts;

-- Step 4: Recreate indexes
CREATE INDEX idx_removal_attempts_finding ON removal_attempts(finding_id);
CREATE INDEX idx_removal_attempts_status ON removal_attempts(status);
CREATE INDEX idx_removal_attempts_created_at ON removal_attempts(created_at DESC);
CREATE INDEX idx_removal_attempts_profile ON removal_attempts(profile_id);
