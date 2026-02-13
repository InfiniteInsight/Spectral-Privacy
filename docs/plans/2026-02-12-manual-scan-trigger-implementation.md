# Manual Scan Trigger Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build automated broker scanning system that searches for user's PII, presents findings for verification, and queues confirmed matches for removal.

**Architecture:** Create spectral-scanner crate with scan orchestration, database migrations for scan/finding tracking, browser-based result parsing, and Tauri/frontend integration.

**Tech Stack:** Rust, spectral-browser, spectral-db (SQLx), Tauri, TypeScript, SvelteKit, scraper (HTML parsing)

---

## Task 1: Create spectral-scanner Crate Structure

**Files:**
- Create: `crates/spectral-scanner/Cargo.toml`
- Create: `crates/spectral-scanner/src/lib.rs`
- Create: `crates/spectral-scanner/src/error.rs`
- Modify: `Cargo.toml` (workspace members)

**Step 1: Create crate directory**

```bash
mkdir -p crates/spectral-scanner/src
```

**Step 2: Write Cargo.toml**

Create `crates/spectral-scanner/Cargo.toml`:

```toml
[package]
name = "spectral-scanner"
version = "0.1.0"
edition = "2021"

[dependencies]
spectral-core = { path = "../spectral-core" }
spectral-broker = { path = "../spectral-broker" }
spectral-browser = { path = "../spectral-browser" }
spectral-db = { path = "../spectral-db" }
spectral-vault = { path = "../spectral-vault" }

tokio = { version = "1.43", features = ["full"] }
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "sqlite"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
tracing = "0.1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.11", features = ["v4", "serde"] }
async-trait = "0.1"
futures = "0.3"
scraper = "0.22"

[dev-dependencies]
tokio-test = "0.4"
```

**Step 3: Add to workspace**

Modify root `Cargo.toml`, add to members list:

```toml
members = [
    # ... existing members ...
    "crates/spectral-scanner",
]
```

**Step 4: Create lib.rs stub**

Create `crates/spectral-scanner/src/lib.rs`:

```rust
//! Spectral Scanner - Automated broker scanning and result management
//!
//! This crate orchestrates scanning data broker sites to find user PII,
//! presents findings for user verification, and integrates with the removal system.

pub mod error;

pub use error::{ScanError, Result};

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        assert!(true);
    }
}
```

**Step 5: Create error types**

Create `crates/spectral-scanner/src/error.rs`:

```rust
use thiserror::Error;
use spectral_core::BrokerId;
use std::time::Duration;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("CAPTCHA required for broker {broker_id}")]
    CaptchaRequired {
        broker_id: BrokerId,
    },

    #[error("Rate limited for broker {broker_id}, retry after {retry_after:?}")]
    RateLimited {
        broker_id: BrokerId,
        retry_after: Duration,
    },

    #[error("Broker site down: {broker_id}, HTTP {http_status}")]
    BrokerSiteDown {
        broker_id: BrokerId,
        http_status: u16,
    },

    #[error("Selectors outdated for broker {broker_id}: {reason}")]
    SelectorsOutdated {
        broker_id: BrokerId,
        reason: String,
    },

    #[error("Insufficient profile data for broker {broker_id}, missing: {missing_fields:?}")]
    InsufficientProfileData {
        broker_id: BrokerId,
        missing_fields: Vec<String>,
    },

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Browser error: {0}")]
    Browser(#[from] spectral_browser::BrowserError),

    #[error("Broker error: {0}")]
    Broker(#[from] spectral_broker::BrokerError),
}

pub type Result<T> = std::result::Result<T, ScanError>;
```

**Step 6: Run test to verify crate builds**

```bash
cargo test -p spectral-scanner
```

Expected: 1 test passing

**Step 7: Commit**

```bash
git add crates/spectral-scanner Cargo.toml
git commit -m "feat(scanner): create spectral-scanner crate with error types"
```

---

## Task 2: Add Database Migration for Scan Tables

**Files:**
- Create: `crates/spectral-db/migrations/003_scan_jobs.sql`
- Modify: `crates/spectral-db/src/lib.rs`
- Create: `crates/spectral-db/src/scan_jobs.rs`

**Step 1: Write failing test for scan_jobs table**

Modify `crates/spectral-db/src/lib.rs`, add after existing tests:

```rust
#[cfg(test)]
mod scan_tests {
    use super::*;

    #[tokio::test]
    async fn test_scan_jobs_table_exists() {
        let db = create_test_database().await.unwrap();

        // Try to query scan_jobs table
        let result = sqlx::query("SELECT id FROM scan_jobs LIMIT 1")
            .fetch_optional(db.pool())
            .await;

        assert!(result.is_ok());
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p spectral-db test_scan_jobs_table_exists
```

Expected: FAIL with "no such table: scan_jobs"

**Step 3: Create migration**

Create `crates/spectral-db/migrations/003_scan_jobs.sql`:

```sql
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
```

**Step 4: Update migration version constant**

Modify `crates/spectral-db/src/migrations.rs`, update `LATEST_VERSION`:

```rust
const LATEST_VERSION: i32 = 3; // Changed from 2 to 3
```

**Step 5: Run test to verify it passes**

```bash
cargo test -p spectral-db test_scan_jobs_table_exists
```

Expected: PASS

**Step 6: Test migrations are idempotent**

```bash
cargo test -p spectral-db test_migrations_idempotent
```

Expected: PASS

**Step 7: Commit**

```bash
git add crates/spectral-db/migrations/003_scan_jobs.sql crates/spectral-db/src/migrations.rs crates/spectral-db/src/lib.rs
git commit -m "feat(db): add scan_jobs, broker_scans, and findings tables"
```

---

## Task 3: Add Scan Job Database Module

**Files:**
- Create: `crates/spectral-db/src/scan_jobs.rs`
- Modify: `crates/spectral-db/src/lib.rs`

**Step 1: Write failing test for create_scan_job**

Create `crates/spectral-db/src/scan_jobs.rs`:

```rust
use sqlx::SqlitePool;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanJob {
    pub id: String,
    pub profile_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: ScanJobStatus,
    pub total_brokers: u32,
    pub completed_brokers: u32,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScanJobStatus {
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for ScanJobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InProgress => write!(f, "InProgress"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::Cancelled => write!(f, "Cancelled"),
        }
    }
}

pub async fn create_scan_job(
    pool: &SqlitePool,
    profile_id: String,
    total_brokers: u32,
) -> Result<ScanJob, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let started_at = Utc::now();
    let status = ScanJobStatus::InProgress;

    sqlx::query(
        "INSERT INTO scan_jobs (id, profile_id, started_at, status, total_brokers, completed_brokers)
         VALUES (?, ?, ?, ?, ?, 0)"
    )
    .bind(&id)
    .bind(&profile_id)
    .bind(started_at.to_rfc3339())
    .bind(status.to_string())
    .bind(total_brokers as i64)
    .execute(pool)
    .await?;

    Ok(ScanJob {
        id,
        profile_id,
        started_at,
        completed_at: None,
        status,
        total_brokers,
        completed_brokers: 0,
        error_message: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::create_encrypted_pool;

    async fn setup_test_db() -> SqlitePool {
        let pool = create_encrypted_pool(":memory:", "test-password")
            .await
            .unwrap();
        crate::migrations::run_migrations(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_create_scan_job() {
        let pool = setup_test_db().await;

        let job = create_scan_job(&pool, "profile-123".to_string(), 5)
            .await
            .unwrap();

        assert_eq!(job.profile_id, "profile-123");
        assert_eq!(job.total_brokers, 5);
        assert_eq!(job.completed_brokers, 0);
        assert_eq!(job.status, ScanJobStatus::InProgress);
    }
}
```

**Step 2: Expose scan_jobs module**

Modify `crates/spectral-db/src/lib.rs`, add after `pub mod removal_attempts;`:

```rust
pub mod scan_jobs;
```

**Step 3: Run test to verify it passes**

```bash
cargo test -p spectral-db test_create_scan_job
```

Expected: PASS

**Step 4: Commit**

```bash
git add crates/spectral-db/src/scan_jobs.rs crates/spectral-db/src/lib.rs
git commit -m "feat(db): add scan_jobs database module with create function"
```

---

## Task 4: Add Result Selectors to Broker Definitions

**Files:**
- Modify: `broker-definitions/people-search/spokeo.toml`
- Modify: `crates/spectral-broker/src/definition.rs`

**Step 1: Write failing test for result_selectors parsing**

Modify `crates/spectral-broker/src/definition.rs`, add test:

```rust
#[cfg(test)]
mod tests {
    // ... existing tests ...

    #[test]
    fn test_search_result_selectors_parsing() {
        let toml = r#"
            [broker]
            id = "test-broker"
            name = "Test Broker"
            url = "https://example.com"
            domain = "example.com"
            category = "people-search"
            difficulty = "Easy"
            typical_removal_days = 7
            recheck_interval_days = 30

            [search]
            method = "url-template"
            template = "https://example.com/{first}-{last}"
            requires_fields = ["first_name", "last_name"]

            [search.result_selectors]
            results_container = ".results"
            result_item = ".result-card"
            listing_url = "a.profile-link"
            name = ".name"

            [removal]
            method = "manual"
            notes = "Manual removal"
        "#;

        let def: BrokerDefinition = toml::from_str(toml).unwrap();
        let selectors = def.search.result_selectors().unwrap();
        assert_eq!(selectors.results_container, ".results");
        assert_eq!(selectors.result_item, ".result-card");
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p spectral-broker test_search_result_selectors_parsing
```

Expected: FAIL with "no method named `result_selectors`"

**Step 3: Add ResultSelectors struct**

Modify `crates/spectral-broker/src/definition.rs`, add after `SearchMethod`:

```rust
/// Selectors for parsing search result pages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultSelectors {
    /// Container holding all results
    pub results_container: String,
    /// Individual result item
    pub result_item: String,
    /// Link to full listing
    pub listing_url: String,
    /// Optional field selectors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relatives: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phones: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emails: Option<String>,
    /// Indicator that no results were found
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_results_indicator: Option<String>,
    /// CAPTCHA detection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captcha_required: Option<String>,
}
```

**Step 4: Add result_selectors field to SearchMethod**

Modify `SearchMethod` enum variants:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "kebab-case")]
pub enum SearchMethod {
    #[serde(rename = "url-template")]
    UrlTemplate {
        template: String,
        requires_fields: Vec<PiiField>,
        #[serde(skip_serializing_if = "Option::is_none")]
        result_selectors: Option<ResultSelectors>,
    },
    WebForm {
        url: String,
        fields: HashMap<String, String>,
        requires_fields: Vec<PiiField>,
        #[serde(skip_serializing_if = "Option::is_none")]
        result_selectors: Option<ResultSelectors>,
    },
    Manual {
        url: String,
        instructions: String,
    },
}

impl SearchMethod {
    pub fn result_selectors(&self) -> Option<&ResultSelectors> {
        match self {
            Self::UrlTemplate { result_selectors, .. } => result_selectors.as_ref(),
            Self::WebForm { result_selectors, .. } => result_selectors.as_ref(),
            Self::Manual { .. } => None,
        }
    }
}
```

**Step 5: Run test to verify it passes**

```bash
cargo test -p spectral-broker test_search_result_selectors_parsing
```

Expected: PASS

**Step 6: Add result_selectors to spokeo.toml**

Modify `broker-definitions/people-search/spokeo.toml`, add after `[search]`:

```toml
[search.result_selectors]
results_container = ".search-results"
result_item = ".result-card"
listing_url = "a.profile-link"
name = ".name"
age = ".age"
location = ".location"
relatives = ".relatives .name"
phones = ".phone-number"
no_results_indicator = ".no-results-message"
captcha_required = "iframe[title*='recaptcha' i]"
```

**Step 7: Validate broker definition**

```bash
python3 scripts/validate-broker-toml.py broker-definitions/people-search/spokeo.toml
```

Expected: OK

**Step 8: Run all broker tests**

```bash
cargo test -p spectral-broker
```

Expected: All tests pass

**Step 9: Commit**

```bash
git add crates/spectral-broker/src/definition.rs broker-definitions/people-search/spokeo.toml
git commit -m "feat(broker): add ResultSelectors for parsing search results

- Add ResultSelectors struct with CSS selectors for result parsing
- Add result_selectors field to SearchMethod variants
- Add selectors to Spokeo broker definition"
```

---

## Task 5: Create URL Builder for Search Templates

**Files:**
- Create: `crates/spectral-scanner/src/url_builder.rs`
- Modify: `crates/spectral-scanner/src/lib.rs`

**Step 1: Write failing test for URL building**

Create `crates/spectral-scanner/src/url_builder.rs`:

```rust
use spectral_broker::SearchMethod;
use spectral_vault::UserProfile;
use crate::error::{Result, ScanError};

pub fn build_search_url(
    method: &SearchMethod,
    profile: &UserProfile,
) -> Result<String> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral_core::PiiField;

    fn mock_profile() -> UserProfile {
        let mut profile = UserProfile::default();
        profile.first_name = Some("John".to_string());
        profile.last_name = Some("Doe".to_string());
        profile.state = Some("CA".to_string());
        profile.city = Some("Springfield".to_string());
        profile
    }

    #[test]
    fn test_build_url_from_template() {
        let method = SearchMethod::UrlTemplate {
            template: "https://example.com/{first}-{last}/{state}/{city}".to_string(),
            requires_fields: vec![
                PiiField::FirstName,
                PiiField::LastName,
                PiiField::State,
                PiiField::City,
            ],
            result_selectors: None,
        };

        let profile = mock_profile();
        let url = build_search_url(&method, &profile).unwrap();

        assert_eq!(url, "https://example.com/john-doe/CA/springfield");
    }
}
```

**Step 2: Expose url_builder module**

Modify `crates/spectral-scanner/src/lib.rs`:

```rust
pub mod error;
pub mod url_builder;

pub use error::{ScanError, Result};
pub use url_builder::build_search_url;
```

**Step 3: Run test to verify it fails**

```bash
cargo test -p spectral-scanner test_build_url_from_template
```

Expected: FAIL with "not yet implemented"

**Step 4: Implement URL builder**

Modify `crates/spectral-scanner/src/url_builder.rs`:

```rust
pub fn build_search_url(
    method: &SearchMethod,
    profile: &UserProfile,
) -> Result<String> {
    match method {
        SearchMethod::UrlTemplate { template, requires_fields, .. } => {
            let mut url = template.clone();

            // Replace placeholders
            if let Some(first) = &profile.first_name {
                url = url.replace("{first}", &first.to_lowercase());
            }
            if let Some(last) = &profile.last_name {
                url = url.replace("{last}", &last.to_lowercase());
            }
            if let Some(state) = &profile.state {
                url = url.replace("{state}", state);
            }
            if let Some(city) = &profile.city {
                url = url.replace("{city}", &city.to_lowercase().replace(" ", "-"));
            }

            Ok(url)
        }
        _ => Err(ScanError::SelectorsOutdated {
            broker_id: "unknown".into(),
            reason: "URL building only supported for UrlTemplate search method".to_string(),
        }),
    }
}
```

**Step 5: Run test to verify it passes**

```bash
cargo test -p spectral-scanner test_build_url_from_template
```

Expected: PASS

**Step 6: Commit**

```bash
git add crates/spectral-scanner/src/url_builder.rs crates/spectral-scanner/src/lib.rs
git commit -m "feat(scanner): add URL builder for search templates"
```

---

## Task 6: Create Result Parser

**Files:**
- Create: `crates/spectral-scanner/src/parser.rs`
- Modify: `crates/spectral-scanner/src/lib.rs`

**Step 1: Write failing test for result parsing**

Create `crates/spectral-scanner/src/parser.rs`:

```rust
use scraper::{Html, Selector, ElementRef};
use spectral_broker::{BrokerDefinition, ResultSelectors};
use serde::{Serialize, Deserialize};
use crate::error::{Result, ScanError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingMatch {
    pub listing_url: String,
    pub extracted_data: ExtractedData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedData {
    pub name: Option<String>,
    pub age: Option<u32>,
    pub addresses: Vec<String>,
    pub phone_numbers: Vec<String>,
    pub relatives: Vec<String>,
    pub emails: Vec<String>,
}

pub struct ResultParser<'a> {
    selectors: &'a ResultSelectors,
    base_url: String,
}

impl<'a> ResultParser<'a> {
    pub fn new(selectors: &'a ResultSelectors, base_url: String) -> Self {
        Self { selectors, base_url }
    }

    pub fn parse(&self, html: &str) -> Result<Vec<ListingMatch>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_search_results() {
        let html = r#"
            <div class="search-results">
                <div class="result-card">
                    <a class="profile-link" href="/profile/john-doe-123">View Profile</a>
                    <div class="name">John Doe</div>
                    <div class="age">35</div>
                    <div class="location">Springfield, CA</div>
                </div>
                <div class="result-card">
                    <a class="profile-link" href="/profile/jane-doe-456">View Profile</a>
                    <div class="name">Jane Doe</div>
                    <div class="age">32</div>
                    <div class="location">Los Angeles, CA</div>
                </div>
            </div>
        "#;

        let selectors = ResultSelectors {
            results_container: ".search-results".to_string(),
            result_item: ".result-card".to_string(),
            listing_url: "a.profile-link".to_string(),
            name: Some(".name".to_string()),
            age: Some(".age".to_string()),
            location: Some(".location".to_string()),
            relatives: None,
            phones: None,
            emails: None,
            no_results_indicator: None,
            captcha_required: None,
        };

        let parser = ResultParser::new(&selectors, "https://example.com".to_string());
        let matches = parser.parse(html).unwrap();

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].extracted_data.name, Some("John Doe".to_string()));
        assert_eq!(matches[0].extracted_data.age, Some(35));
        assert_eq!(matches[0].listing_url, "https://example.com/profile/john-doe-123");
    }
}
```

**Step 2: Expose parser module**

Modify `crates/spectral-scanner/src/lib.rs`:

```rust
pub mod parser;

pub use parser::{ResultParser, ListingMatch, ExtractedData};
```

**Step 3: Run test to verify it fails**

```bash
cargo test -p spectral-scanner test_parse_search_results
```

Expected: FAIL with "not yet implemented"

**Step 4: Implement result parser**

Modify `crates/spectral-scanner/src/parser.rs`:

```rust
impl<'a> ResultParser<'a> {
    pub fn parse(&self, html: &str) -> Result<Vec<ListingMatch>> {
        let document = Html::parse_document(html);

        // Check for CAPTCHA
        if let Some(captcha_sel) = &self.selectors.captcha_required {
            if let Ok(selector) = Selector::parse(captcha_sel) {
                if document.select(&selector).next().is_some() {
                    return Err(ScanError::CaptchaRequired {
                        broker_id: "unknown".into(),
                    });
                }
            }
        }

        // Check for no results
        if let Some(no_results_sel) = &self.selectors.no_results_indicator {
            if let Ok(selector) = Selector::parse(no_results_sel) {
                if document.select(&selector).next().is_some() {
                    return Ok(vec![]);
                }
            }
        }

        // Parse results
        let container_selector = Selector::parse(&self.selectors.results_container)
            .map_err(|e| ScanError::SelectorsOutdated {
                broker_id: "unknown".into(),
                reason: format!("Invalid container selector: {}", e),
            })?;

        let item_selector = Selector::parse(&self.selectors.result_item)
            .map_err(|e| ScanError::SelectorsOutdated {
                broker_id: "unknown".into(),
                reason: format!("Invalid item selector: {}", e),
            })?;

        let mut matches = Vec::new();

        for item in document.select(&item_selector) {
            if let Some(listing_match) = self.parse_item(&item)? {
                matches.push(listing_match);
            }
        }

        Ok(matches)
    }

    fn parse_item(&self, element: &ElementRef) -> Result<Option<ListingMatch>> {
        // Extract listing URL
        let url_selector = Selector::parse(&self.selectors.listing_url)
            .map_err(|e| ScanError::SelectorsOutdated {
                broker_id: "unknown".into(),
                reason: format!("Invalid URL selector: {}", e),
            })?;

        let listing_url = element
            .select(&url_selector)
            .next()
            .and_then(|el| el.value().attr("href"))
            .map(|href| {
                if href.starts_with("http") {
                    href.to_string()
                } else {
                    format!("{}{}", self.base_url, href)
                }
            });

        if listing_url.is_none() {
            return Ok(None);
        }

        // Extract data fields
        let name = self.extract_text(element, &self.selectors.name);
        let age = self.extract_text(element, &self.selectors.age)
            .and_then(|s| s.parse::<u32>().ok());
        let location = self.extract_text(element, &self.selectors.location);

        Ok(Some(ListingMatch {
            listing_url: listing_url.unwrap(),
            extracted_data: ExtractedData {
                name,
                age,
                addresses: location.into_iter().collect(),
                phone_numbers: vec![],
                relatives: vec![],
                emails: vec![],
            },
        }))
    }

    fn extract_text(&self, element: &ElementRef, selector: &Option<String>) -> Option<String> {
        selector.as_ref().and_then(|sel| {
            Selector::parse(sel)
                .ok()
                .and_then(|s| element.select(&s).next())
                .map(|el| el.text().collect::<String>().trim().to_string())
        })
    }
}
```

**Step 5: Run test to verify it passes**

```bash
cargo test -p spectral-scanner test_parse_search_results
```

Expected: PASS

**Step 6: Commit**

```bash
git add crates/spectral-scanner/src/parser.rs crates/spectral-scanner/src/lib.rs
git commit -m "feat(scanner): add HTML result parser with CSS selectors"
```

---

## Task 7: Add Tauri Commands for Scanning

**Files:**
- Create: `src-tauri/src/commands/scan.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Create scan commands module**

Create `src-tauri/src/commands/scan.rs`:

```rust
use crate::state::AppState;
use tauri::State;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct StartScanRequest {
    pub profile_id: String,
    pub broker_filter: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ScanJobResponse {
    pub id: String,
    pub status: String,
}

#[tauri::command]
pub async fn start_scan(
    state: State<'_, AppState>,
    profile_id: String,
    broker_filter: Option<String>,
) -> Result<ScanJobResponse, String> {
    // Stub for now
    Ok(ScanJobResponse {
        id: "scan-job-123".to_string(),
        status: "InProgress".to_string(),
    })
}

#[tauri::command]
pub async fn get_scan_status(
    state: State<'_, AppState>,
    scan_job_id: String,
) -> Result<ScanJobResponse, String> {
    Ok(ScanJobResponse {
        id: scan_job_id,
        status: "InProgress".to_string(),
    })
}

#[cfg(test)]
mod tests {
    // Tests will be added when we implement the actual logic
}
```

**Step 2: Expose scan module**

Modify `src-tauri/src/commands/mod.rs`:

```rust
pub mod scan;
```

**Step 3: Register Tauri commands**

Modify `src-tauri/src/lib.rs`, add to `invoke_handler`:

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    commands::scan::start_scan,
    commands::scan::get_scan_status,
])
```

**Step 4: Run tests**

```bash
cargo test -p spectral-app --lib
```

Expected: All tests pass

**Step 5: Commit**

```bash
git add src-tauri/src/commands/scan.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(tauri): add scan command stubs"
```

---

## Task 8: Create Frontend Scan API

**Files:**
- Create: `src/lib/api/scan.ts`

**Step 1: Create TypeScript API wrapper**

Create `src/lib/api/scan.ts`:

```typescript
import { invoke } from '@tauri-apps/api/core';

export interface ScanJobStatus {
	id: string;
	status: 'InProgress' | 'Completed' | 'Failed' | 'Cancelled';
}

export interface Finding {
	id: string;
	broker_id: string;
	listing_url: string;
	verification_status: 'PendingVerification' | 'Confirmed' | 'Rejected';
	extracted_data: ExtractedData;
	discovered_at: string;
}

export interface ExtractedData {
	name?: string;
	age?: number;
	addresses: string[];
	phone_numbers: string[];
	relatives: string[];
	emails: string[];
}

export const scanAPI = {
	/**
	 * Start a new scan job
	 */
	async start(profileId: string, brokerFilter?: string): Promise<ScanJobStatus> {
		return await invoke<ScanJobStatus>('start_scan', {
			profileId,
			brokerFilter
		});
	},

	/**
	 * Get scan job status
	 */
	async getStatus(scanJobId: string): Promise<ScanJobStatus> {
		return await invoke<ScanJobStatus>('get_scan_status', {
			scanJobId
		});
	},

	/**
	 * Get findings for a scan job
	 */
	async getFindings(
		scanJobId: string,
		filter?: 'PendingVerification' | 'Confirmed' | 'Rejected'
	): Promise<Finding[]> {
		return await invoke<Finding[]>('get_findings', {
			scanJobId,
			filter
		});
	},

	/**
	 * Verify a finding
	 */
	async verify(findingId: string, isMatch: boolean): Promise<void> {
		return await invoke('verify_finding', {
			findingId,
			isMatch
		});
	},

	/**
	 * Submit removal requests for all confirmed findings
	 */
	async submitRemovals(scanJobId: string): Promise<string[]> {
		return await invoke<string[]>('submit_removals_for_confirmed', {
			scanJobId
		});
	}
};
```

**Step 2: Commit**

```bash
git add src/lib/api/scan.ts
git commit -m "feat(frontend): add scan API wrapper"
```

---

## Execution Handoff

Plan complete and saved to `docs/plans/2026-02-12-manual-scan-trigger-implementation.md`.

**Two execution options:**

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**
