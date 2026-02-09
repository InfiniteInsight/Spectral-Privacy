## 14. Database Schema

All tables reside inside the SQLCipher-encrypted database.

```sql
-- ═══════════════════════════════════════════════════════════════
-- CORE TABLES
-- ═══════════════════════════════════════════════════════════════

CREATE TABLE profiles (
    id TEXT PRIMARY KEY,
    data BLOB NOT NULL,          -- ChaCha20-Poly1305 encrypted JSON
    nonce BLOB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE broker_results (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profiles(id),
    broker_id TEXT NOT NULL,
    status TEXT NOT NULL,         -- 'found', 'removal_requested', 'removed', 'reappeared', 'error'
    found_data_hash TEXT,         -- hash of what was found (not the data itself)
    screenshot_path TEXT,         -- encrypted screenshot for evidence
    first_seen TEXT NOT NULL,
    last_checked TEXT NOT NULL,
    removal_requested_at TEXT,
    removal_confirmed_at TEXT,
    metadata BLOB                -- encrypted broker-specific metadata
);

CREATE TABLE removal_actions (
    id TEXT PRIMARY KEY,
    broker_result_id TEXT NOT NULL REFERENCES broker_results(id),
    action_type TEXT NOT NULL,    -- 'form_submit', 'email_sent', 'api_call', 'manual'
    status TEXT NOT NULL,         -- 'pending', 'in_progress', 'completed', 'failed', 'needs_verification'
    attempt_number INTEGER NOT NULL DEFAULT 1,
    executed_at TEXT NOT NULL,
    response_summary TEXT,
    error_detail TEXT,
    next_retry_at TEXT
);

CREATE TABLE scan_history (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profiles(id),
    scan_type TEXT NOT NULL,      -- 'full', 'targeted', 'recheck'
    started_at TEXT NOT NULL,
    completed_at TEXT,
    brokers_scanned INTEGER,
    results_found INTEGER,
    status TEXT NOT NULL
);

CREATE TABLE audit_log (
    id TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL,
    detail TEXT,                  -- never contains raw PII
    source TEXT NOT NULL          -- 'user', 'system', 'plugin', 'llm'
);

-- ═══════════════════════════════════════════════════════════════
-- NETWORK MONITORING TABLES
-- ═══════════════════════════════════════════════════════════════

CREATE TABLE netmon_alert_rules (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    rule_type TEXT NOT NULL,              -- 'new_broker_contact', 'threshold', 'pattern'
    category_filter TEXT,                 -- which domain categories trigger this
    threshold_value REAL,                 -- for threshold-based rules
    enabled INTEGER NOT NULL DEFAULT 1,
    notify_method TEXT NOT NULL,          -- 'dashboard', 'desktop_notification', 'email'
    created_at TEXT NOT NULL
);

CREATE TABLE netmon_alerts (
    id TEXT PRIMARY KEY,
    rule_id TEXT REFERENCES netmon_alert_rules(id),
    triggered_at TEXT NOT NULL,
    title TEXT NOT NULL,
    detail TEXT NOT NULL,
    severity TEXT NOT NULL,
    acknowledged INTEGER NOT NULL DEFAULT 0,
    acknowledged_at TEXT
);

-- ═══════════════════════════════════════════════════════════════
-- EMAIL THREAD TRACKING
-- ═══════════════════════════════════════════════════════════════

CREATE TABLE email_threads (
    id TEXT PRIMARY KEY,
    broker_result_id TEXT NOT NULL REFERENCES broker_results(id),
    broker_id TEXT NOT NULL,
    broker_email TEXT NOT NULL,
    subject TEXT NOT NULL,
    status TEXT NOT NULL,
    auto_reply_count INTEGER NOT NULL DEFAULT 0,
    llm_call_count INTEGER NOT NULL DEFAULT 0,
    tokens_used INTEGER NOT NULL DEFAULT 0,
    budget_config TEXT NOT NULL,           -- JSON: ThreadBudget
    created_at TEXT NOT NULL,
    last_activity TEXT NOT NULL,
    frozen_at TEXT,                        -- set when reply limit reached
    frozen_reason TEXT
);

CREATE TABLE email_messages (
    id TEXT PRIMARY KEY,
    thread_id TEXT NOT NULL REFERENCES email_threads(id),
    direction TEXT NOT NULL,              -- 'outbound', 'inbound'
    timestamp TEXT NOT NULL,
    from_address TEXT NOT NULL,
    to_address TEXT NOT NULL,
    subject TEXT NOT NULL,
    body_encrypted BLOB NOT NULL,         -- encrypted email body
    body_nonce BLOB NOT NULL,
    classification TEXT,                  -- JSON: ResponseClassification
    was_auto_generated INTEGER NOT NULL DEFAULT 0,
    llm_tokens_used INTEGER,
    user_approved INTEGER NOT NULL DEFAULT 0,
    safety_flags TEXT                     -- JSON: any risk indicators from safety layer
);

-- ═══════════════════════════════════════════════════════════════
-- VERIFICATION TRACKING
-- ═══════════════════════════════════════════════════════════════

CREATE TABLE verification_schedules (
    id TEXT PRIMARY KEY,
    broker_result_id TEXT NOT NULL REFERENCES broker_results(id),
    broker_id TEXT NOT NULL,
    removal_requested_at TEXT NOT NULL,
    expected_completion TEXT NOT NULL,
    legal_regulation TEXT NOT NULL,
    legal_deadline TEXT NOT NULL,
    status TEXT NOT NULL,
    escalation_level INTEGER NOT NULL DEFAULT 0,
    last_checked TEXT,
    next_check TEXT,
    check_history TEXT NOT NULL           -- JSON: Vec<VerificationCheck>
);

-- ═══════════════════════════════════════════════════════════════
-- CROSS-CORRELATION INSIGHTS
-- ═══════════════════════════════════════════════════════════════

CREATE TABLE insights (
    id TEXT PRIMARY KEY,
    generated_at TEXT NOT NULL,
    severity TEXT NOT NULL,
    category TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    evidence TEXT NOT NULL,               -- JSON: Vec<InsightEvidence>
    suggested_actions TEXT NOT NULL,      -- JSON: Vec<SuggestedAction>
    acknowledged INTEGER NOT NULL DEFAULT 0,
    acknowledged_at TEXT,
    acted_on INTEGER NOT NULL DEFAULT 0
);
```

---
