CREATE TABLE IF NOT EXISTS removal_followups (
    id           TEXT PRIMARY KEY NOT NULL,
    attempt_id   TEXT NOT NULL REFERENCES removal_attempts(id) ON DELETE CASCADE,
    broker_id    TEXT NOT NULL,
    recipient    TEXT NOT NULL,      -- broker email address to follow up with
    follow_up_at TEXT NOT NULL,      -- ISO-8601: submitted_at + 15 days
    sent_at      TEXT,               -- ISO-8601: null = not yet sent or dismissed
    dismissed_at TEXT,               -- ISO-8601: null = not dismissed by user
    method       TEXT                -- 'smtp_auto' | 'user_dismissed' | null when pending
);

CREATE INDEX idx_removal_followups_attempt ON removal_followups(attempt_id);
CREATE INDEX idx_removal_followups_due
    ON removal_followups(follow_up_at)
    WHERE sent_at IS NULL AND dismissed_at IS NULL;
