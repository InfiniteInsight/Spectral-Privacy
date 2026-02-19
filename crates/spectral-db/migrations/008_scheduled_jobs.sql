CREATE TABLE IF NOT EXISTS scheduled_jobs (
    id TEXT PRIMARY KEY,
    job_type TEXT NOT NULL,          -- 'ScanAll' | 'VerifyRemovals' | 'PollImap'
    interval_days INTEGER NOT NULL,
    next_run_at TEXT NOT NULL,
    last_run_at TEXT,
    enabled INTEGER NOT NULL DEFAULT 1
);

-- Seed default jobs
INSERT OR IGNORE INTO scheduled_jobs (id, job_type, interval_days, next_run_at, enabled)
VALUES
    ('default-scan-all',       'ScanAll',        7, datetime('now'), 1),
    ('default-verify-removals','VerifyRemovals',  3, datetime('now'), 1);
