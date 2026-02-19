CREATE TABLE IF NOT EXISTS email_removals (
    id TEXT PRIMARY KEY NOT NULL,
    attempt_id TEXT REFERENCES removal_attempts(id),
    broker_id TEXT NOT NULL,
    sent_at TEXT NOT NULL,
    method TEXT NOT NULL,       -- 'mailto' | 'smtp'
    recipient TEXT NOT NULL,
    subject TEXT NOT NULL,
    body_hash TEXT NOT NULL
);
