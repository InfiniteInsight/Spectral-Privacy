# Phase 4: Findings Storage and Verification Workflow - Design Document

**Date:** 2026-02-13
**Status:** Approved
**Phase:** Phase 4 of Manual Scan Trigger (Task 1.7)

## Overview

Complete the scan workflow by implementing real findings storage with parsed data, verification commands, and removal request creation.

**Dependencies:**
- Phase 1: Scan Orchestrator (merged) ✅
- Phase 2: Browser Integration (merged) ✅
- Phase 3: Scan UI Components (merged) ✅

**Goal:** Enable users to see real findings from broker scans, verify accuracy, and submit removal requests.

## Architecture

### Current State Analysis

**Already Implemented:**
- `findings` database table with verification_status, extracted_data (JSON), timestamps
- `spectral_db::findings` module with CRUD operations
- `ResultParser` that extracts data from HTML using CSS selectors
- Orchestrator creates dummy findings (placeholders)
- UI review page ready to display findings

**Currently Stubbed (Phase 4):**
- Orchestrator uses dummy data instead of real parser
- `get_findings` returns empty array
- `verify_finding` is no-op
- `submit_removals_for_confirmed` returns empty array

### Data Flow

```
┌─────────────────┐
│  Scan Started   │
└────────┬────────┘
         │
         ▼
┌─────────────────────────┐
│ Orchestrator fetches    │
│ HTML from broker        │
└────────┬────────────────┘
         │
         ▼
┌─────────────────────────┐
│ ResultParser extracts   │
│ data using CSS selectors│
│ from broker definition  │
└────────┬────────────────┘
         │
         ▼
┌─────────────────────────┐
│ Deduplication check:    │
│ Skip if listing_url     │
│ already exists in scan  │
└────────┬────────────────┘
         │
         ▼
┌─────────────────────────┐
│ Create finding record   │
│ Status: Pending         │
└────────┬────────────────┘
         │
         ▼
┌─────────────────────────┐
│ User reviews in UI      │
└────────┬────────────────┘
         │
         ▼
┌─────────────────────────┐
│ verify_finding updates  │
│ status to Confirmed/    │
│ Rejected                │
└────────┬────────────────┘
         │
         ▼
┌─────────────────────────┐
│ submit_removals creates │
│ removal_attempt records │
│ for confirmed findings  │
└─────────────────────────┘
```

## Component Design

### 1. Orchestrator Enhancement

**File:** `crates/spectral-scanner/src/orchestrator.rs`

**Current Implementation (lines 420-451):**
```rust
async fn parse_and_store_findings(...) -> Result<usize> {
    // Simplified: Creates dummy finding
    let extracted_data = serde_json::json!({
        "name": "Example Name",
        "location": "Example City, ST"
    });

    spectral_db::findings::create_finding(...).await?;
    Ok(1)
}
```

**New Implementation:**
1. Get `result_selectors` from broker definition
2. Create `ResultParser` with selectors and base URL
3. Parse HTML to get `Vec<ListingMatch>`
4. For each match:
   - Check deduplication (query existing findings by listing_url + scan_job_id)
   - Convert `ExtractedData` to JSON
   - Create finding record
5. Return count of new findings created

**Error Handling:**
- If no selectors available → Log warning, create manual-review finding with URL only
- If parse fails → Log error, continue with other brokers (don't fail entire scan)
- If duplicate found → Skip silently, increment skipped counter

### 2. Get Findings Command

**File:** `src-tauri/src/commands/scan.rs`

**Signature:**
```rust
#[tauri::command]
pub async fn get_findings(
    state: State<'_, AppState>,
    vault_id: String,
    scan_job_id: String,
    filter: Option<String>, // "PendingVerification" | "Confirmed" | "Rejected"
) -> Result<Vec<FindingResponse>, String>
```

**Implementation:**
1. Get unlocked vault
2. Get vault database
3. Query findings by scan_job_id using `spectral_db::findings::get_by_scan_job`
4. Filter by verification status if filter provided
5. Convert to `FindingResponse` (matches frontend `Finding` type)
6. Return findings

**FindingResponse Structure:**
```rust
#[derive(Serialize)]
pub struct FindingResponse {
    pub id: String,
    pub broker_id: String,
    pub listing_url: String,
    pub verification_status: String,
    pub extracted_data: ExtractedDataResponse,
    pub discovered_at: String,
}

#[derive(Serialize)]
pub struct ExtractedDataResponse {
    pub name: Option<String>,
    pub age: Option<u32>,
    pub addresses: Vec<String>,
    pub phone_numbers: Vec<String>,
    pub relatives: Vec<String>,
    pub emails: Vec<String>,
}
```

### 3. Verify Finding Command

**File:** `src-tauri/src/commands/scan.rs`

**Signature:**
```rust
#[tauri::command]
pub async fn verify_finding(
    state: State<'_, AppState>,
    vault_id: String,
    finding_id: String,
    is_match: bool,
) -> Result<(), String>
```

**Implementation:**
1. Get unlocked vault
2. Get vault database
3. Call `spectral_db::findings::verify_finding(pool, finding_id, is_match, true)`
   - `true` = verified_by_user (vs. auto-verified)
4. Return Ok(())

**Validation:**
- Verify finding exists before updating
- Return clear error if not found

### 4. Submit Removals Command

**File:** `src-tauri/src/commands/scan.rs`

**Signature:**
```rust
#[tauri::command]
pub async fn submit_removals_for_confirmed(
    state: State<'_, AppState>,
    vault_id: String,
    scan_job_id: String,
) -> Result<Vec<String>, String>
```

**Implementation:**
1. Get unlocked vault
2. Get vault database
3. Query confirmed findings: `get_by_scan_job` + filter where status = "Confirmed"
4. For each confirmed finding:
   - Create `removal_attempt` record in database
   - Link removal_attempt_id to finding
5. Return array of removal_attempt IDs

**Removal Attempt Schema (already exists in migration 002):**
```sql
CREATE TABLE removal_attempts (
    id TEXT PRIMARY KEY,
    finding_id TEXT NOT NULL,
    broker_id TEXT NOT NULL,
    status TEXT NOT NULL, -- 'Pending', 'Submitted', 'Completed', 'Failed'
    created_at TEXT NOT NULL,
    submitted_at TEXT,
    completed_at TEXT,
    error_message TEXT,
    FOREIGN KEY (finding_id) REFERENCES findings(id)
);
```

### 5. Deduplication Logic

**Location:** `crates/spectral-db/src/findings.rs`

**New Function:**
```rust
pub async fn finding_exists_by_url(
    pool: &Pool<Sqlite>,
    scan_job_id: &str,
    listing_url: &str,
) -> Result<bool, sqlx::Error>
```

**Query:**
```sql
SELECT EXISTS(
    SELECT 1 FROM findings f
    JOIN broker_scans bs ON f.broker_scan_id = bs.id
    WHERE bs.scan_job_id = ? AND f.listing_url = ?
)
```

**Usage:** Call before creating finding to avoid duplicates within same scan.

### 6. ExtractedData Conversion

**Challenge:** `ResultParser` returns `parser::ExtractedData` but we need `serde_json::Value`

**Solution:** Create conversion function in orchestrator:
```rust
fn extracted_data_to_json(data: &parser::ExtractedData) -> serde_json::Value {
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

## Error Handling

### Parse Failures

**Scenario:** CSS selectors outdated, HTML structure changed

**Handling:**
1. Log error with broker_id and reason
2. Mark broker_scan as "Failed" with error message
3. Continue scanning other brokers
4. Don't create findings for failed parse

### Missing Selectors

**Scenario:** Broker definition has no `result_selectors`

**Handling:**
1. Log warning
2. Mark broker_scan as "Skipped"
3. Don't create findings (user will see 0 results for that broker)

### Database Errors

**Scenario:** Finding creation fails

**Handling:**
1. Log error
2. Return error from orchestrator task
3. Mark broker_scan as "Failed"
4. Overall scan continues (other brokers not affected)

## Testing Strategy

### Unit Tests

**Orchestrator:**
- Test parse_and_store_findings with real ResultParser
- Test deduplication logic
- Test error handling (parse failure, no selectors)

**Tauri Commands:**
- Test get_findings with filter
- Test verify_finding updates status
- Test submit_removals creates removal_attempts

### Integration Tests

**Full Flow:**
1. Start scan with broker that has selectors
2. Verify findings created in database
3. Call get_findings, verify data structure
4. Call verify_finding, check status updated
5. Call submit_removals, verify removal_attempts created

## Success Criteria

**Phase 4 Complete When:**
- ✅ Orchestrator uses real ResultParser with CSS selectors
- ✅ Findings contain extracted data (name, age, addresses, phones, etc.)
- ✅ get_findings returns actual findings from database
- ✅ Filter by verification status works
- ✅ verify_finding updates database
- ✅ submit_removals creates removal_attempt records
- ✅ Deduplication prevents duplicate findings
- ✅ All unit tests pass
- ✅ Integration test covers full flow
- ✅ Manual testing shows real data in UI

## Out of Scope (Future Phases)

**Not in Phase 4:**
- Actual removal form submission (Phase 5)
- Email verification handling (Phase 5)
- CAPTCHA solving (Phase 5)
- Finding re-verification on subsequent scans
- Batch verify operations (nice-to-have)
- Finding similarity detection (advanced dedup)
