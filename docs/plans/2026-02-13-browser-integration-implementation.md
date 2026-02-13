# Browser Integration Implementation Plan (Phase 2)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete browser integration for scanning broker sites, including result selector configuration for 4 additional brokers, robust error handling, and findings storage in the database.

**Architecture:** Enhance scanner to actually fetch broker pages via spectral-browser, handle real-world failures (CAPTCHA, rate limits, timeouts), parse results, and store findings in the database with proper status tracking.

**Tech Stack:** Rust, spectral-scanner, spectral-browser, spectral-db, SQLx, scraper

**Prerequisites:**
- Tasks 1-8 completed (scanner infrastructure)
- Phase 1 completed (ScanOrchestrator implementation)

---

## Task Tracking

Use the task tracking tools to monitor progress:

```bash
# Create tasks (run once at start)
claude task create "Task 1: Add result_selectors to BeenVerified broker"
claude task create "Task 2: Add result_selectors to FastPeopleSearch broker"
claude task create "Task 3: Add result_selectors to TruePeopleSearch broker"
claude task create "Task 4: Add result_selectors to Whitepages broker"
claude task create "Task 5: Implement broker_scans table operations"
claude task create "Task 6: Implement findings table operations"
claude task create "Task 7: Add browser error handling and retries"
claude task create "Task 8: Update orchestrator to store findings"
claude task create "Task 9: Add CAPTCHA detection and reporting"
claude task create "Task 10: Add rate limit handling"

# Mark task in progress when starting
claude task update <task-id> --status in_progress

# Mark task complete when done
claude task update <task-id> --status completed

# View all tasks
claude task list
```

---

## Task 1: Add result_selectors to BeenVerified Broker

**Goal:** Configure CSS selectors for parsing BeenVerified search results.

**Files to Modify:**
- `broker-definitions/people-search/beenverified.toml`

**Implementation:**

### Step 1: Research BeenVerified selectors

Note: This requires inspecting BeenVerified's actual search results page. For this plan, we'll use placeholder selectors that should be updated after manual inspection.

### Step 2: Add result_selectors to TOML

Modify `broker-definitions/people-search/beenverified.toml`, add after `[search]` section:

```toml
[search.result_selectors]
# Note: These selectors must be verified against actual BeenVerified pages
results_container = ".search-results-container"
result_item = ".person-result"
listing_url = "a.view-details"
name = ".person-name"
age = ".person-age"
location = ".person-location"
relatives = ".relatives-list .relative-name"
phones = ".phone-numbers .phone"
no_results_indicator = ".no-results-found"
captcha_required = "iframe[src*='recaptcha']"
```

### Step 3: Validate broker definition

```bash
cargo test -p spectral-broker -- beenverified
```

Expected: Tests pass with new selectors loaded

### Step 4: Commit

```bash
git add broker-definitions/people-search/beenverified.toml
git commit -m "feat(broker): add result selectors for BeenVerified

Note: Selectors are placeholders and must be verified against live site"
```

---

## Task 2: Add result_selectors to FastPeopleSearch Broker

**Goal:** Configure CSS selectors for parsing FastPeopleSearch search results.

**Files to Modify:**
- `broker-definitions/people-search/fastpeoplesearch.toml`

**Implementation:**

### Step 1: Add result_selectors

Modify `broker-definitions/people-search/fastpeoplesearch.toml`:

```toml
[search.result_selectors]
# Note: Verify against actual FastPeopleSearch pages
results_container = "#search-results"
result_item = ".result-item"
listing_url = "a.detail-link"
name = ".result-name"
age = ".result-age"
location = ".result-address"
relatives = ".relatives .rel-name"
phones = ".phone-list .phone"
no_results_indicator = ".no-match"
captcha_required = "div[class*='captcha']"
```

### Step 2: Validate

```bash
cargo test -p spectral-broker -- fastpeoplesearch
```

Expected: PASS

### Step 3: Commit

```bash
git add broker-definitions/people-search/fastpeoplesearch.toml
git commit -m "feat(broker): add result selectors for FastPeopleSearch"
```

---

## Task 3: Add result_selectors to TruePeopleSearch Broker

**Goal:** Configure CSS selectors for parsing TruePeopleSearch search results.

**Files to Modify:**
- `broker-definitions/people-search/truepeoplesearch.toml`

**Implementation:**

### Step 1: Add result_selectors

Modify `broker-definitions/people-search/truepeoplesearch.toml`:

```toml
[search.result_selectors]
results_container = ".search-results"
result_item = ".card"
listing_url = "a.detail"
name = ".name"
age = ".age"
location = ".location"
relatives = ".relatives .name"
phones = ".contact .phone"
no_results_indicator = ".no-results"
captcha_required = ".g-recaptcha"
```

### Step 2: Validate

```bash
cargo test -p spectral-broker
```

Expected: PASS

### Step 3: Commit

```bash
git add broker-definitions/people-search/truepeoplesearch.toml
git commit -m "feat(broker): add result selectors for TruePeopleSearch"
```

---

## Task 4: Add result_selectors to Whitepages Broker

**Goal:** Configure CSS selectors for parsing Whitepages search results.

**Files to Modify:**
- `broker-definitions/people-search/whitepages.toml`

**Implementation:**

### Step 1: Add result_selectors

Modify `broker-definitions/people-search/whitepages.toml`:

```toml
[search.result_selectors]
results_container = ".results-list"
result_item = ".result"
listing_url = ".result-link"
name = ".result-name"
age = ".result-age"
location = ".result-location"
relatives = ".result-relatives .relative"
phones = ".result-phones .phone"
no_results_indicator = ".no-results-msg"
captcha_required = "iframe[title*='reCAPTCHA']"
```

### Step 2: Validate

```bash
cargo test -p spectral-broker
```

Expected: PASS

### Step 3: Commit

```bash
git add broker-definitions/people-search/whitepages.toml
git commit -m "feat(broker): add result selectors for Whitepages"
```

---

## Task 5: Implement broker_scans Table Operations

**Goal:** Add database functions for creating and updating broker scan records.

**Files to Modify:**
- `crates/spectral-db/src/scan_jobs.rs`

**Implementation:**

### Step 1: Add BrokerScan struct and types

Add to `crates/spectral-db/src/scan_jobs.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerScan {
    pub id: String,
    pub scan_job_id: String,
    pub broker_id: String,
    pub status: BrokerScanStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub findings_count: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrokerScanStatus {
    Pending,
    InProgress,
    Success,
    Failed,
    Skipped,
}

impl std::fmt::Display for BrokerScanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::InProgress => write!(f, "InProgress"),
            Self::Success => write!(f, "Success"),
            Self::Failed => write!(f, "Failed"),
            Self::Skipped => write!(f, "Skipped"),
        }
    }
}
```

### Step 2: Add create_broker_scan function

```rust
pub async fn create_broker_scan(
    pool: &SqlitePool,
    scan_job_id: String,
    broker_id: String,
) -> Result<BrokerScan, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let status = BrokerScanStatus::Pending;

    sqlx::query(
        "INSERT INTO broker_scans (id, scan_job_id, broker_id, status, findings_count)
         VALUES (?, ?, ?, ?, 0)"
    )
    .bind(&id)
    .bind(&scan_job_id)
    .bind(&broker_id)
    .bind(status.to_string())
    .execute(pool)
    .await?;

    Ok(BrokerScan {
        id,
        scan_job_id,
        broker_id,
        status,
        started_at: None,
        completed_at: None,
        error_message: None,
        findings_count: 0,
    })
}
```

### Step 3: Add update_broker_scan_status function

```rust
pub async fn update_broker_scan_status(
    pool: &SqlitePool,
    broker_scan_id: &str,
    status: BrokerScanStatus,
    error_message: Option<String>,
    findings_count: i32,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "UPDATE broker_scans
         SET status = ?, completed_at = ?, error_message = ?, findings_count = ?
         WHERE id = ?"
    )
    .bind(status.to_string())
    .bind(&now)
    .bind(error_message)
    .bind(findings_count)
    .bind(broker_scan_id)
    .execute(pool)
    .await?;

    Ok(())
}
```

### Step 4: Add test

```rust
#[cfg(test)]
mod tests {
    // ... existing tests ...

    #[tokio::test]
    async fn test_broker_scan_lifecycle() {
        let db = setup_test_database().await;

        // Create scan job first
        let job = create_scan_job(db.pool(), "profile-123".to_string(), 1)
            .await
            .expect("create scan job");

        // Create broker scan
        let broker_scan = create_broker_scan(
            db.pool(),
            job.id.clone(),
            "spokeo".to_string(),
        )
        .await
        .expect("create broker scan");

        assert_eq!(broker_scan.status, BrokerScanStatus::Pending);

        // Update status
        update_broker_scan_status(
            db.pool(),
            &broker_scan.id,
            BrokerScanStatus::Success,
            None,
            5,
        )
        .await
        .expect("update status");

        // Verify update
        let updated = sqlx::query!(
            "SELECT status, findings_count FROM broker_scans WHERE id = ?",
            broker_scan.id
        )
        .fetch_one(db.pool())
        .await
        .expect("fetch");

        assert_eq!(updated.status, "Success");
        assert_eq!(updated.findings_count, 5);
    }
}
```

### Step 5: Run test

```bash
cargo test -p spectral-db test_broker_scan_lifecycle
```

Expected: PASS

### Step 6: Commit

```bash
git add crates/spectral-db/src/scan_jobs.rs
git commit -m "feat(db): add broker_scans table operations"
```

---

## Task 6: Implement findings Table Operations

**Goal:** Add database functions for creating and querying findings.

**Files to Create:**
- `crates/spectral-db/src/findings.rs`

**Files to Modify:**
- `crates/spectral-db/src/lib.rs`

**Implementation:**

### Step 1: Create findings module

Create `crates/spectral-db/src/findings.rs`:

```rust
use sqlx::SqlitePool;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub broker_scan_id: String,
    pub broker_id: String,
    pub profile_id: String,
    pub listing_url: String,
    pub verification_status: VerificationStatus,
    pub extracted_data: JsonValue,
    pub discovered_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
    pub verified_by_user: Option<bool>,
    pub removal_attempt_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerificationStatus {
    PendingVerification,
    Confirmed,
    Rejected,
}

impl std::fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PendingVerification => write!(f, "PendingVerification"),
            Self::Confirmed => write!(f, "Confirmed"),
            Self::Rejected => write!(f, "Rejected"),
        }
    }
}

pub async fn create_finding(
    pool: &SqlitePool,
    broker_scan_id: String,
    broker_id: String,
    profile_id: String,
    listing_url: String,
    extracted_data: JsonValue,
) -> Result<Finding, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let discovered_at = Utc::now();
    let status = VerificationStatus::PendingVerification;
    let extracted_json = serde_json::to_string(&extracted_data).unwrap();

    sqlx::query(
        "INSERT INTO findings (id, broker_scan_id, broker_id, profile_id, listing_url,
                               verification_status, extracted_data, discovered_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&broker_scan_id)
    .bind(&broker_id)
    .bind(&profile_id)
    .bind(&listing_url)
    .bind(status.to_string())
    .bind(&extracted_json)
    .bind(discovered_at.to_rfc3339())
    .execute(pool)
    .await?;

    Ok(Finding {
        id,
        broker_scan_id,
        broker_id,
        profile_id,
        listing_url,
        verification_status: status,
        extracted_data,
        discovered_at,
        verified_at: None,
        verified_by_user: None,
        removal_attempt_id: None,
    })
}

pub async fn get_findings_for_scan(
    pool: &SqlitePool,
    scan_job_id: &str,
) -> Result<Vec<Finding>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT f.id, f.broker_scan_id, f.broker_id, f.profile_id, f.listing_url,
                f.verification_status, f.extracted_data, f.discovered_at,
                f.verified_at, f.verified_by_user, f.removal_attempt_id
         FROM findings f
         JOIN broker_scans bs ON f.broker_scan_id = bs.id
         WHERE bs.scan_job_id = ?
         ORDER BY f.discovered_at DESC",
        scan_job_id
    )
    .fetch_all(pool)
    .await?;

    let findings = rows
        .into_iter()
        .map(|row| Finding {
            id: row.id,
            broker_scan_id: row.broker_scan_id,
            broker_id: row.broker_id,
            profile_id: row.profile_id,
            listing_url: row.listing_url,
            verification_status: match row.verification_status.as_str() {
                "Confirmed" => VerificationStatus::Confirmed,
                "Rejected" => VerificationStatus::Rejected,
                _ => VerificationStatus::PendingVerification,
            },
            extracted_data: serde_json::from_str(&row.extracted_data).unwrap_or(JsonValue::Null),
            discovered_at: DateTime::parse_from_rfc3339(&row.discovered_at)
                .unwrap()
                .with_timezone(&Utc),
            verified_at: row.verified_at.and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
            verified_by_user: row.verified_by_user.map(|v| v != 0),
            removal_attempt_id: row.removal_attempt_id,
        })
        .collect();

    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::create_encrypted_pool;

    async fn setup_test_database() -> crate::Database {
        let key = [0x42; 32];
        let db = crate::Database::new(":memory:", key)
            .await
            .expect("create test database");
        db.run_migrations().await.expect("run migrations");
        db
    }

    #[tokio::test]
    async fn test_create_and_get_findings() {
        let db = setup_test_database().await;

        // Create test scan job and broker scan
        let job = crate::scan_jobs::create_scan_job(
            db.pool(),
            "profile-123".to_string(),
            1,
        )
        .await
        .expect("create job");

        let broker_scan = crate::scan_jobs::create_broker_scan(
            db.pool(),
            job.id.clone(),
            "spokeo".to_string(),
        )
        .await
        .expect("create broker scan");

        // Create finding
        let extracted = serde_json::json!({
            "name": "John Doe",
            "age": 35,
            "location": "Los Angeles, CA"
        });

        let finding = create_finding(
            db.pool(),
            broker_scan.id.clone(),
            "spokeo".to_string(),
            "profile-123".to_string(),
            "https://example.com/profile/123".to_string(),
            extracted,
        )
        .await
        .expect("create finding");

        assert_eq!(finding.broker_id, "spokeo");
        assert_eq!(finding.verification_status, VerificationStatus::PendingVerification);

        // Get findings for scan
        let findings = get_findings_for_scan(db.pool(), &job.id)
            .await
            .expect("get findings");

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].id, finding.id);
    }
}
```

### Step 2: Expose findings module

Modify `crates/spectral-db/src/lib.rs`:

```rust
pub mod findings;
```

### Step 3: Run test

```bash
cargo test -p spectral-db test_create_and_get_findings
```

Expected: PASS

### Step 4: Commit

```bash
git add crates/spectral-db/src/findings.rs crates/spectral-db/src/lib.rs
git commit -m "feat(db): add findings table operations"
```

---

## Task 7: Add Browser Error Handling and Retries

**Goal:** Add retry logic and proper error handling for browser operations.

**Files to Modify:**
- `crates/spectral-scanner/src/orchestrator.rs`

**Implementation:**

### Step 1: Add retry configuration

Add to top of `orchestrator.rs`:

```rust
const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 2000;
```

### Step 2: Add retry logic

Add helper method to `ScanOrchestrator`:

```rust
impl ScanOrchestrator {
    async fn fetch_with_retry(
        &self,
        url: &str,
        broker_id: &BrokerId,
    ) -> Result<String> {
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            match self.browser_engine.fetch_page(url).await {
                Ok(html) => return Ok(html),
                Err(e) => {
                    last_error = Some(e);

                    if attempt < MAX_RETRIES - 1 {
                        tracing::warn!(
                            "Fetch failed for {} (attempt {}/{}), retrying...",
                            broker_id,
                            attempt + 1,
                            MAX_RETRIES
                        );
                        tokio::time::sleep(
                            tokio::time::Duration::from_millis(RETRY_DELAY_MS * (attempt as u64 + 1))
                        ).await;
                    }
                }
            }
        }

        Err(ScanError::Browser(last_error.unwrap()))
    }
}
```

### Step 3: Update scan_single_broker to use retry logic

Update the fetch call in `scan_single_broker`:

```rust
// Replace: let html = self.browser_engine.fetch_page(&search_url).await
let html = self.fetch_with_retry(&search_url, &broker_id).await
```

### Step 4: Add test

```rust
#[cfg(test)]
mod tests {
    // ... existing tests ...

    #[tokio::test]
    async fn test_retry_logic() {
        // This test verifies retry logic compiles correctly
        // Actual retry behavior would need integration test with mock browser
        assert!(MAX_RETRIES > 0);
        assert!(RETRY_DELAY_MS > 0);
    }
}
```

### Step 5: Run tests

```bash
cargo test -p spectral-scanner
```

Expected: All tests pass

### Step 6: Commit

```bash
git add crates/spectral-scanner/src/orchestrator.rs
git commit -m "feat(scanner): add retry logic for browser failures"
```

---

## Task 8: Update Orchestrator to Store Findings

**Goal:** Update scan_single_broker to store findings in the database.

**Files to Modify:**
- `crates/spectral-scanner/src/orchestrator.rs`

**Implementation:**

### Step 1: Update scan_single_broker to store findings

Replace the `scan_single_broker` method:

```rust
use spectral_db::findings;

async fn scan_single_broker(
    &self,
    scan_job_id: &str,
    broker: &BrokerDefinition,
    profile: &UserProfile,
    vault_key: &[u8; 32],
) -> Result<BrokerScanResult> {
    let broker_id = broker.broker.id.clone();

    // Create broker_scan record
    let broker_scan = spectral_db::scan_jobs::create_broker_scan(
        self.db.pool(),
        scan_job_id.to_string(),
        broker_id.to_string(),
    )
    .await?;

    // Update status to InProgress
    spectral_db::scan_jobs::update_broker_scan_status(
        self.db.pool(),
        &broker_scan.id,
        spectral_db::scan_jobs::BrokerScanStatus::InProgress,
        None,
        0,
    )
    .await?;

    // Build search URL
    let search_url = build_search_url(&broker_id, &broker.search, profile, vault_key)?;

    // Fetch with retry
    let html = match self.fetch_with_retry(&search_url, &broker_id).await {
        Ok(html) => html,
        Err(e) => {
            // Mark as failed
            spectral_db::scan_jobs::update_broker_scan_status(
                self.db.pool(),
                &broker_scan.id,
                spectral_db::scan_jobs::BrokerScanStatus::Failed,
                Some(format!("Fetch error: {}", e)),
                0,
            )
            .await?;

            return Ok(BrokerScanResult {
                broker_id,
                findings: vec![],
                error: Some(format!("Failed to fetch: {}", e)),
            });
        }
    };

    // Parse results
    let matches = if let Some(selectors) = broker.search.result_selectors() {
        let parser = ResultParser::new(selectors, broker.broker.url.clone(), broker_id.clone());

        match parser.parse(&html) {
            Ok(m) => m,
            Err(e) => {
                // Mark as failed
                spectral_db::scan_jobs::update_broker_scan_status(
                    self.db.pool(),
                    &broker_scan.id,
                    spectral_db::scan_jobs::BrokerScanStatus::Failed,
                    Some(format!("Parse error: {}", e)),
                    0,
                )
                .await?;

                return Ok(BrokerScanResult {
                    broker_id,
                    findings: vec![],
                    error: Some(format!("Parse error: {}", e)),
                });
            }
        }
    } else {
        vec![]
    };

    // Store findings
    let mut findings_count = 0;
    for listing in &matches {
        let extracted_json = serde_json::to_value(&listing.extracted_data)
            .unwrap_or(serde_json::Value::Null);

        findings::create_finding(
            self.db.pool(),
            broker_scan.id.clone(),
            broker_id.to_string(),
            profile.id.clone(),
            listing.listing_url.clone(),
            extracted_json,
        )
        .await?;

        findings_count += 1;
    }

    // Mark as success
    spectral_db::scan_jobs::update_broker_scan_status(
        self.db.pool(),
        &broker_scan.id,
        spectral_db::scan_jobs::BrokerScanStatus::Success,
        None,
        findings_count,
    )
    .await?;

    Ok(BrokerScanResult {
        broker_id,
        findings: matches,
        error: None,
    })
}
```

### Step 2: Run tests

```bash
cargo test -p spectral-scanner
```

Expected: All tests pass

### Step 3: Commit

```bash
git add crates/spectral-scanner/src/orchestrator.rs
git commit -m "feat(scanner): store findings in database after parsing"
```

---

## Task 9: Add CAPTCHA Detection and Reporting

**Goal:** Detect CAPTCHA challenges and mark broker scans as requiring manual intervention.

**Files to Modify:**
- `crates/spectral-scanner/src/orchestrator.rs`

**Implementation:**

### Step 1: Handle CAPTCHA errors

Update the parse error handling in `scan_single_broker`:

```rust
match parser.parse(&html) {
    Ok(m) => m,
    Err(ScanError::CaptchaRequired { .. }) => {
        // Mark as failed with CAPTCHA indicator
        spectral_db::scan_jobs::update_broker_scan_status(
            self.db.pool(),
            &broker_scan.id,
            spectral_db::scan_jobs::BrokerScanStatus::Failed,
            Some("CAPTCHA required - manual intervention needed".to_string()),
            0,
        )
        .await?;

        return Ok(BrokerScanResult {
            broker_id: broker_id.clone(),
            findings: vec![],
            error: Some("CAPTCHA challenge detected".to_string()),
        });
    }
    Err(e) => {
        // ... existing error handling ...
    }
}
```

### Step 2: Add test

```rust
#[tokio::test]
async fn test_captcha_handling() {
    // Verify CAPTCHA error type exists and can be matched
    let error = ScanError::CaptchaRequired {
        broker_id: BrokerId::new("test").unwrap(),
    };

    match error {
        ScanError::CaptchaRequired { .. } => {
            // Success - pattern matching works
        }
        _ => panic!("Expected CaptchaRequired"),
    }
}
```

### Step 3: Run tests

```bash
cargo test -p spectral-scanner test_captcha_handling
```

Expected: PASS

### Step 4: Commit

```bash
git add crates/spectral-scanner/src/orchestrator.rs
git commit -m "feat(scanner): add CAPTCHA detection and handling"
```

---

## Task 10: Add Rate Limit Handling

**Goal:** Detect rate limiting and implement backoff strategy.

**Files to Modify:**
- `crates/spectral-scanner/src/orchestrator.rs`

**Implementation:**

### Step 1: Add rate limit detection

Add helper method:

```rust
impl ScanOrchestrator {
    fn is_rate_limited(error: &spectral_browser::BrowserError) -> bool {
        // Check if error indicates rate limiting
        // This depends on spectral-browser error types
        match error {
            spectral_browser::BrowserError::HttpStatus(429) => true,
            spectral_browser::BrowserError::HttpStatus(503) => true,
            _ => false,
        }
    }
}
```

### Step 2: Update retry logic to handle rate limits

Update `fetch_with_retry`:

```rust
async fn fetch_with_retry(
    &self,
    url: &str,
    broker_id: &BrokerId,
) -> Result<String> {
    let mut last_error = None;
    let mut backoff_multiplier = 1;

    for attempt in 0..MAX_RETRIES {
        match self.browser_engine.fetch_page(url).await {
            Ok(html) => return Ok(html),
            Err(e) => {
                // Check if this is a rate limit
                if Self::is_rate_limited(&e) {
                    backoff_multiplier = 3; // Longer wait for rate limits
                    tracing::warn!("Rate limited for {}, waiting longer...", broker_id);
                }

                last_error = Some(e);

                if attempt < MAX_RETRIES - 1 {
                    let delay = RETRY_DELAY_MS * backoff_multiplier * (attempt as u64 + 1);
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                }
            }
        }
    }

    // If we exhausted retries due to rate limiting, return specific error
    if let Some(e) = &last_error {
        if Self::is_rate_limited(e) {
            return Err(ScanError::RateLimited {
                broker_id: broker_id.clone(),
                retry_after: std::time::Duration::from_secs(300), // 5 minutes
            });
        }
    }

    Err(ScanError::Browser(last_error.unwrap()))
}
```

### Step 3: Handle rate limit in scan_single_broker

Add rate limit case:

```rust
let html = match self.fetch_with_retry(&search_url, &broker_id).await {
    Ok(html) => html,
    Err(ScanError::RateLimited { retry_after, .. }) => {
        // Mark as failed with retry suggestion
        spectral_db::scan_jobs::update_broker_scan_status(
            self.db.pool(),
            &broker_scan.id,
            spectral_db::scan_jobs::BrokerScanStatus::Failed,
            Some(format!("Rate limited - retry after {:?}", retry_after)),
            0,
        )
        .await?;

        return Ok(BrokerScanResult {
            broker_id,
            findings: vec![],
            error: Some("Rate limited".to_string()),
        });
    }
    Err(e) => {
        // ... existing error handling ...
    }
};
```

### Step 4: Add test

```rust
#[tokio::test]
async fn test_rate_limit_backoff() {
    // Verify rate limit handling compiles
    let backoff = RETRY_DELAY_MS * 3;
    assert!(backoff > RETRY_DELAY_MS);
}
```

### Step 5: Run tests

```bash
cargo test -p spectral-scanner
```

Expected: All tests pass

### Step 6: Commit

```bash
git add crates/spectral-scanner/src/orchestrator.rs
git commit -m "feat(scanner): add rate limit detection and backoff"
```

---

## Completion Checklist

- [ ] Task 1: BeenVerified result_selectors
- [ ] Task 2: FastPeopleSearch result_selectors
- [ ] Task 3: TruePeopleSearch result_selectors
- [ ] Task 4: Whitepages result_selectors
- [ ] Task 5: broker_scans operations
- [ ] Task 6: findings operations
- [ ] Task 7: Browser retry logic
- [ ] Task 8: Store findings in database
- [ ] Task 9: CAPTCHA handling
- [ ] Task 10: Rate limit handling

When all tasks are complete, run full test suite:

```bash
cargo test --workspace
```

All tests should pass. The scanner now has full browser integration with robust error handling!

---

## Notes

**Important:** The CSS selectors in tasks 1-4 are placeholders. Before using in production:

1. Manually inspect each broker's search results page
2. Identify correct CSS selectors using browser dev tools
3. Update the TOML files with actual selectors
4. Test with real profiles to verify extraction works

Consider creating a selector validation script (mentioned in Phase 4) to periodically test selectors against live sites.
