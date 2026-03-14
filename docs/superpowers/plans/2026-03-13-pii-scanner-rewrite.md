# PII Scanner Rewrite Implementation Plan

> **For agentic workers:** This is a living state machine. Update status fields as you complete work. Each task includes context setup for fresh sessions and handoff instructions for the next task.

**Goal:** Rebuild the PII Scanner with proper thread architecture, user-selectable PII types, and comprehensive finding management.

**Architecture:** Thread pool (rayon) for filesystem traversal, channels for progress, database for session/log tracking.

**Tech Stack:** Rust (rayon, crossbeam, walkdir), SQLite, Svelte 5, Tauri

---

## Plan Status Dashboard

| Phase | Status | Tasks | Notes |
|-------|--------|-------|-------|
| Phase 1: Database Schema | `COMPLETE` | 3 | Migration + DB functions |
| Phase 2: Scanner Core | `COMPLETE` | 3 | Types, patterns, engine |
| Phase 3: Tauri Integration | `COMPLETE` | 2 | Commands + wiring |
| Phase 4: Frontend Components | `NOT_STARTED` | 4 | UI components |
| Phase 5: Main Page | `NOT_STARTED` | 1 | Page rewrite |
| Phase 6: Testing | `NOT_STARTED` | 1 | Integration tests |

**Current Task:** `Phase 4 / Task 4.1 - API Types and PII Explainer`
**Last Updated:** `2026-03-13`
**Blocking Issues:** None

---

## Phase 1: Database Schema and Storage

**Phase Goal:** Add database tables for scan sessions and logs, plus the Rust functions to interact with them.

**Phase Status:** `COMPLETE`

---

### Task 1.1: Database Migration

**Status:** `COMPLETE`
**Estimated Context:** Small (single file)
**Dependencies:** None

#### Context Setup
If starting fresh, you're working on the Spectral app - a Tauri + Svelte privacy tool. This task adds a database migration for tracking PII scan sessions.

**Key files to know exist:**
- `crates/spectral-db/migrations/` - SQLx migrations folder
- Migrations use format `NNN_description.sql`
- Latest migration number: Check folder, likely ~024

#### Steps

- [ ] **Step 1:** Check latest migration number
```bash
ls crates/spectral-db/migrations/ | tail -5
```

- [ ] **Step 2:** Create migration file `crates/spectral-db/migrations/025_scan_sessions.sql`

```sql
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
```

- [ ] **Step 3:** Verify file exists
```bash
cat crates/spectral-db/migrations/025_scan_sessions.sql
```

- [ ] **Step 4:** Commit
```bash
git add crates/spectral-db/migrations/025_scan_sessions.sql
git commit -m "feat(db): add scan_sessions and scan_logs tables"
```

#### Task Completion
- [ ] Migration file created
- [ ] Committed to git

**Status on completion:** Update to `COMPLETE`

#### Handoff to Task 1.2
Next task creates Rust functions to interact with these tables. The key types are:
- `ScanConfig` - JSON blob storing which PII types to scan
- `ScanSession` - Tracks a scan run with status, counts
- Scan logs - One row per file scanned

---

### Task 1.2: Scan Logs Database Module

**Status:** `IN_PROGRESS`
**Estimated Context:** Medium (one new file, one small edit)
**Dependencies:** Task 1.1

#### Context Setup
The migration from Task 1.1 created `scan_sessions` and `scan_logs` tables. Now we need Rust functions to:
- Create/update scan sessions
- Batch insert scanned files
- Retrieve logs for download

**Key patterns from codebase:**
- Use `sqlx::query()` with `?` bind parameters
- Return `Result<T, sqlx::Error>`
- Timestamps use RFC3339 format

#### Steps

- [ ] **Step 1:** Create `crates/spectral-db/src/scan_logs.rs`

```rust
//! Scan session and log management for PII discovery

use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScanConfig {
    pub scan_emails: bool,
    pub scan_phones: bool,
    pub scan_ssn: bool,
    pub scan_addresses: bool,
    pub scan_names: bool,
    pub scan_dob: bool,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            scan_emails: true,
            scan_phones: true,
            scan_ssn: true,
            scan_addresses: true,
            scan_names: true,
            scan_dob: true,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScanSession {
    pub id: String,
    pub vault_id: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub status: String,
    pub total_files_scanned: i64,
    pub total_findings: i64,
    pub scan_config: ScanConfig,
    pub error_message: Option<String>,
}

pub async fn create_scan_session(
    pool: &SqlitePool,
    vault_id: &str,
    config: &ScanConfig,
) -> Result<String, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let started_at = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();
    let config_json = serde_json::to_string(config).unwrap_or_default();

    sqlx::query(
        "INSERT INTO scan_sessions (id, vault_id, started_at, status, scan_config) VALUES (?, ?, ?, 'running', ?)",
    )
    .bind(&id)
    .bind(vault_id)
    .bind(&started_at)
    .bind(&config_json)
    .execute(pool)
    .await?;

    Ok(id)
}

pub async fn update_scan_session(
    pool: &SqlitePool,
    session_id: &str,
    status: &str,
    files_scanned: i64,
    findings_count: i64,
    error_message: Option<&str>,
) -> Result<(), sqlx::Error> {
    let completed_at = if status != "running" {
        Some(
            OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
        )
    } else {
        None
    };

    sqlx::query(
        "UPDATE scan_sessions SET status = ?, total_files_scanned = ?, total_findings = ?, completed_at = ?, error_message = ? WHERE id = ?",
    )
    .bind(status)
    .bind(files_scanned)
    .bind(findings_count)
    .bind(completed_at)
    .bind(error_message)
    .bind(session_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn log_scanned_files_batch(
    pool: &SqlitePool,
    session_id: &str,
    files: &[(String, bool)],
) -> Result<(), sqlx::Error> {
    if files.is_empty() {
        return Ok(());
    }

    let scanned_at = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();

    for (path, had_findings) in files {
        sqlx::query(
            "INSERT INTO scan_logs (session_id, file_path, scanned_at, had_findings) VALUES (?, ?, ?, ?)",
        )
        .bind(session_id)
        .bind(path)
        .bind(&scanned_at)
        .bind(*had_findings as i32)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn get_scan_log(
    pool: &SqlitePool,
    session_id: &str,
) -> Result<Vec<(String, String, bool)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String, i32)>(
        "SELECT file_path, scanned_at, had_findings FROM scan_logs WHERE session_id = ? ORDER BY id ASC",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(p, t, f)| (p, t, f != 0)).collect())
}

pub async fn get_latest_scan_session(
    pool: &SqlitePool,
    vault_id: &str,
) -> Result<Option<ScanSession>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String, String, Option<String>, String, i64, i64, String, Option<String>)>(
        "SELECT id, vault_id, started_at, completed_at, status, total_files_scanned, total_findings, scan_config, error_message FROM scan_sessions WHERE vault_id = ? ORDER BY started_at DESC LIMIT 1",
    )
    .bind(vault_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, vault_id, started_at, completed_at, status, files, findings, config_json, error)| {
        ScanSession {
            id,
            vault_id,
            started_at,
            completed_at,
            status,
            total_files_scanned: files,
            total_findings: findings,
            scan_config: serde_json::from_str(&config_json).unwrap_or_default(),
            error_message: error,
        }
    }))
}
```

- [ ] **Step 2:** Add export to `crates/spectral-db/src/lib.rs`

Add this line:
```rust
pub mod scan_logs;
```

- [ ] **Step 3:** Verify compilation
```bash
cargo check -p spectral-db
```

- [ ] **Step 4:** Commit
```bash
git add crates/spectral-db/src/scan_logs.rs crates/spectral-db/src/lib.rs
git commit -m "feat(db): add scan_logs module"
```

#### Task Completion
- [ ] Module created with all functions
- [ ] Exported from lib.rs
- [ ] Compiles successfully
- [ ] Committed

**Status on completion:** Update to `COMPLETE`

#### Handoff to Task 1.3
Phase 1 complete after this task. Phase 2 creates the scanner core library. The `ScanConfig` type defined here will be used by the scanner to know which PII types to search for.

---

### Task 1.3: Verify Phase 1 Integration

**Status:** `NOT_STARTED`
**Estimated Context:** Small (verification only)
**Dependencies:** Tasks 1.1, 1.2

#### Context Setup
Phase 1 added database schema and Rust module. This task verifies everything works together.

#### Steps

- [ ] **Step 1:** Run full cargo check
```bash
cargo check --workspace
```

- [ ] **Step 2:** Verify exports available
```bash
cargo doc -p spectral-db --no-deps 2>&1 | head -20
```

- [ ] **Step 3:** Update plan status
Update Phase 1 status to `COMPLETE` in the dashboard above.

#### Task Completion
- [ ] Full workspace compiles
- [ ] Phase 1 marked complete

#### Handoff to Phase 2
Phase 2 creates the scanner core library in `spectral-discovery` crate:
- Task 2.1: Types module (PiiType, ScanConfig, etc.)
- Task 2.2: Pattern matching engine
- Task 2.3: Thread-pool scanner

The key insight: we use `rayon` for parallel file scanning on dedicated threads, NOT tokio async. This prevents blocking the Tauri runtime.

---

## Phase 2: Scanner Core Library

**Phase Goal:** Build the PII scanning engine with thread-pool architecture.

**Phase Status:** `NOT_STARTED`

**Key Design Decisions:**
- Use `rayon` thread pool for filesystem traversal (not tokio)
- Use `crossbeam` channels for progress communication
- Use `walkdir` for efficient directory traversal
- All file I/O is synchronous on rayon threads

---

### Task 2.1: Discovery Types Module

**Status:** `NOT_STARTED`
**Estimated Context:** Medium (new file)
**Dependencies:** Phase 1 complete

#### Context Setup
The `spectral-discovery` crate currently has a `filesystem.rs` that we'll replace. This task creates the new types module that defines all shared types.

**Crate location:** `crates/spectral-discovery/`

#### Steps

- [ ] **Step 1:** Create `crates/spectral-discovery/src/types.rs`

```rust
//! Shared types for PII discovery scanning

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    pub scan_emails: bool,
    pub scan_phones: bool,
    pub scan_ssn: bool,
    pub scan_addresses: bool,
    pub scan_names: bool,
    pub scan_dob: bool,
    pub custom_directories: Option<Vec<PathBuf>>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            scan_emails: true,
            scan_phones: true,
            scan_ssn: true,
            scan_addresses: true,
            scan_names: true,
            scan_dob: true,
            custom_directories: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct UserPii {
    pub emails: Vec<String>,
    pub phones: Vec<String>,
    pub ssn: Option<String>,
    pub addresses: Vec<AddressInfo>,
    pub names: Vec<String>,
    pub date_of_birth: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AddressInfo {
    pub street: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PiiType {
    Email,
    Phone,
    Ssn,
    Address,
    Name,
    DateOfBirth,
}

impl PiiType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PiiType::Email => "email",
            PiiType::Phone => "phone",
            PiiType::Ssn => "ssn",
            PiiType::Address => "address",
            PiiType::Name => "name",
            PiiType::DateOfBirth => "dob",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            PiiType::Email => "Email Address",
            PiiType::Phone => "Phone Number",
            PiiType::Ssn => "Social Security Number",
            PiiType::Address => "Physical Address",
            PiiType::Name => "Personal Name",
            PiiType::DateOfBirth => "Date of Birth",
        }
    }

    pub fn risk_level(&self) -> &'static str {
        match self {
            PiiType::Ssn => "critical",
            PiiType::DateOfBirth => "high",
            PiiType::Address | PiiType::Phone | PiiType::Email => "medium",
            PiiType::Name => "low",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiMatch {
    pub pii_type: PiiType,
    pub matched_value: String,
    pub line_number: usize,
    pub line_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileScanResult {
    pub path: PathBuf,
    pub matches: Vec<PiiMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub files_scanned: usize,
    pub files_with_findings: usize,
    pub current_directory: String,
    pub is_complete: bool,
    pub was_stopped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanCommand {
    Continue,
    Pause,
    Stop,
}

pub const SCANNABLE_EXTENSIONS: &[&str] = &[
    "txt", "csv", "json", "md", "log", "xml", "yaml", "yml",
    "ini", "cfg", "conf", "properties", "toml",
    "html", "htm", "js", "ts", "jsx", "tsx",
    "py", "rb", "php", "java", "c", "cpp", "h",
    "rs", "go", "swift", "sql", "sh", "bash", "ps1",
];

pub const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB
pub const MAX_SCAN_DEPTH: usize = 15;
```

- [ ] **Step 2:** Verify compilation
```bash
cargo check -p spectral-discovery
```

- [ ] **Step 3:** Commit
```bash
git add crates/spectral-discovery/src/types.rs
git commit -m "feat(discovery): add types module"
```

#### Task Completion
- [ ] Types module created
- [ ] Compiles
- [ ] Committed

#### Handoff to Task 2.2
Next task creates the pattern matching engine. Key types it will use:
- `UserPii` - The user's PII data to search for
- `ScanConfig` - Which types are enabled
- `PiiMatch` - A single match found
- `PiiType` - The enum of PII categories

---

### Task 2.2: Pattern Matching Engine

**Status:** `NOT_STARTED`
**Estimated Context:** Large (complex logic, tests)
**Dependencies:** Task 2.1

#### Context Setup
This creates the pattern matching engine that searches text for user-specific PII. It compiles regex patterns from the user's data and efficiently searches text.

**Key design:**
- Case-insensitive email matching
- Phone number normalization (handle various formats)
- SSN masking for display security
- Address matching (street OR zip)

#### Steps

- [ ] **Step 1:** Create `crates/spectral-discovery/src/patterns.rs`

```rust
//! PII pattern matching engine

use crate::types::{AddressInfo, PiiMatch, PiiType, ScanConfig, UserPii};
use regex::{Regex, RegexBuilder};

pub struct PiiPatterns {
    email_patterns: Vec<(String, Regex)>,
    phone_patterns: Vec<(String, Vec<Regex>)>,
    ssn_pattern: Option<(String, Regex)>,
    address_patterns: Vec<AddressPattern>,
    name_patterns: Vec<(String, Regex)>,
    dob_pattern: Option<(String, Regex)>,
    config: ScanConfig,
}

struct AddressPattern {
    original: AddressInfo,
    street_regex: Option<Regex>,
    zip_regex: Option<Regex>,
}

impl PiiPatterns {
    pub fn new(user_pii: &UserPii, config: &ScanConfig) -> Self {
        let mut patterns = Self {
            email_patterns: Vec::new(),
            phone_patterns: Vec::new(),
            ssn_pattern: None,
            address_patterns: Vec::new(),
            name_patterns: Vec::new(),
            dob_pattern: None,
            config: config.clone(),
        };

        if config.scan_emails {
            patterns.compile_emails(&user_pii.emails);
        }
        if config.scan_phones {
            patterns.compile_phones(&user_pii.phones);
        }
        if config.scan_ssn {
            if let Some(ssn) = &user_pii.ssn {
                patterns.compile_ssn(ssn);
            }
        }
        if config.scan_addresses {
            patterns.compile_addresses(&user_pii.addresses);
        }
        if config.scan_names {
            patterns.compile_names(&user_pii.names);
        }
        if config.scan_dob {
            if let Some(dob) = &user_pii.date_of_birth {
                patterns.compile_dob(dob);
            }
        }

        patterns
    }

    fn compile_emails(&mut self, emails: &[String]) {
        for email in emails {
            if let Ok(regex) = RegexBuilder::new(&regex::escape(&email.to_lowercase()))
                .case_insensitive(true)
                .build()
            {
                self.email_patterns.push((email.clone(), regex));
            }
        }
    }

    fn compile_phones(&mut self, phones: &[String]) {
        for phone in phones {
            let normalized: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
            if normalized.len() >= 10 {
                let mut regexes = Vec::new();
                let (area, prefix, line) = (&normalized[0..3], &normalized[3..6], &normalized[6..10]);

                for pattern in [
                    format!(r"\({}\)\s*{}-{}", area, prefix, line),
                    format!(r"{}-{}-{}", area, prefix, line),
                    format!(r"{}\.{}\.{}", area, prefix, line),
                    normalized.clone(),
                ] {
                    if let Ok(regex) = Regex::new(&pattern) {
                        regexes.push(regex);
                    }
                }

                if !regexes.is_empty() {
                    self.phone_patterns.push((phone.clone(), regexes));
                }
            }
        }
    }

    fn compile_ssn(&mut self, ssn: &str) {
        let normalized: String = ssn.chars().filter(|c| c.is_ascii_digit()).collect();
        if normalized.len() == 9 {
            let pattern = format!(
                r"{}[-\s]?{}[-\s]?{}",
                &normalized[0..3], &normalized[3..5], &normalized[5..9]
            );
            if let Ok(regex) = Regex::new(&pattern) {
                self.ssn_pattern = Some((ssn.to_string(), regex));
            }
        }
    }

    fn compile_addresses(&mut self, addresses: &[AddressInfo]) {
        for addr in addresses {
            let street_regex = addr.street.as_ref().and_then(|s| {
                if s.len() >= 5 {
                    RegexBuilder::new(&regex::escape(s)).case_insensitive(true).build().ok()
                } else {
                    None
                }
            });

            let zip_regex = addr.zip.as_ref().and_then(|z| {
                let normalized: String = z.chars().filter(|c| c.is_ascii_digit()).collect();
                if normalized.len() >= 5 {
                    Regex::new(&normalized).ok()
                } else {
                    None
                }
            });

            if street_regex.is_some() || zip_regex.is_some() {
                self.address_patterns.push(AddressPattern {
                    original: addr.clone(),
                    street_regex,
                    zip_regex,
                });
            }
        }
    }

    fn compile_names(&mut self, names: &[String]) {
        for name in names {
            if name.len() >= 3 {
                if let Ok(regex) = RegexBuilder::new(&format!(r"\b{}\b", regex::escape(name)))
                    .case_insensitive(true)
                    .build()
                {
                    self.name_patterns.push((name.clone(), regex));
                }
            }
        }
    }

    fn compile_dob(&mut self, dob: &str) {
        if let Some((month, day, year)) = parse_date(dob) {
            let pattern = format!(
                r"{:02}/{:02}/{}|{:02}-{:02}-{}|{}-{:02}-{:02}",
                month, day, year, month, day, year, year, month, day
            );
            if let Ok(regex) = Regex::new(&pattern) {
                self.dob_pattern = Some((dob.to_string(), regex));
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.email_patterns.is_empty()
            && self.phone_patterns.is_empty()
            && self.ssn_pattern.is_none()
            && self.address_patterns.is_empty()
            && self.name_patterns.is_empty()
            && self.dob_pattern.is_none()
    }

    pub fn find_all(&self, text: &str) -> Vec<PiiMatch> {
        let mut matches = Vec::new();

        for (line_num, line) in text.lines().enumerate() {
            let line_number = line_num + 1;

            for (original, regex) in &self.email_patterns {
                if regex.is_match(line) {
                    matches.push(PiiMatch {
                        pii_type: PiiType::Email,
                        matched_value: original.clone(),
                        line_number,
                        line_content: truncate(line, 200),
                    });
                }
            }

            for (original, regexes) in &self.phone_patterns {
                if regexes.iter().any(|r| r.is_match(line)) {
                    matches.push(PiiMatch {
                        pii_type: PiiType::Phone,
                        matched_value: original.clone(),
                        line_number,
                        line_content: truncate(line, 200),
                    });
                    break;
                }
            }

            if let Some((original, regex)) = &self.ssn_pattern {
                if regex.is_match(line) {
                    matches.push(PiiMatch {
                        pii_type: PiiType::Ssn,
                        matched_value: mask_ssn(original),
                        line_number,
                        line_content: truncate(line, 200),
                    });
                }
            }

            for addr in &self.address_patterns {
                let street_match = addr.street_regex.as_ref().map(|r| r.is_match(line)).unwrap_or(false);
                let zip_match = addr.zip_regex.as_ref().map(|r| r.is_match(line)).unwrap_or(false);
                if street_match || zip_match {
                    matches.push(PiiMatch {
                        pii_type: PiiType::Address,
                        matched_value: format_address(&addr.original),
                        line_number,
                        line_content: truncate(line, 200),
                    });
                }
            }

            for (original, regex) in &self.name_patterns {
                if regex.is_match(line) {
                    matches.push(PiiMatch {
                        pii_type: PiiType::Name,
                        matched_value: original.clone(),
                        line_number,
                        line_content: truncate(line, 200),
                    });
                }
            }

            if let Some((original, regex)) = &self.dob_pattern {
                if regex.is_match(line) {
                    matches.push(PiiMatch {
                        pii_type: PiiType::DateOfBirth,
                        matched_value: original.clone(),
                        line_number,
                        line_content: truncate(line, 200),
                    });
                }
            }
        }

        // Dedupe same type on same line
        matches.sort_by(|a, b| a.line_number.cmp(&b.line_number).then(a.pii_type.as_str().cmp(b.pii_type.as_str())));
        matches.dedup_by(|a, b| a.line_number == b.line_number && a.pii_type == b.pii_type);
        matches
    }
}

fn parse_date(date: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = date.split(['/', '-', '.']).collect();
    if parts.len() == 3 {
        let nums: Vec<u32> = parts.iter().filter_map(|p| p.parse().ok()).collect();
        if nums.len() == 3 {
            if nums[0] > 31 { return Some((nums[1], nums[2], nums[0])); }
            if nums[2] > 31 { return Some((nums[0], nums[1], nums[2])); }
        }
    }
    None
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max]) }
}

fn mask_ssn(ssn: &str) -> String {
    let digits: String = ssn.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() >= 4 {
        format!("***-**-{}", &digits[digits.len() - 4..])
    } else {
        "***-**-****".to_string()
    }
}

fn format_address(addr: &AddressInfo) -> String {
    [addr.street.as_deref(), addr.city.as_deref(), addr.state.as_deref(), addr.zip.as_deref()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_matching() {
        let pii = UserPii { emails: vec!["test@example.com".into()], ..Default::default() };
        let patterns = PiiPatterns::new(&pii, &ScanConfig::default());
        let matches = patterns.find_all("Contact: test@example.com");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pii_type, PiiType::Email);
    }

    #[test]
    fn test_phone_matching() {
        let pii = UserPii { phones: vec!["555-123-4567".into()], ..Default::default() };
        let patterns = PiiPatterns::new(&pii, &ScanConfig::default());
        let matches = patterns.find_all("Call (555) 123-4567");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pii_type, PiiType::Phone);
    }

    #[test]
    fn test_ssn_masking() {
        assert_eq!(mask_ssn("123-45-6789"), "***-**-6789");
    }
}
```

- [ ] **Step 2:** Verify compilation and tests
```bash
cargo test -p spectral-discovery patterns
```

- [ ] **Step 3:** Commit
```bash
git add crates/spectral-discovery/src/patterns.rs
git commit -m "feat(discovery): add pattern matching engine"
```

#### Task Completion
- [ ] Patterns module created
- [ ] Tests pass
- [ ] Committed

#### Handoff to Task 2.3
Next task creates the scanner engine that:
- Uses rayon thread pool for parallel file scanning
- Uses walkdir for directory traversal
- Communicates progress via crossbeam channels
- Calls into the `PiiPatterns` to find matches

---

### Task 2.3: Thread-Pool Scanner Engine

**Status:** `NOT_STARTED`
**Estimated Context:** Large (complex, dependencies)
**Dependencies:** Tasks 2.1, 2.2

#### Context Setup
This is the core scanner that runs on a thread pool. Key architecture:
- `Scanner::scan()` runs on rayon threads (NOT tokio)
- Progress sent via crossbeam channel
- Commands (pause/stop) received via channel
- Returns `ScanResult` with all findings

**Must add dependencies to Cargo.toml first.**

#### Steps

- [ ] **Step 1:** Update `crates/spectral-discovery/Cargo.toml` dependencies

Add under `[dependencies]`:
```toml
rayon = "1.10"
crossbeam-channel = "0.5"
walkdir = "2.5"
```

- [ ] **Step 2:** Create `crates/spectral-discovery/src/scanner.rs`

```rust
//! Thread-pool based PII scanner

use crate::patterns::PiiPatterns;
use crate::types::*;
use crossbeam_channel::{bounded, Receiver, Sender};
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;

const EXCLUDED_DIRS: &[&str] = &[
    "AppData", ".cache", "node_modules", ".git", "target", "build",
    "Windows", "Program Files", "ProgramData", "$Recycle.Bin",
    ".npm", ".cargo", ".rustup", "Temp", "tmp", "venv", ".venv",
    ".vscode", ".idea", "Steam", "Games", "Videos", ".docker",
];

#[derive(Debug)]
pub struct ScanResult {
    pub files_scanned: usize,
    pub findings: Vec<FileScanResult>,
    pub was_stopped: bool,
}

pub struct Scanner {
    patterns: PiiPatterns,
    ignored_paths: HashSet<String>,
    command_rx: Receiver<ScanCommand>,
    progress_tx: Sender<ScanProgress>,
    stop_flag: Arc<AtomicBool>,
    files_scanned: Arc<AtomicUsize>,
    files_with_findings: Arc<AtomicUsize>,
}

impl Scanner {
    pub fn new(
        user_pii: UserPii,
        config: ScanConfig,
        ignored_paths: HashSet<String>,
        command_rx: Receiver<ScanCommand>,
        progress_tx: Sender<ScanProgress>,
    ) -> Self {
        Self {
            patterns: PiiPatterns::new(&user_pii, &config),
            ignored_paths,
            command_rx,
            progress_tx,
            stop_flag: Arc::new(AtomicBool::new(false)),
            files_scanned: Arc::new(AtomicUsize::new(0)),
            files_with_findings: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn scan(self, directories: Vec<PathBuf>) -> ScanResult {
        if self.patterns.is_empty() {
            return ScanResult { files_scanned: 0, findings: Vec::new(), was_stopped: false };
        }

        let files = self.collect_files(&directories);

        let findings: Vec<FileScanResult> = files
            .par_iter()
            .filter_map(|path| {
                if self.stop_flag.load(Ordering::Relaxed) { return None; }
                self.check_commands();

                let result = self.scan_file(path);
                let count = self.files_scanned.fetch_add(1, Ordering::Relaxed) + 1;

                if result.is_some() {
                    self.files_with_findings.fetch_add(1, Ordering::Relaxed);
                }

                if count % 100 == 0 {
                    let _ = self.progress_tx.try_send(ScanProgress {
                        files_scanned: count,
                        files_with_findings: self.files_with_findings.load(Ordering::Relaxed),
                        current_directory: path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("").to_string(),
                        is_complete: false,
                        was_stopped: false,
                    });
                }

                result
            })
            .collect();

        let was_stopped = self.stop_flag.load(Ordering::Relaxed);
        let files_scanned = self.files_scanned.load(Ordering::Relaxed);

        let _ = self.progress_tx.try_send(ScanProgress {
            files_scanned,
            files_with_findings: self.files_with_findings.load(Ordering::Relaxed),
            current_directory: String::new(),
            is_complete: true,
            was_stopped,
        });

        ScanResult { files_scanned, findings, was_stopped }
    }

    fn collect_files(&self, directories: &[PathBuf]) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for dir in directories {
            if !dir.exists() { continue; }

            for entry in WalkDir::new(dir)
                .max_depth(MAX_SCAN_DEPTH)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| !self.should_skip(e.path()))
            {
                if self.stop_flag.load(Ordering::Relaxed) { break; }
                self.check_commands();

                if let Ok(entry) = entry {
                    let path = entry.path();
                    if self.ignored_paths.contains(&path.to_string_lossy().to_string()) { continue; }
                    if path.is_file() && self.is_scannable(path) {
                        files.push(path.to_path_buf());
                    }
                }
            }
        }
        files
    }

    fn should_skip(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|name| EXCLUDED_DIRS.iter().any(|d| d.eq_ignore_ascii_case(name)))
            .unwrap_or(false)
    }

    fn is_scannable(&self, path: &Path) -> bool {
        let ext_ok = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| SCANNABLE_EXTENSIONS.contains(&e.to_lowercase().as_str()))
            .unwrap_or(false);

        if !ext_ok { return false; }
        fs::metadata(path).map(|m| m.len() <= MAX_FILE_SIZE).unwrap_or(false)
    }

    fn scan_file(&self, path: &Path) -> Option<FileScanResult> {
        let content = fs::read_to_string(path).ok()?;
        let matches = self.patterns.find_all(&content);
        if matches.is_empty() { None }
        else { Some(FileScanResult { path: path.to_path_buf(), matches }) }
    }

    fn check_commands(&self) {
        while let Ok(cmd) = self.command_rx.try_recv() {
            match cmd {
                ScanCommand::Stop => { self.stop_flag.store(true, Ordering::Relaxed); }
                ScanCommand::Pause => {
                    while !self.stop_flag.load(Ordering::Relaxed) {
                        match self.command_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                            Ok(ScanCommand::Continue) => break,
                            Ok(ScanCommand::Stop) => { self.stop_flag.store(true, Ordering::Relaxed); break; }
                            _ => {}
                        }
                    }
                }
                ScanCommand::Continue => {}
            }
        }
    }
}

pub fn create_scanner_channels() -> (Sender<ScanCommand>, Receiver<ScanCommand>, Sender<ScanProgress>, Receiver<ScanProgress>) {
    let (cmd_tx, cmd_rx) = bounded(10);
    let (progress_tx, progress_rx) = bounded(100);
    (cmd_tx, cmd_rx, progress_tx, progress_rx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_scanner_finds_email() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("test.txt");
        fs::write(&file, "Contact: john@example.com").unwrap();

        let pii = UserPii { emails: vec!["john@example.com".into()], ..Default::default() };
        let (cmd_tx, cmd_rx, progress_tx, _) = create_scanner_channels();
        drop(cmd_tx);

        let scanner = Scanner::new(pii, ScanConfig::default(), HashSet::new(), cmd_rx, progress_tx);
        let result = scanner.scan(vec![temp.path().to_path_buf()]);

        assert_eq!(result.files_scanned, 1);
        assert_eq!(result.findings.len(), 1);
    }
}
```

- [ ] **Step 3:** Update `crates/spectral-discovery/src/lib.rs`

Replace contents:
```rust
//! Spectral Discovery - PII scanning library

pub mod patterns;
pub mod scanner;
pub mod types;

pub use patterns::PiiPatterns;
pub use scanner::{create_scanner_channels, Scanner, ScanResult};
pub use types::*;
```

- [ ] **Step 4:** Delete old filesystem.rs
```bash
rm -f crates/spectral-discovery/src/filesystem.rs
```

- [ ] **Step 5:** Verify build and tests
```bash
cargo test -p spectral-discovery
```

- [ ] **Step 6:** Commit
```bash
git add crates/spectral-discovery/
git commit -m "feat(discovery): add thread-pool scanner engine"
```

#### Task Completion
- [ ] Scanner module created
- [ ] Dependencies added
- [ ] Old filesystem.rs removed
- [ ] Tests pass
- [ ] Committed

#### Handoff to Phase 3
Phase 2 complete! The scanner library is ready. Phase 3 integrates it with Tauri:
- Task 3.1: Rewrite Tauri commands to use new scanner
- Task 3.2: Wire up commands in main.rs if needed

Key integration points:
- `Scanner::new()` takes UserPii (from vault profile), ScanConfig, ignored paths, channels
- `scanner.scan(directories)` runs the scan and returns `ScanResult`
- Progress comes via the progress channel
- Commands (pause/stop) go via command channel

---

## Phase 3: Tauri Integration

**Phase Goal:** Connect the scanner library to Tauri commands.

**Phase Status:** `COMPLETE`

---

### Task 3.1: Rewrite Discovery Commands

**Status:** `COMPLETE`
**Estimated Context:** Large (complex file rewrite)
**Dependencies:** Phase 2 complete

#### Context Setup
The Tauri commands need to:
1. Extract PII from user's vault profile
2. Create scanner with channels
3. Spawn scanner on std::thread (NOT tokio)
4. Spawn progress reporter on tokio
5. Handle pause/stop commands

**File:** `src-tauri/src/commands/discovery.rs`

This task provides the complete new file. It's long but self-contained.

#### Steps

- [ ] **Step 1:** Back up existing file
```bash
cp src-tauri/src/commands/discovery.rs src-tauri/src/commands/discovery.rs.bak
```

- [ ] **Step 2:** Replace `src-tauri/src/commands/discovery.rs`

See separate file: `docs/superpowers/plans/files/discovery_commands.rs`

(Due to length, this code block is in a companion file. Copy it from there.)

**Key changes:**
- Uses `std::thread::spawn` for scanner (not tokio::spawn)
- Creates channels with `create_scanner_channels()`
- Spawns progress reporter on tokio
- `ACTIVE_SCAN` tracks current scan for pause/stop

- [ ] **Step 3:** Verify compilation
```bash
cargo check -p spectral-app
```

- [ ] **Step 4:** Commit
```bash
git add src-tauri/src/commands/discovery.rs
git commit -m "feat(discovery): rewrite commands for thread-pool scanner"
```

#### Task Completion
- [ ] Commands rewritten
- [ ] Compiles
- [ ] Committed

#### Handoff to Task 3.2
The commands use types from `spectral_discovery` and `spectral_db::scan_logs`. Verify these are properly imported in the Tauri app's Cargo.toml if there are compile errors.

---

### Task 3.2: Verify Tauri Integration

**Status:** `COMPLETE`
**Estimated Context:** Small
**Dependencies:** Task 3.1

#### Context Setup
Quick verification that the Tauri app compiles with all the new code.

#### Steps

- [ ] **Step 1:** Full workspace build
```bash
cargo build -p spectral-app
```

- [ ] **Step 2:** Check for any missing exports or types
If errors, check that `spectral_db::scan_logs` is exported and `spectral_discovery` types are available.

- [ ] **Step 3:** Update Phase 3 status to COMPLETE

#### Task Completion
- [ ] Full build succeeds
- [ ] Phase 3 marked complete

#### Handoff to Phase 4
Phase 4 creates the frontend UI components:
- PiiExplainer: Educational component
- ScanConfig: PII type selection
- FindingCard: Individual finding display
- FindingsFilter: Filter chips

---

## Phase 4: Frontend Components

**Phase Goal:** Create reusable Svelte components for the discovery UI.

**Phase Status:** `NOT_STARTED`

---

### Task 4.1: API Types and PII Explainer

**Status:** `NOT_STARTED`
**Estimated Context:** Small (two files)
**Dependencies:** Phase 3 complete

#### Context Setup
Update the TypeScript API types and create the educational explainer component.

#### Steps

- [ ] **Step 1:** Update `src/lib/api/discovery.ts`

```typescript
import { invoke } from '@tauri-apps/api/core';

export interface DiscoveryFinding {
    id: string;
    source: string;
    source_detail: string;
    finding_type: string;
    risk_level: 'critical' | 'high' | 'medium' | 'low';
    description: string;
    recommended_action: string | null;
    pii_type: 'email' | 'phone' | 'ssn' | 'address' | 'name' | 'dob';
    remediated: boolean;
    ignored: boolean;
    still_present_after_remediation: boolean;
    found_at: string;
    matched_value?: string | null;
    line_number?: number | null;
}

export interface ScanConfig {
    scan_emails: boolean;
    scan_phones: boolean;
    scan_ssn: boolean;
    scan_addresses: boolean;
    scan_names: boolean;
    scan_dob: boolean;
}

export interface ScanProgress {
    session_id: string;
    files_scanned: number;
    files_with_findings: number;
    current_directory: string;
    is_complete: boolean;
    was_stopped: boolean;
}

export async function startDiscoveryScan(vaultId: string, config: ScanConfig): Promise<string> {
    return invoke('start_discovery_scan', { vaultId, config });
}

export async function stopDiscoveryScan(): Promise<void> {
    return invoke('stop_discovery_scan');
}

export async function pauseDiscoveryScan(): Promise<void> {
    return invoke('pause_discovery_scan');
}

export async function resumeDiscoveryScan(): Promise<void> {
    return invoke('resume_discovery_scan');
}

export async function getDiscoveryFindings(vaultId: string, includeIgnored?: boolean): Promise<DiscoveryFinding[]> {
    return invoke('get_discovery_findings', { vaultId, includeIgnored });
}

export async function markFindingRemediated(vaultId: string, findingId: string): Promise<void> {
    return invoke('mark_finding_remediated', { vaultId, findingId });
}

export async function markFindingIgnored(vaultId: string, findingId: string, ignored: boolean): Promise<void> {
    return invoke('mark_finding_ignored', { vaultId, findingId, ignored });
}

export async function deleteFile(filePath: string): Promise<void> {
    return invoke('delete_file', { filePath });
}

export async function openFileLocation(filePath: string): Promise<void> {
    return invoke('open_file_location', { filePath });
}

export async function getScanLog(vaultId: string, sessionId: string): Promise<string> {
    return invoke('get_scan_log', { vaultId, sessionId });
}
```

- [ ] **Step 2:** Create directory and explainer component
```bash
mkdir -p src/lib/components/discovery
```

- [ ] **Step 3:** Create `src/lib/components/discovery/PiiExplainer.svelte`

```svelte
<script lang="ts">
    let expanded = $state(true);
</script>

<div class="mb-6 rounded-lg border border-blue-200 bg-blue-50">
    <button
        onclick={() => (expanded = !expanded)}
        class="cursor-pointer w-full flex items-center justify-between p-4 text-left"
    >
        <div class="flex items-center gap-2">
            <span class="text-lg">&#x1F6E1;</span>
            <h2 class="text-base font-semibold text-blue-900">What is PII and why scan for it?</h2>
        </div>
        <span class="text-blue-600 text-xl">{expanded ? '▼' : '▶'}</span>
    </button>

    {#if expanded}
        <div class="px-4 pb-4 text-sm text-blue-800 space-y-4">
            <div>
                <h3 class="font-semibold mb-1">What is Personally Identifiable Information (PII)?</h3>
                <p>PII is any information that can identify you: name, email, phone, SSN, address, date of birth. When stored in unprotected files, it creates privacy and security risks.</p>
            </div>

            <div>
                <h3 class="font-semibold mb-1">Why scan for PII?</h3>
                <ul class="list-disc list-inside space-y-1 ml-2">
                    <li><strong>Data breaches:</strong> Exposed PII can be stolen for identity theft</li>
                    <li><strong>Privacy:</strong> Old files may contain sensitive info you forgot about</li>
                    <li><strong>Peace of mind:</strong> Know exactly where your sensitive data lives</li>
                </ul>
            </div>

            <div class="mt-4 p-3 bg-blue-100 rounded-lg">
                <p class="font-medium text-blue-900">&#x1F512; Your privacy is protected</p>
                <p class="text-blue-700 text-xs mt-1">All scanning happens locally. No data leaves your computer.</p>
            </div>
        </div>
    {/if}
</div>
```

- [ ] **Step 4:** Commit
```bash
git add src/lib/api/discovery.ts src/lib/components/discovery/PiiExplainer.svelte
git commit -m "feat(discovery): update API types and add explainer component"
```

#### Task Completion
- [ ] API types updated
- [ ] Explainer component created
- [ ] Committed

#### Handoff to Task 4.2
Next creates ScanConfig component for PII type selection checkboxes.

---

### Task 4.2: Scan Config Component

**Status:** `NOT_STARTED`
**Estimated Context:** Small
**Dependencies:** Task 4.1

#### Steps

- [ ] **Step 1:** Create `src/lib/components/discovery/ScanConfig.svelte`

```svelte
<script lang="ts">
    interface Props {
        config: {
            scan_emails: boolean;
            scan_phones: boolean;
            scan_ssn: boolean;
            scan_addresses: boolean;
            scan_names: boolean;
            scan_dob: boolean;
        };
        onConfigChange: (config: Props['config']) => void;
        disabled?: boolean;
    }

    let { config, onConfigChange, disabled = false }: Props = $props();

    function toggle(key: keyof Props['config']) {
        onConfigChange({ ...config, [key]: !config[key] });
    }

    const options = [
        { key: 'scan_emails' as const, label: 'Emails', icon: '&#x2709;' },
        { key: 'scan_phones' as const, label: 'Phones', icon: '&#x1F4DE;' },
        { key: 'scan_ssn' as const, label: 'SSN', icon: '&#x1F510;' },
        { key: 'scan_addresses' as const, label: 'Addresses', icon: '&#x1F3E0;' },
        { key: 'scan_names' as const, label: 'Names', icon: '&#x1F464;' },
        { key: 'scan_dob' as const, label: 'Date of Birth', icon: '&#x1F382;' },
    ];

    const hasAny = $derived(Object.values(config).some(Boolean));
</script>

<div class="rounded-lg border border-gray-200 bg-white p-4">
    <h3 class="text-sm font-semibold text-gray-900 mb-3">What PII to scan for?</h3>
    <div class="grid grid-cols-2 md:grid-cols-3 gap-2">
        {#each options as opt}
            <label class="flex items-center gap-2 p-2 rounded-lg border cursor-pointer transition-colors
                {config[opt.key] ? 'border-indigo-300 bg-indigo-50' : 'border-gray-200 hover:bg-gray-50'}
                {disabled ? 'opacity-50 cursor-not-allowed' : ''}">
                <input type="checkbox" checked={config[opt.key]} onchange={() => toggle(opt.key)} {disabled}
                    class="h-4 w-4 rounded border-gray-300 text-indigo-600" />
                <span class="text-sm">{@html opt.icon}</span>
                <span class="text-sm text-gray-700">{opt.label}</span>
            </label>
        {/each}
    </div>
    {#if !hasAny}
        <p class="mt-2 text-xs text-red-600">Select at least one PII type.</p>
    {/if}
</div>
```

- [ ] **Step 2:** Commit
```bash
git add src/lib/components/discovery/ScanConfig.svelte
git commit -m "feat(discovery): add scan config component"
```

#### Task Completion
- [ ] Component created
- [ ] Committed

#### Handoff to Task 4.3
Next creates FindingCard and FindingsFilter components.

---

### Task 4.3: Finding Card Component

**Status:** `NOT_STARTED`
**Estimated Context:** Medium
**Dependencies:** Task 4.2

#### Steps

- [ ] **Step 1:** Create `src/lib/components/discovery/FindingCard.svelte`

```svelte
<script lang="ts">
    import type { DiscoveryFinding } from '$lib/api/discovery';

    interface Props {
        finding: DiscoveryFinding;
        onMarkFixed: (id: string) => void;
        onIgnore: (id: string) => void;
        onDelete: (id: string, path: string) => void;
        onOpenLocation: (path: string) => void;
    }

    let { finding, onMarkFixed, onIgnore, onDelete, onOpenLocation }: Props = $props();
    let showDeleteConfirm = $state(false);

    function riskClass(level: string): string {
        const classes: Record<string, string> = {
            critical: 'bg-red-100 text-red-800',
            high: 'bg-orange-100 text-orange-800',
            medium: 'bg-yellow-100 text-yellow-800',
            low: 'bg-blue-100 text-blue-800',
        };
        return classes[level] || 'bg-gray-100 text-gray-800';
    }

    function formatDate(iso: string): string {
        try { return new Date(iso).toLocaleDateString(); } catch { return iso; }
    }
</script>

<div class="rounded-lg border border-gray-200 bg-white p-4">
    <div class="flex items-center gap-2 mb-2 flex-wrap">
        <span class="rounded-full px-2 py-0.5 text-xs font-medium {riskClass(finding.risk_level)}">{finding.risk_level}</span>
        <span class="rounded-full px-2 py-0.5 text-xs font-medium bg-purple-100 text-purple-800">{finding.pii_type}</span>
        {#if finding.remediated}<span class="rounded-full px-2 py-0.5 text-xs font-medium bg-green-100 text-green-800">Fixed</span>{/if}
        {#if finding.ignored}<span class="rounded-full px-2 py-0.5 text-xs font-medium bg-gray-100 text-gray-800">Ignored</span>{/if}
        {#if finding.still_present_after_remediation}<span class="rounded-full px-2 py-0.5 text-xs font-medium bg-orange-100 text-orange-800">Still Present</span>{/if}
    </div>

    <p class="text-sm font-medium text-gray-900 truncate" title={finding.source_detail}>{finding.source_detail}</p>

    {#if finding.matched_value || finding.line_number}
        <div class="mt-2 p-2 rounded bg-gray-50 font-mono text-xs">
            {#if finding.line_number}<span class="text-gray-500">Line {finding.line_number}:</span>{/if}
            {#if finding.matched_value}<span class="text-gray-900 ml-1">{finding.matched_value}</span>{/if}
        </div>
    {/if}

    <p class="mt-2 text-xs text-gray-400">Found {formatDate(finding.found_at)}</p>

    {#if !finding.remediated || finding.still_present_after_remediation}
        <div class="mt-3 pt-3 border-t border-gray-100 flex flex-wrap gap-2">
            <button onclick={() => onOpenLocation(finding.source_detail)} class="px-3 py-1.5 text-xs font-medium rounded-md bg-blue-50 text-blue-700 hover:bg-blue-100">Open Location</button>
            <button onclick={() => onMarkFixed(finding.id)} class="px-3 py-1.5 text-xs font-medium rounded-md bg-green-50 text-green-700 hover:bg-green-100">Mark Fixed</button>
            <button onclick={() => onIgnore(finding.id)} class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-50 text-gray-700 hover:bg-gray-100">Ignore</button>
            {#if !showDeleteConfirm}
                <button onclick={() => (showDeleteConfirm = true)} class="px-3 py-1.5 text-xs font-medium rounded-md bg-red-50 text-red-700 hover:bg-red-100">Delete File</button>
            {:else}
                <div class="flex items-center gap-2 p-2 rounded bg-red-50">
                    <span class="text-xs text-red-700">Delete?</span>
                    <button onclick={() => { onDelete(finding.id, finding.source_detail); showDeleteConfirm = false; }} class="px-2 py-1 text-xs rounded bg-red-600 text-white">Yes</button>
                    <button onclick={() => (showDeleteConfirm = false)} class="px-2 py-1 text-xs rounded bg-gray-200 text-gray-700">No</button>
                </div>
            {/if}
        </div>
    {/if}
</div>
```

- [ ] **Step 2:** Commit
```bash
git add src/lib/components/discovery/FindingCard.svelte
git commit -m "feat(discovery): add finding card component"
```

#### Task Completion
- [ ] Component created
- [ ] Committed

#### Handoff to Task 4.4
Next creates the filter component.

---

### Task 4.4: Findings Filter Component

**Status:** `NOT_STARTED`
**Estimated Context:** Small
**Dependencies:** Task 4.3

#### Steps

- [ ] **Step 1:** Create `src/lib/components/discovery/FindingsFilter.svelte`

```svelte
<script lang="ts">
    interface Props {
        piiTypeFilter: Set<string>;
        riskLevelFilter: Set<string>;
        showIgnored: boolean;
        onPiiTypeToggle: (type: string) => void;
        onRiskLevelToggle: (level: string) => void;
        onShowIgnoredChange: (show: boolean) => void;
        totalCount: number;
        filteredCount: number;
    }

    let { piiTypeFilter, riskLevelFilter, showIgnored, onPiiTypeToggle, onRiskLevelToggle, onShowIgnoredChange, totalCount, filteredCount }: Props = $props();

    function chipClass(selected: boolean): string {
        return selected
            ? 'px-3 py-1 rounded-full text-sm font-medium bg-indigo-600 text-white cursor-pointer'
            : 'px-3 py-1 rounded-full text-sm font-medium bg-gray-200 text-gray-700 cursor-pointer hover:bg-gray-300';
    }

    const piiTypes = ['email', 'phone', 'ssn', 'address', 'name', 'dob'];
    const riskLevels = ['critical', 'high', 'medium', 'low'];
</script>

<div class="space-y-3 mb-6">
    <div class="flex items-center gap-2 flex-wrap">
        <span class="text-sm font-medium text-gray-700">PII Type:</span>
        {#each piiTypes as type}
            <button onclick={() => onPiiTypeToggle(type)} class={chipClass(piiTypeFilter.has(type))}>{type}</button>
        {/each}
    </div>

    <div class="flex items-center gap-2 flex-wrap">
        <span class="text-sm font-medium text-gray-700">Risk:</span>
        {#each riskLevels as level}
            <button onclick={() => onRiskLevelToggle(level)} class={chipClass(riskLevelFilter.has(level))}>{level}</button>
        {/each}
    </div>

    <div class="flex items-center justify-between">
        <label class="flex items-center gap-2 cursor-pointer">
            <input type="checkbox" checked={showIgnored} onchange={(e) => onShowIgnoredChange(e.currentTarget.checked)} class="h-4 w-4 rounded" />
            <span class="text-sm text-gray-700">Show ignored</span>
        </label>
        <span class="text-sm text-gray-600">Showing {filteredCount} of {totalCount}</span>
    </div>
</div>
```

- [ ] **Step 2:** Commit
```bash
git add src/lib/components/discovery/FindingsFilter.svelte
git commit -m "feat(discovery): add findings filter component"
```

- [ ] **Step 3:** Update Phase 4 status to COMPLETE

#### Task Completion
- [ ] Component created
- [ ] Committed
- [ ] Phase 4 marked complete

#### Handoff to Phase 5
All components ready. Phase 5 rewrites the main discovery page to use them.

---

## Phase 5: Main Page Rewrite

**Phase Goal:** Rewrite the discovery page to use all new components.

**Phase Status:** `NOT_STARTED`

---

### Task 5.1: Rewrite Discovery Page

**Status:** `NOT_STARTED`
**Estimated Context:** Large (full page rewrite)
**Dependencies:** Phase 4 complete

#### Context Setup
Replace `src/routes/discovery/+page.svelte` with new implementation using all the components.

#### Steps

- [ ] **Step 1:** Back up existing file
```bash
cp src/routes/discovery/+page.svelte src/routes/discovery/+page.svelte.bak
```

- [ ] **Step 2:** Replace `src/routes/discovery/+page.svelte`

See companion file for full implementation. Key structure:
- Import all new components
- State for config, progress, findings, filters
- Event listeners in onMount
- Handler functions for all actions
- Layout with explainer, config, controls, progress, findings

- [ ] **Step 3:** Run frontend check
```bash
npm run check
```

- [ ] **Step 4:** Commit
```bash
git add src/routes/discovery/+page.svelte
git commit -m "feat(discovery): rewrite main page with new components"
```

- [ ] **Step 5:** Update Phase 5 status to COMPLETE

#### Task Completion
- [ ] Page rewritten
- [ ] Type check passes
- [ ] Committed

#### Handoff to Phase 6
Final phase: integration testing and verification.

---

## Phase 6: Testing and Verification

**Phase Goal:** Verify everything works end-to-end.

**Phase Status:** `NOT_STARTED`

---

### Task 6.1: Integration Testing

**Status:** `NOT_STARTED`
**Estimated Context:** Small (manual testing)
**Dependencies:** Phase 5 complete

#### Steps

- [ ] **Step 1:** Build and run
```bash
cargo build -p spectral-app && npm run tauri:dev
```

- [ ] **Step 2:** Manual test checklist

Test each item:
- [ ] Discovery tab opens
- [ ] PII explainer shows/hides
- [ ] Can toggle PII type checkboxes
- [ ] Start scan button works
- [ ] Progress updates show during scan
- [ ] Pause/resume works
- [ ] Stop works
- [ ] Findings display after scan
- [ ] Filters work (PII type, risk level)
- [ ] Mark fixed works
- [ ] Ignore works
- [ ] Delete file works (use test file)
- [ ] Open location works
- [ ] Download log works

- [ ] **Step 3:** Final commit if any fixes needed

- [ ] **Step 4:** Update all phase statuses to COMPLETE

- [ ] **Step 5:** Final summary commit
```bash
git add -A
git commit -m "feat(discovery): complete PII scanner rewrite"
```

#### Task Completion
- [ ] All manual tests pass
- [ ] All phases complete
- [ ] Final commit made

---

## Companion Files

Large code blocks are in separate files:
- `docs/superpowers/plans/files/discovery_commands.rs` - Tauri commands (Task 3.1)
- `docs/superpowers/plans/files/discovery_page.svelte` - Main page (Task 5.1)

Create these files before executing those tasks.

---

## Context Recovery Guide

If you're resuming after context compaction:

1. **Check the dashboard** at the top for current phase/task
2. **Read the task's Context Setup** section
3. **Check task completion boxes** to see what's done
4. **Continue from next unchecked step**

Key files to understand the current state:
- This plan file (status dashboard)
- `git log --oneline -10` (recent commits)
- `cargo check --workspace` (does it compile?)
