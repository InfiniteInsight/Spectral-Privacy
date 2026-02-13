# Manual Scan Trigger Design (Task 1.7)

**Goal:** Implement automated broker scanning with user verification workflow to find user's PII on data broker sites and present results for confirmation before removal.

**Architecture:** Automated scan + human verification + automated removal. Create `spectral-scanner` crate with scan orchestration, browser-based search, result parsing, and integration with existing removal system.

**Tech Stack:** Rust, spectral-browser (browser automation), spectral-db (SQLx), Tauri, TypeScript, SvelteKit

---

## 1. High-Level Workflow

```
User clicks "Scan Now"
  ↓
Scan Orchestrator creates scan job
  ↓
For each broker in parallel:
  - Check if profile has required fields
  - Use SearchMethod from broker definition
  - Navigate to search URL (or fill form)
  - Parse results page
  - Extract potential matches
  ↓
Store findings in database (status: PendingVerification)
  ↓
Display Results UI: "Found 12 potential matches on 5 brokers"
  ↓
User reviews each finding:
  ✅ "Yes, that's me" → status: Confirmed
  ❌ "Not me" → status: Rejected
  ⏭️ "Skip for now" → status: PendingVerification
  ↓
For confirmed matches:
  - Queue removal request
  - Use existing spectral-browser removal flow (Task 2.2)
  - Track removal status
```

**Key Principles:**
- Async/parallel scanning (5-10 brokers concurrently)
- Graceful degradation (CAPTCHA or failure on one broker doesn't block others)
- User control (never auto-remove without confirmation)
- Resumable (can stop/continue verification later)
- Auditable (full history of what was found, when, and what action was taken)

---

## 2. Data Model & Database Schema

**New Tables:**

```sql
-- Scan jobs track overall scan operations
CREATE TABLE scan_jobs (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    status TEXT NOT NULL, -- 'InProgress', 'Completed', 'Failed', 'Cancelled'
    total_brokers INTEGER NOT NULL,
    completed_brokers INTEGER NOT NULL,
    error_message TEXT,
    FOREIGN KEY (profile_id) REFERENCES profiles(id)
);

-- Individual broker scans within a job
CREATE TABLE broker_scans (
    id TEXT PRIMARY KEY,
    scan_job_id TEXT NOT NULL,
    broker_id TEXT NOT NULL,
    status TEXT NOT NULL, -- 'Pending', 'InProgress', 'Success', 'Failed', 'Skipped'
    started_at TEXT,
    completed_at TEXT,
    error_message TEXT,
    findings_count INTEGER DEFAULT 0,
    FOREIGN KEY (scan_job_id) REFERENCES scan_jobs(id)
);

-- Findings are potential matches found on broker sites
CREATE TABLE findings (
    id TEXT PRIMARY KEY,
    broker_scan_id TEXT NOT NULL,
    broker_id TEXT NOT NULL,
    profile_id TEXT NOT NULL,
    listing_url TEXT NOT NULL,
    verification_status TEXT NOT NULL, -- 'PendingVerification', 'Confirmed', 'Rejected'

    -- Extracted data from listing (encrypted)
    extracted_data TEXT NOT NULL, -- JSON: {name, age, location, relatives, phones, etc}

    -- Metadata
    discovered_at TEXT NOT NULL,
    verified_at TEXT,
    verified_by_user BOOLEAN,

    -- Removal tracking
    removal_attempt_id TEXT, -- Links to removal_attempts table (from Task 2.2)

    FOREIGN KEY (broker_scan_id) REFERENCES broker_scans(id),
    FOREIGN KEY (profile_id) REFERENCES profiles(id),
    FOREIGN KEY (removal_attempt_id) REFERENCES removal_attempts(id)
);
```

**Rust Types:**

```rust
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

pub enum ScanJobStatus {
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

pub struct Finding {
    pub id: String,
    pub broker_id: BrokerId,
    pub listing_url: String,
    pub verification_status: VerificationStatus,
    pub extracted_data: ExtractedListingData,
    pub discovered_at: DateTime<Utc>,
    pub removal_attempt_id: Option<String>,
}

pub enum VerificationStatus {
    PendingVerification,
    Confirmed,
    Rejected,
}

pub struct ExtractedListingData {
    pub name: Option<String>,
    pub age: Option<u32>,
    pub addresses: Vec<String>,
    pub phone_numbers: Vec<String>,
    pub relatives: Vec<String>,
    pub emails: Vec<String>,
}
```

---

## 3. Scan Orchestrator & Execution

**New Crate: `spectral-scanner`**

```rust
pub struct ScanOrchestrator {
    broker_registry: Arc<BrokerRegistry>,
    browser_engine: Arc<BrowserEngine>,
    db: Arc<EncryptedPool>,
    max_concurrent_scans: usize, // Default: 5
}

impl ScanOrchestrator {
    pub async fn start_scan(
        &self,
        profile: &UserProfile,
        broker_filter: BrokerFilter,
    ) -> Result<ScanJobId> {
        // 1. Create scan job in database
        // 2. Get list of brokers to scan
        // 3. Filter brokers by profile completeness
        // 4. Launch scans in parallel (up to max_concurrent_scans)
    }

    async fn execute_scan_job(
        &self,
        job_id: ScanJobId,
        brokers: Vec<BrokerDefinition>,
        profile: &UserProfile,
    ) {
        // Parallel execution with FuturesUnordered
        // Respect max_concurrent_scans limit
        // Mark job complete when all done
    }

    async fn scan_single_broker(
        &self,
        job_id: ScanJobId,
        broker: BrokerDefinition,
        profile: &UserProfile,
    ) -> Result<BrokerScanResult> {
        // 1. Create broker_scan record
        // 2. Build search URL or navigate to search form
        // 3. Parse results and create findings
        // 4. Update scan status
    }
}
```

**Key Features:**
- Concurrent scanning with configurable limit
- Respects rate limits from spectral-browser
- Graceful failure (one broker failure doesn't stop others)
- Profile field validation (skip brokers if missing required data)
- Manual broker handling (some brokers need human intervention)

---

## 4. Result Detection & Parsing

**Enhanced Broker Definition (TOML):**

```toml
[search]
method = "url-template"
template = "https://www.spokeo.com/{first}-{last}/{state}/{city}"
requires_fields = ["first_name", "last_name", "state", "city"]

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
captcha_required = "iframe[title*='recaptcha']"
```

**Parser Implementation:**

```rust
pub struct ResultParser {
    broker_def: BrokerDefinition,
}

impl ResultParser {
    pub fn parse_search_results(
        &self,
        html: &str,
        base_url: &str,
    ) -> Result<Vec<ListingMatch>> {
        // 1. Check for no-results indicator
        // 2. Check for CAPTCHA
        // 3. Parse result items using CSS selectors
        // 4. Extract name, age, location, relatives, phones, emails
    }
}
```

**Fallback Strategy:**
- If broker doesn't have result_selectors defined:
  - Navigate to search URL
  - Take screenshot
  - Return finding with status RequiresManualReview
  - Show user the screenshot in UI

---

## 5. User Interface Flow

**1. Scan Initiation Screen:**
- Profile selection
- Broker filter options (all, category, custom)
- Profile completeness warnings

**2. Scan Progress Screen:**
- Progress bar (X/Y brokers)
- Live broker status updates
- Running findings count
- Pause/cancel options

**3. Results Review Screen (Main UI):**
- Grouped by broker
- Individual finding cards showing:
  - Extracted data (name, age, location, relatives, phones)
  - Confidence indicator (High/Medium/Low)
  - Actions: "View Full Listing", "✓ Yes, Me", "✗ Not Me"
- Filter by verification status
- Batch "Submit Removals" button

**4. Removal Confirmation:**
- Summary of confirmed findings
- Removal process explanation
- Confirm & start removals

**Component Structure:**
```
src/lib/components/scan/
  ScanWizard.svelte          // Initiate scan
  ScanProgress.svelte        // Live progress updates
  FindingsReview.svelte      // Main results UI
  FindingCard.svelte         // Individual match card
  RemovalConfirmation.svelte // Batch removal dialog
```

---

## 6. Error Handling & Edge Cases

**Common Failure Scenarios:**

```rust
pub enum ScanError {
    CaptchaRequired { broker_id, screenshot_path },
    RateLimited { broker_id, retry_after },
    BrokerSiteDown { broker_id, http_status },
    SelectorsOutdated { broker_id, reason },
    InsufficientProfileData { broker_id, missing_fields },
}
```

**Handling:**
- CAPTCHA: Save state, prompt user for manual solve, resume
- Rate Limited: Schedule retry, continue with others
- Site Down: Mark failed, continue
- Outdated Selectors: Flag for maintenance, fall back to manual
- Missing Data: Skip broker, show prompt to complete profile

**Additional Edge Cases:**
- Partial profile data → skip brokers gracefully
- Ambiguous results → show confidence levels
- Scan interruption → resume on restart
- Browser crashes → retry with backoff
- Stale findings → track historical data

---

## 7. Integration with Existing Systems

**Browser Engine (spectral-browser):**
- Reuse browser automation from Task 2.1
- Navigate to search URLs and forms
- Extract HTML for parsing

**Database (spectral-db):**
- New migration: 003_scan_jobs.sql
- New modules: scan_jobs, findings
- Links findings → removal_attempts via foreign key

**Profile System (spectral-vault):**
- Check profile completeness before scanning
- Helper: `has_required_fields()`, `missing_fields()`

**Removal Flow (Task 2.2):**
- After user confirms finding → create removal_attempt
- Link finding to removal via removal_attempt_id
- Reuse existing browser-based removal logic

**Tauri Commands:**
```rust
start_scan(profile_id, broker_filter) -> ScanJobId
get_scan_status(scan_job_id) -> ScanJobStatus
get_findings(scan_job_id, filter) -> Vec<Finding>
verify_finding(finding_id, is_match) -> ()
submit_removals_for_confirmed(scan_job_id) -> Vec<RemovalAttemptId>
```

**Frontend API:**
```typescript
scanAPI.start(profileId, filter) -> ScanJobId
scanAPI.getStatus(scanJobId) -> ScanJobStatus
scanAPI.getFindings(scanJobId, filter) -> Finding[]
scanAPI.verify(findingId, isMatch) -> void
scanAPI.submitRemovals(scanJobId) -> string[]
```

---

## 8. Testing Strategy

**Unit Tests:**
- Scan orchestrator parallel execution
- Profile field filtering
- Result parser with real HTML samples
- No-results and CAPTCHA detection

**Integration Tests:**
- Full scan workflow (database + browser)
- Mock HTTP servers to avoid hitting real brokers in CI

**Selector Validation:**
- CI job to periodically test broker selectors
- Flag brokers with broken selectors

**UI Component Tests:**
- Finding verification actions
- Scan progress updates
- Removal confirmation flow

---

## 9. Implementation Roadmap

**Phase 1: Core Infrastructure (Week 1)**
- Create spectral-scanner crate structure
- Database schema + migrations
- ScanOrchestrator skeleton
- URL building for UrlTemplate search methods

**Phase 2: Browser Integration (Week 2)**
- ResultParser with CSS selector extraction
- Add result_selectors to 5 existing broker TOMLs
- Browser-based scanning
- Error handling (CAPTCHA, rate limits, failures)

**Phase 3: UI (Week 3)**
- Tauri commands
- Frontend API wrappers
- Scan wizard + progress screen
- Findings review UI
- Integration with removal flow

**Phase 4: Polish (Week 4)**
- Testing (unit + integration)
- Selector validation script
- Documentation
- Manual verification fallback

---

## 10. Current Broker Coverage

**Phase 1 MVP (5 brokers):**
1. Spokeo (Easy) - URL template search
2. BeenVerified (Medium) - Web form search
3. FastPeopleSearch (Easy) - URL template search
4. TruePeopleSearch (Easy) - URL template search
5. Whitepages (Medium) - Web form search

**Future Expansion:**
- Tier 1 (v0.1): 10 total brokers
- Tier 2 (v0.2): 20 total brokers
- Tier 3 (v0.3+): 100+ brokers across all categories

---

## 11. Out of Scope (Future Features)

**Privacy Rights Requests (Separate Feature):**
- CCPA/GDPR deletion requests to adtech companies
- Batch email generation for 50+ tracking companies
- No verification required (companies usually comply without jurisdiction proof)
- Separate from broker scanning (different workflow)

**Network Telemetry (Phase 3):**
- Passive monitoring of network connections to detect tracking
- Domain intelligence database for adtech/tracker classification
- Privacy score calculation
- Different architecture from active broker scanning

---

## Design Approval

This design was collaboratively developed and approved for implementation on 2026-02-12.

**Next Steps:**
1. Create git worktree for isolated development
2. Create detailed implementation plan
3. Execute using subagent-driven development
