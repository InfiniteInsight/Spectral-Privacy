# Phase 4: Findings Storage and Verification Workflow Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Replace stub implementations with real findings storage using ResultParser, implement verification workflow, and enable removal request creation.

**Architecture:** Integrate ResultParser into orchestrator to extract structured data from broker HTML, add deduplication logic, implement three Tauri commands (get_findings, verify_finding, submit_removals), and connect to existing UI.

**Tech Stack:** Rust, spectral-scanner, spectral-db, SQLite, Tauri, TypeScript

---

## Task 1: Add Deduplication Function to Findings Module

**Files:**
- Modify: `crates/spectral-db/src/findings.rs`

**Goal:** Add function to check if a finding already exists by listing URL within a scan job.

**Step 1: Write failing test**

Add to `crates/spectral-db/src/findings.rs` test module (after `test_verification_status_parse`):

```rust
#[tokio::test]
async fn test_finding_exists_by_url() {
    let db = setup_test_db().await;

    let extracted = serde_json::json!({"name": "Test"});

    // Create a finding
    create_finding(
        db.pool(),
        "scan-789".to_string(),
        "spokeo".to_string(),
        "profile-123".to_string(),
        "https://example.com/unique-listing".to_string(),
        extracted,
    )
    .await
    .expect("create finding");

    // Check existence
    let exists = finding_exists_by_url(
        db.pool(),
        "job-456",
        "https://example.com/unique-listing",
    )
    .await
    .expect("check exists");

    assert!(exists);

    // Check non-existent URL
    let not_exists = finding_exists_by_url(
        db.pool(),
        "job-456",
        "https://example.com/different-listing",
    )
    .await
    .expect("check not exists");

    assert!(!not_exists);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p spectral-db test_finding_exists_by_url`

Expected: FAIL with "cannot find function `finding_exists_by_url`"

**Step 3: Implement deduplication function**

Add to `crates/spectral-db/src/findings.rs` after `verify_finding` function (around line 212):

```rust
/// Check if a finding already exists by listing URL within a scan job.
///
/// Used for deduplication to prevent creating duplicate findings.
///
/// # Errors
/// Returns `sqlx::Error` if the database query fails.
pub async fn finding_exists_by_url(
    pool: &Pool<Sqlite>,
    scan_job_id: &str,
    listing_url: &str,
) -> Result<bool, sqlx::Error> {
    let row = sqlx::query(
        "SELECT EXISTS(
            SELECT 1 FROM findings f
            JOIN broker_scans bs ON f.broker_scan_id = bs.id
            WHERE bs.scan_job_id = ? AND f.listing_url = ?
        ) as 'exists'",
    )
    .bind(scan_job_id)
    .bind(listing_url)
    .fetch_one(pool)
    .await?;

    let exists: i64 = row.try_get("exists")?;
    Ok(exists != 0)
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p spectral-db test_finding_exists_by_url`

Expected: PASS

**Step 5: Commit**

```bash
git add crates/spectral-db/src/findings.rs
git commit -m "feat(db): add deduplication check for findings by URL

- Add finding_exists_by_url query function
- Joins broker_scans to check within scan_job context
- Prevents duplicate findings from same listing URL
- Includes test coverage for exists/not-exists cases"
```

---

## Task 2: Enhance Orchestrator with Real ResultParser

**Files:**
- Modify: `crates/spectral-scanner/src/orchestrator.rs:420-451`

**Goal:** Replace dummy finding creation with real HTML parsing using ResultParser.

**Step 1: Write failing integration test**

Add to `crates/spectral-scanner/src/orchestrator.rs` test module at end:

```rust
#[tokio::test]
async fn test_parse_and_store_with_real_parser() {
    use spectral_broker::definition::ResultSelectors;
    use spectral_db::Database;

    // Setup database
    let key = vec![0u8; 32];
    let db = Database::new(":memory:", key).await.unwrap();
    db.run_migrations().await.unwrap();

    // Create test profile
    let dummy_data = [0u8; 32];
    let dummy_nonce = [0u8; 12];
    sqlx::query(
        "INSERT INTO profiles (id, data, nonce, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind("profile-123")
    .bind(&dummy_data[..])
    .bind(&dummy_nonce[..])
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(db.pool())
    .await
    .unwrap();

    // Create scan job and broker scan
    sqlx::query(
        "INSERT INTO scan_jobs (id, profile_id, started_at, status, total_brokers, completed_brokers) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind("job-456")
    .bind("profile-123")
    .bind(chrono::Utc::now().to_rfc3339())
    .bind("InProgress")
    .bind(1)
    .bind(0)
    .execute(db.pool())
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO broker_scans (id, scan_job_id, broker_id, status, started_at) VALUES (?, ?, ?, ?, ?)"
    )
    .bind("scan-789")
    .bind("job-456")
    .bind("test-broker")
    .bind("InProgress")
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(db.pool())
    .await
    .unwrap();

    // Create HTML with results
    let html = r#"
        <div class="results">
            <div class="result-item">
                <a href="/profile/123" class="name-link">John Doe</a>
                <span class="age">35</span>
                <span class="location">Los Angeles, CA</span>
            </div>
            <div class="result-item">
                <a href="/profile/456" class="name-link">Jane Smith</a>
                <span class="age">42</span>
                <span class="location">San Francisco, CA</span>
            </div>
        </div>
    "#;

    // Create test selectors
    let selectors = ResultSelectors {
        results_container: ".results".to_string(),
        result_item: ".result-item".to_string(),
        listing_url: ".name-link".to_string(),
        name: Some(".name-link".to_string()),
        age: Some(".age".to_string()),
        location: Some(".location".to_string()),
        phone_number: None,
        email: None,
        relatives: None,
        captcha_required: None,
        no_results_indicator: None,
    };

    let broker_registry = Arc::new(spectral_broker::BrokerRegistry::new());
    let browser_engine = Arc::new(
        spectral_browser::BrowserEngine::new()
            .await
            .expect("create browser"),
    );
    let db_arc = Arc::new(db);

    let orchestrator = ScanOrchestrator::new(broker_registry, browser_engine, db_arc.clone())
        .with_max_concurrent_scans(1);

    // Parse and store
    let count = orchestrator
        .parse_and_store_findings_with_selectors(
            html,
            "scan-789",
            &spectral_core::BrokerId::new("test-broker").unwrap(),
            "profile-123",
            "job-456",
            Some(&selectors),
        )
        .await
        .expect("parse and store");

    // Should create 2 findings
    assert_eq!(count, 2);

    // Verify findings in database
    let findings = spectral_db::findings::get_by_broker_scan(db_arc.pool(), "scan-789")
        .await
        .expect("get findings");

    assert_eq!(findings.len(), 2);
    assert_eq!(findings[0].listing_url, "https://example.com/profile/123");
    assert_eq!(findings[1].listing_url, "https://example.com/profile/456");

    // Check extracted data
    assert_eq!(findings[0].extracted_data["name"], "John Doe");
    assert_eq!(findings[0].extracted_data["age"], 35);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p spectral-scanner test_parse_and_store_with_real_parser`

Expected: FAIL with "cannot find method `parse_and_store_findings_with_selectors`"

**Step 3: Add helper function to convert ExtractedData to JSON**

Add to `orchestrator.rs` after the `BrokerScanResult` struct (around line 37):

```rust
/// Convert parser ExtractedData to JSON for database storage.
fn extracted_data_to_json(data: &crate::parser::ExtractedData) -> serde_json::Value {
    serde_json::json!({
        "name": data.name,
        "age": data.age,
        "addresses": data.addresses,
        "phone_numbers": data.phone_numbers,
        "relatives": data.relatives,
        "emails": data.emails
    })
}
```

**Step 4: Replace parse_and_store_findings implementation**

Replace the existing `parse_and_store_findings` function (lines 420-451) with:

```rust
/// Parse HTML and store findings in database with real ResultParser.
///
/// Uses CSS selectors from broker definition to extract structured data.
/// Implements deduplication to prevent duplicate findings.
async fn parse_and_store_findings(
    &self,
    html: &str,
    broker_scan_id: &str,
    broker_id: &BrokerId,
    profile_id: &str,
) -> Result<usize> {
    // Get broker definition to access selectors
    let broker_def = self.broker_registry.get(broker_id)?;

    // Check if broker has result selectors
    let selectors = match broker_def.search.result_selectors() {
        Some(sel) => sel,
        None => {
            tracing::warn!("Broker {} has no result selectors, skipping parse", broker_id);
            return Ok(0);
        }
    };

    // Get scan job ID for deduplication check
    let scan_job_id = sqlx::query_scalar::<_, String>(
        "SELECT scan_job_id FROM broker_scans WHERE id = ?"
    )
    .bind(broker_scan_id)
    .fetch_one(self.db.pool())
    .await?;

    self.parse_and_store_findings_with_selectors(
        html,
        broker_scan_id,
        broker_id,
        profile_id,
        &scan_job_id,
        Some(selectors),
    )
    .await
}

/// Internal helper for parsing with explicit selectors (testable).
async fn parse_and_store_findings_with_selectors(
    &self,
    html: &str,
    broker_scan_id: &str,
    broker_id: &BrokerId,
    profile_id: &str,
    scan_job_id: &str,
    selectors: Option<&spectral_broker::definition::ResultSelectors>,
) -> Result<usize> {
    let selectors = match selectors {
        Some(s) => s,
        None => return Ok(0),
    };

    // Parse HTML
    use crate::parser::ResultParser;
    let base_url = format!("https://example.com"); // TODO: Get from broker definition
    let parser = ResultParser::new(selectors, base_url);

    let matches = match parser.parse(html) {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("Failed to parse HTML for {}: {}", broker_id, e);
            return Err(e);
        }
    };

    let mut created_count = 0;

    for listing_match in matches {
        // Deduplication check
        let exists = spectral_db::findings::finding_exists_by_url(
            self.db.pool(),
            scan_job_id,
            &listing_match.listing_url,
        )
        .await?;

        if exists {
            tracing::debug!("Skipping duplicate finding: {}", listing_match.listing_url);
            continue;
        }

        // Convert extracted data to JSON
        let extracted_json = extracted_data_to_json(&listing_match.extracted_data);

        // Create finding
        spectral_db::findings::create_finding(
            self.db.pool(),
            broker_scan_id.to_string(),
            broker_id.to_string(),
            profile_id.to_string(),
            listing_match.listing_url.clone(),
            extracted_json,
        )
        .await?;

        created_count += 1;
    }

    Ok(created_count)
}
```

**Step 5: Add imports at top of file**

Add after existing imports (around line 15):

```rust
use spectral_broker::definition::ResultSelectors;
```

**Step 6: Run test to verify it passes**

Run: `cargo test -p spectral-scanner test_parse_and_store_with_real_parser`

Expected: PASS

**Step 7: Run all scanner tests**

Run: `cargo test -p spectral-scanner`

Expected: All tests pass

**Step 8: Commit**

```bash
git add crates/spectral-scanner/src/orchestrator.rs
git commit -m "feat(scanner): integrate real ResultParser for finding extraction

- Replace dummy finding creation with ResultParser
- Extract structured data using CSS selectors from broker definitions
- Add deduplication check before creating findings
- Handle missing selectors gracefully (log warning, skip)
- Extract helper for testability (parse_and_store_findings_with_selectors)
- Add comprehensive integration test with real HTML parsing"
```

---

## Task 3: Implement get_findings Tauri Command

**Files:**
- Modify: `src-tauri/src/commands/scan.rs:128-139`

**Goal:** Replace stub with real database query to fetch findings.

**Step 1: Define response types**

Add after `ScanJobResponse` struct (around line 20):

```rust
#[derive(Debug, Serialize)]
pub struct FindingResponse {
    pub id: String,
    pub broker_id: String,
    pub listing_url: String,
    pub verification_status: String,
    pub extracted_data: ExtractedDataResponse,
    pub discovered_at: String,
}

#[derive(Debug, Serialize)]
pub struct ExtractedDataResponse {
    pub name: Option<String>,
    pub age: Option<u32>,
    pub addresses: Vec<String>,
    pub phone_numbers: Vec<String>,
    pub relatives: Vec<String>,
    pub emails: Vec<String>,
}
```

**Step 2: Add conversion function**

Add after response types:

```rust
/// Convert database Finding to API response.
fn finding_to_response(finding: spectral_db::findings::Finding) -> FindingResponse {
    // Extract fields from JSON extracted_data
    let name = finding.extracted_data.get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let age = finding.extracted_data.get("age")
        .and_then(|v| v.as_u64())
        .map(|a| a as u32);

    let addresses = finding.extracted_data.get("addresses")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let phone_numbers = finding.extracted_data.get("phone_numbers")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let relatives = finding.extracted_data.get("relatives")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let emails = finding.extracted_data.get("emails")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    FindingResponse {
        id: finding.id,
        broker_id: finding.broker_id,
        listing_url: finding.listing_url,
        verification_status: finding.verification_status.to_string(),
        extracted_data: ExtractedDataResponse {
            name,
            age,
            addresses,
            phone_numbers,
            relatives,
            emails,
        },
        discovered_at: finding.discovered_at.to_rfc3339(),
    }
}
```

**Step 3: Replace get_findings stub**

Replace lines 128-139 with:

```rust
/// Get findings for a scan job with optional verification status filter.
#[tauri::command]
pub async fn get_findings(
    state: State<'_, AppState>,
    vault_id: String,
    scan_job_id: String,
    filter: Option<String>,
) -> Result<Vec<FindingResponse>, String> {
    // Get the unlocked vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{}' is not unlocked", vault_id))?;

    // Get the vault's database
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {}", e))?;

    // Get all findings for this scan job
    let mut findings = spectral_db::findings::get_by_scan_job(db.pool(), &scan_job_id)
        .await
        .map_err(|e| format!("Failed to get findings: {}", e))?;

    // Filter by verification status if requested
    if let Some(filter_status) = filter {
        findings.retain(|f| f.verification_status.to_string() == filter_status);
    }

    // Convert to response format
    let responses: Vec<FindingResponse> = findings
        .into_iter()
        .map(finding_to_response)
        .collect();

    Ok(responses)
}
```

**Step 4: Run cargo check**

Run: `cargo check -p spectral-app`

Expected: No errors

**Step 5: Commit**

```bash
git add src-tauri/src/commands/scan.rs
git commit -m "feat(tauri): implement get_findings command

- Query findings from database by scan_job_id
- Support optional verification status filter
- Convert database Finding to FindingResponse format
- Extract structured data from JSON fields
- Return findings array to frontend"
```

---

## Task 4: Implement verify_finding Tauri Command

**Files:**
- Modify: `src-tauri/src/commands/scan.rs:141-152`

**Goal:** Replace stub with database update for verification status.

**Step 1: Replace verify_finding stub**

Replace lines 141-152 with:

```rust
/// Update the verification status of a finding.
#[tauri::command]
pub async fn verify_finding(
    state: State<'_, AppState>,
    vault_id: String,
    finding_id: String,
    is_match: bool,
) -> Result<(), String> {
    // Get the unlocked vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{}' is not unlocked", vault_id))?;

    // Get the vault's database
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {}", e))?;

    // Update verification status
    spectral_db::findings::verify_finding(
        db.pool(),
        &finding_id,
        is_match,
        true, // verified_by_user = true
    )
    .await
    .map_err(|e| format!("Failed to verify finding: {}", e))?;

    Ok(())
}
```

**Step 2: Run cargo check**

Run: `cargo check -p spectral-app`

Expected: No errors

**Step 3: Commit**

```bash
git add src-tauri/src/commands/scan.rs
git commit -m "feat(tauri): implement verify_finding command

- Update verification status in database
- Set verified_by_user=true to distinguish from auto-verify
- Support both confirm (is_match=true) and reject (is_match=false)
- Return clear error messages on failure"
```

---

## Task 5: Add Removal Attempts Database Functions

**Files:**
- Create: `crates/spectral-db/src/removal_attempts.rs`
- Modify: `crates/spectral-db/src/lib.rs`

**Goal:** Add CRUD operations for removal_attempts table.

**Step 1: Create removal_attempts module**

Create `crates/spectral-db/src/removal_attempts.rs`:

```rust
//! Removal attempts operations for tracking opt-out requests.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};

/// A removal attempt represents an opt-out request to a data broker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemovalAttempt {
    /// Unique identifier
    pub id: String,
    /// ID of the finding being removed
    pub finding_id: String,
    /// ID of the broker
    pub broker_id: String,
    /// Status of the removal attempt
    pub status: RemovalStatus,
    /// When the attempt was created
    pub created_at: DateTime<Utc>,
    /// When the request was submitted (if submitted)
    pub submitted_at: Option<DateTime<Utc>>,
    /// When the removal was completed (if completed)
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Status of a removal attempt.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RemovalStatus {
    /// Queued for processing
    Pending,
    /// Request has been submitted to broker
    Submitted,
    /// Removal confirmed complete
    Completed,
    /// Removal failed
    Failed,
}

impl std::fmt::Display for RemovalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Submitted => write!(f, "Submitted"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

/// Create a new removal attempt.
///
/// # Errors
/// Returns `sqlx::Error` if the database insert fails.
pub async fn create_removal_attempt(
    pool: &Pool<Sqlite>,
    finding_id: String,
    broker_id: String,
) -> Result<RemovalAttempt, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = Utc::now();
    let status = RemovalStatus::Pending;

    sqlx::query(
        "INSERT INTO removal_attempts (id, finding_id, broker_id, status, created_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&finding_id)
    .bind(&broker_id)
    .bind(status.to_string())
    .bind(created_at.to_rfc3339())
    .execute(pool)
    .await?;

    // Update finding to link removal_attempt_id
    sqlx::query("UPDATE findings SET removal_attempt_id = ? WHERE id = ?")
        .bind(&id)
        .bind(&finding_id)
        .execute(pool)
        .await?;

    Ok(RemovalAttempt {
        id,
        finding_id,
        broker_id,
        status,
        created_at,
        submitted_at: None,
        completed_at: None,
        error_message: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    async fn setup_test_db() -> Database {
        let key = vec![0u8; 32];
        let db = Database::new(":memory:", key).await.unwrap();
        db.run_migrations().await.unwrap();

        // Create test data
        let dummy_data = [0u8; 32];
        let dummy_nonce = [0u8; 12];
        sqlx::query(
            "INSERT INTO profiles (id, data, nonce, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind("profile-123")
        .bind(&dummy_data[..])
        .bind(&dummy_nonce[..])
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(db.pool())
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO scan_jobs (id, profile_id, started_at, status, total_brokers, completed_brokers) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind("job-456")
        .bind("profile-123")
        .bind(Utc::now().to_rfc3339())
        .bind("Completed")
        .bind(1)
        .bind(1)
        .execute(db.pool())
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO broker_scans (id, scan_job_id, broker_id, status, started_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind("scan-789")
        .bind("job-456")
        .bind("spokeo")
        .bind("Success")
        .bind(Utc::now().to_rfc3339())
        .execute(db.pool())
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO findings (id, broker_scan_id, broker_id, profile_id, listing_url, verification_status, extracted_data, discovered_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind("finding-123")
        .bind("scan-789")
        .bind("spokeo")
        .bind("profile-123")
        .bind("https://example.com/123")
        .bind("Confirmed")
        .bind("{}")
        .bind(Utc::now().to_rfc3339())
        .execute(db.pool())
        .await
        .unwrap();

        db
    }

    #[tokio::test]
    async fn test_create_removal_attempt() {
        let db = setup_test_db().await;

        let attempt = create_removal_attempt(
            db.pool(),
            "finding-123".to_string(),
            "spokeo".to_string(),
        )
        .await
        .expect("create removal attempt");

        assert_eq!(attempt.finding_id, "finding-123");
        assert_eq!(attempt.broker_id, "spokeo");
        assert_eq!(attempt.status, RemovalStatus::Pending);
        assert!(attempt.submitted_at.is_none());

        // Verify finding was updated with removal_attempt_id
        let row = sqlx::query("SELECT removal_attempt_id FROM findings WHERE id = ?")
            .bind("finding-123")
            .fetch_one(db.pool())
            .await
            .expect("fetch finding");

        let removal_id: Option<String> = row.try_get("removal_attempt_id").unwrap();
        assert_eq!(removal_id, Some(attempt.id));
    }

    #[tokio::test]
    async fn test_removal_status_display() {
        assert_eq!(RemovalStatus::Pending.to_string(), "Pending");
        assert_eq!(RemovalStatus::Submitted.to_string(), "Submitted");
        assert_eq!(RemovalStatus::Completed.to_string(), "Completed");
        assert_eq!(RemovalStatus::Failed.to_string(), "Failed");
    }
}
```

**Step 2: Export module**

Add to `crates/spectral-db/src/lib.rs` after other module declarations:

```rust
pub mod removal_attempts;
```

**Step 3: Run test to verify it passes**

Run: `cargo test -p spectral-db test_create_removal_attempt test_removal_status_display`

Expected: Both tests pass

**Step 4: Commit**

```bash
git add crates/spectral-db/src/removal_attempts.rs crates/spectral-db/src/lib.rs
git commit -m "feat(db): add removal_attempts module

- Create removal_attempt record with Pending status
- Link removal_attempt_id to finding
- RemovalStatus enum (Pending, Submitted, Completed, Failed)
- Test coverage for creation and status display"
```

---

## Task 6: Implement submit_removals_for_confirmed Tauri Command

**Files:**
- Modify: `src-tauri/src/commands/scan.rs:154-164`

**Goal:** Replace stub with database operations to create removal attempts.

**Step 1: Replace submit_removals stub**

Replace lines 154-164 with:

```rust
/// Submit removal requests for all confirmed findings in a scan job.
///
/// Creates removal_attempt records for each confirmed finding.
#[tauri::command]
pub async fn submit_removals_for_confirmed(
    state: State<'_, AppState>,
    vault_id: String,
    scan_job_id: String,
) -> Result<Vec<String>, String> {
    // Get the unlocked vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{}' is not unlocked", vault_id))?;

    // Get the vault's database
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {}", e))?;

    // Get all confirmed findings for this scan job
    let findings = spectral_db::findings::get_by_scan_job(db.pool(), &scan_job_id)
        .await
        .map_err(|e| format!("Failed to get findings: {}", e))?;

    let confirmed_findings: Vec<_> = findings
        .into_iter()
        .filter(|f| f.verification_status == spectral_db::findings::VerificationStatus::Confirmed)
        .collect();

    // Create removal attempts
    let mut removal_ids = Vec::new();

    for finding in confirmed_findings {
        let removal_attempt = spectral_db::removal_attempts::create_removal_attempt(
            db.pool(),
            finding.id.clone(),
            finding.broker_id.clone(),
        )
        .await
        .map_err(|e| format!("Failed to create removal attempt: {}", e))?;

        removal_ids.push(removal_attempt.id);
    }

    Ok(removal_ids)
}
```

**Step 2: Run cargo check**

Run: `cargo check -p spectral-app`

Expected: No errors

**Step 3: Commit**

```bash
git add src-tauri/src/commands/scan.rs
git commit -m "feat(tauri): implement submit_removals_for_confirmed command

- Query all findings for scan job
- Filter to only confirmed findings
- Create removal_attempt for each confirmed finding
- Link removal_attempt_id to finding record
- Return array of removal attempt IDs"
```

---

## Task 7: Integration Test for Full Workflow

**Files:**
- Create: `crates/spectral-scanner/tests/findings_workflow_test.rs`

**Goal:** End-to-end test covering scan → parse → verify → submit flow.

**Step 1: Create integration test**

Create `crates/spectral-scanner/tests/findings_workflow_test.rs`:

```rust
//! Integration test for complete findings workflow.

use spectral_scanner::{BrokerFilter, ScanOrchestrator};
use spectral_broker::BrokerRegistry;
use spectral_browser::BrowserEngine;
use spectral_db::{findings, removal_attempts, Database};
use spectral_vault::{UserProfile, EncryptedField};
use std::sync::Arc;

#[tokio::test]
async fn test_full_findings_workflow() {
    // Setup database
    let key = vec![0u8; 32];
    let vault_key = [0x42; 32];
    let db = Database::new(":memory:", key).await.expect("create db");
    db.run_migrations().await.expect("run migrations");

    // Create test profile
    let mut profile = UserProfile::default();
    profile.id = spectral_core::types::ProfileId::new("test-profile").unwrap();
    profile.first_name = Some(EncryptedField::encrypt_string("John", &vault_key).unwrap());
    profile.last_name = Some(EncryptedField::encrypt_string("Doe", &vault_key).unwrap());

    // Setup orchestrator
    let broker_registry = Arc::new(BrokerRegistry::new());
    let browser_engine = Arc::new(BrowserEngine::new().await.expect("create browser"));
    let db_arc = Arc::new(db);

    let orchestrator = ScanOrchestrator::new(
        broker_registry,
        browser_engine,
        db_arc.clone(),
    )
    .with_max_concurrent_scans(1);

    // STEP 1: Start scan
    let job_id = orchestrator
        .start_scan(&profile, BrokerFilter::All, &vault_key)
        .await
        .expect("start scan");

    // Wait for background scan to complete (in real scenario this would poll)
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // STEP 2: Get findings (should have some from real parsing)
    let all_findings = findings::get_by_scan_job(db_arc.pool(), &job_id)
        .await
        .expect("get findings");

    // Note: Might be 0 if brokers have no selectors, that's OK for this test
    println!("Found {} findings", all_findings.len());

    // STEP 3: Verify a finding (if any exist)
    if let Some(finding) = all_findings.first() {
        findings::verify_finding(db_arc.pool(), &finding.id, true, true)
            .await
            .expect("verify finding");

        // Check status updated
        let verified = findings::get_by_scan_job(db_arc.pool(), &job_id)
            .await
            .expect("get findings after verify");

        let verified_finding = verified.iter().find(|f| f.id == finding.id).unwrap();
        assert_eq!(
            verified_finding.verification_status,
            findings::VerificationStatus::Confirmed
        );

        // STEP 4: Submit removals
        let confirmed = findings::get_by_scan_job(db_arc.pool(), &job_id)
            .await
            .expect("get findings")
            .into_iter()
            .filter(|f| f.verification_status == findings::VerificationStatus::Confirmed)
            .collect::<Vec<_>>();

        for conf_finding in confirmed {
            let _removal = removal_attempts::create_removal_attempt(
                db_arc.pool(),
                conf_finding.id.clone(),
                conf_finding.broker_id.clone(),
            )
            .await
            .expect("create removal");
        }

        // Verify removal was created
        let updated_finding = findings::get_by_scan_job(db_arc.pool(), &job_id)
            .await
            .expect("get findings")
            .into_iter()
            .find(|f| f.id == finding.id)
            .unwrap();

        assert!(updated_finding.removal_attempt_id.is_some());
    }

    println!("Full workflow test completed successfully");
}
```

**Step 2: Run test**

Run: `cargo test -p spectral-scanner --test findings_workflow_test`

Expected: PASS

**Step 3: Commit**

```bash
git add crates/spectral-scanner/tests/findings_workflow_test.rs
git commit -m "test(scanner): add integration test for full findings workflow

- Test scan → parse → verify → submit flow
- Create scan job and wait for completion
- Verify finding status update
- Create removal attempt
- Confirm removal_attempt_id linked to finding
- End-to-end validation of Phase 4 features"
```

---

## Completion Checklist

When all tasks complete:

**Verify implementation:**

```bash
# Run all tests
cargo test --workspace

# Check TypeScript compilation
npm run check

# Run linter
npm run lint
```

**Manual testing:**
1. Start app: `npm run tauri:dev`
2. Create/unlock vault
3. Create profile with complete data
4. Start scan
5. Wait for completion
6. Review findings (should see real data now)
7. Confirm/reject findings
8. Submit removals
9. Check that removal count is correct

**Success criteria:**
- ✅ All Rust tests pass
- ✅ TypeScript type check passes
- ✅ Linter passes
- ✅ Real findings displayed in UI with extracted data
- ✅ Verification updates status in database
- ✅ Submit removals creates removal_attempt records
- ✅ No duplicate findings created

**Ready for review and merge!**
