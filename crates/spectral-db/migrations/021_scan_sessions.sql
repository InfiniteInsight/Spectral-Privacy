-- Scan sessions track each PII scan run
CREATE TABLE IF NOT EXISTS scan_sessions (
    id TEXT PRIMARY KEY,
    vault_id TEXT NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    status TEXT NOT NULL DEFAULT 'running',
    total_files_scanned INTEGER NOT NULL DEFAULT 0,
    total_findings INTEGER NOT NULL DEFAULT 0,
    scan_config TEXT NOT NULL,
    error_message TEXT
);

CREATE INDEX IF NOT EXISTS idx_scan_sessions_vault_id ON scan_sessions(vault_id);
CREATE INDEX IF NOT EXISTS idx_scan_sessions_started_at ON scan_sessions(started_at);

-- Scan logs record every file checked
CREATE TABLE IF NOT EXISTS scan_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES scan_sessions(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    scanned_at TEXT NOT NULL,
    had_findings INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_scan_logs_session_id ON scan_logs(session_id);
