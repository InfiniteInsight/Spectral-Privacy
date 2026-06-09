# Selector Discovery Engine — Spec & Plan

## Problem

Broker scanner CSS selectors become stale as broker sites change their HTML structure. When selectors don't match, scans silently return zero results. The user has no visibility into what went wrong.

## Solution

A **Selector Discovery Engine** that runs browser automation against live broker sites, analyzes the DOM to find which CSS selectors contain the user's known PII, and automatically updates broker definition TOML files. It operates silently as part of the scan pre-flight — no user intervention required.

## Design

### Core Components

| Component | Responsibility |
|-----------|---------------|
| **Discovery Engine** (Rust) | Orchestrates browser sessions, runs JS, interprets results |
| **DOM Analyzer** (JS) | Injected into browser context, collects PII→selector mappings |
| **Selector Scorer** (Rust) | Ranks discovered selectors by robustness (class-based > id-based > tag-based > positional) |
| **TOML Merger** (Rust) | Safely updates broker definition files with fresh selectors |
| **Health Checker** (Rust) | Compares current selectors against discovered selectors, reports drift |

### Discovery Flow

```
User clicks "Scan"
  → Orchestrator loads broker definitions
  → For each broker with stale selectors (>7 days old or never verified):
      → Launch headless browser with stealth patches
      → Navigate to broker search page
      → Submit search query (URL template or form fill)
      → Run DOM Analyzer JS to map PII → selectors
      → Score and select best selectors
      → Write updated TOML (atomic save with backup)
      → Proceed with scan using fresh selectors
  → If discovery fails (CAPTCHA, site error, no results page):
      → Log warning, skip broker, continue with other brokers
```

### JS DOM Analyzer Output Format

The JS returns structured data:
```json
{
  "page_state": "results" | "no_results" | "captcha" | "error",
  "pii_matches": [
    {
      "field": "first_name",
      "value": "Evan",
      "selectors": [
        {"selector": "div.result-card:nth-child(1) h3.name", "confidence": 0.95},
        {"selector": "div.result-card:nth-child(1) > .info > h2", "confidence": 0.7}
      ]
    }
  ],
  "structure": {
    "result_containers": ["div.search-results-grid"],
    "result_items": ["div.result-card"],
    "listing_links": ["a.profile-link", "div.result-card a"]
  }
}
```

### Selector Scoring Rules

1. **Specificity bonus**: `div.result-card > h3.name` scores higher than `div.result-card h3.name` (direct child > descendant)
2. **Class preference**: `.name` scores higher than `h3` (class is more stable than tag name)
3. **Positional penalty**: `:nth-child(N)` gets heavy penalty (fragile to layout changes)
4. **Length bonus**: Simpler selectors score higher (shorter is usually more robust)
5. **Uniqueness bonus**: Selector that matches exactly 1 item scores higher than one matching N

### File Update Strategy

- **Atomic writes**: Write to `.tmp` file, then `rename` (atomic on Unix/Windows)
- **Backups**: Keep `broker.toml.bak` with previous content
- **Timestamps**: Update `broker.last_verified` to current date
- **Rollback**: If scan fails after update, keep TOML as-is (scan failures aren't selector problems)
- **Diff logging**: Log what changed in `tracing::info!` for audit trail

### Error Handling

| Error | Action |
|-------|--------|
| CAPTCHA detected | Log warning, skip broker, continue |
| Site unreachable | Log error, skip broker, continue |
| Page returns "no results" | Try different query if available, else log and skip |
| JS injection fails | Fall back to plain HTTP fetch for page content |
| TOML write fails | Keep old selectors, log error, continue |
| All discovery attempts fail | Mark broker as "needs manual update", continue |

---

# Implementation Plan

## Phase 1: Core Discovery Engine

Build the Rust engine that orchestrates browser sessions, runs JS, and interprets DOM analysis results.

### Task 1.1: Scaffold `spectral-selector-discovery` crate

**Prompt:** Create a new Rust crate `crates/spectral-selector-discovery` in the workspace. It depends on `spectral-browser`, `spectral-broker`, `spectral-db`, and `spectral-core`. Create the module structure: `lib.rs`, `engine.rs`, `dom_analyzer.rs`, `scorer.rs`, `toml_updater.rs`. Add a `Cargo.toml` with appropriate dependencies.

**Tracking:**
- [ ] Started
- [ ] Completed
- Deviation:

### Task 1.2: Implement DOM Analyzer JS

**Prompt:** Write the JavaScript blob (`dom_analyzer.js`) that gets injected into the browser context. It must:

1. Walk the entire DOM tree collecting text content mapped to CSS selectors
2. Search for user PII values (first name, last name, age, city, state, phone patterns)
3. For each PII match, record the element's selector, class list, tag, and parent chain
4. Detect result containers, individual result items, and listing link patterns
5. Detect CAPTCHA presence, "no results" messages, and error states
6. Return structured JSON via `window.__selectorDiscovery = {...}`

The JS should be compact but thorough — optimize for execution speed since it runs on every discovery pass.

**Tracking:**
- [ ] Started
- [ ] Completed
- Deviation:

### Task 1.3: Implement Discovery Engine

**Prompt:** Implement `Engine` struct in `engine.rs` with these methods:

```rust
pub struct DiscoveryEngine {
    browser_engine: Arc<BrowserEngine>,
}

impl DiscoveryEngine {
    pub fn new(browser_engine: Arc<BrowserEngine>) -> Self;

    /// Run full discovery against a broker. Navigates, searches, analyzes DOM.
    pub async fn discover(
        &self,
        broker: &BrokerDefinition,
        profile: &UserProfile,
        vault_key: &[u8; 32],
    ) -> Result<DiscoveryResult>;

    /// Navigate to a URL and extract page state.
    pub async fn navigate_and_extract(&self, url: &str) -> Result<PageState>;

    /// Submit a form fill and wait for results.
    pub async fn submit_search(&self, broker: &BrokerDefinition, profile: &UserProfile, vault_key: &[u8; 32]) -> Result<PageState>;
}

pub struct DiscoveryResult {
    pub page_state: PageState,
    pub pii_matches: Vec<PIIMatch>,
    pub structure: DiscoveredStructure,
    pub current_selectors: CurrentSelectors,
    pub selector_changes: Vec<SelectorChange>,
}
```

The `discover` method should:
1. Navigate to the broker's search URL (from TOML or URL template)
2. If URL-template: navigate directly; if WebForm: fill form using current selectors
3. Wait for page to stabilize (3s timeout)
4. Extract DOM analysis via `page.evaluate()`
5. Parse the JS output into structured Rust types
6. Compare against current TOML selectors

**Tracking:**
- [ ] Started
- [ ] Completed
- Deviation:

### Task 1.4: Implement Selector Scorer

**Prompt:** Implement `SelectorScorer` in `scorer.rs`. It ranks discovered selectors by robustness:

```rust
pub struct SelectorScorer;

impl SelectorScorer {
    pub fn score(&self, selector: &str, match_count: usize) -> Score;
    pub fn pick_best(&self, candidates: &[SelectorCandidate]) -> String;
}

pub struct Score {
    pub total: f64,
    pub specificity: f64,
    pub class_bonus: f64,
    pub positional_penalty: f64,
    pub length_score: f64,
    pub uniqueness: f64,
}
```

Scoring rules:
- **Direct child (`>`)**: +1.0 bonus over descendant (` `)
- **Class selector (`.name`)**: +2.0 bonus
- **ID selector (`#name`)**: +1.5 bonus (use cautiously, IDs can change)
- **Tag-only (`div`)**: -1.0 penalty
- **`:nth-child(N)`**: -3.0 penalty (fragile)
- **`:contains()` pseudo**: -0.5 penalty (not standard)
- **Length > 50 chars**: -0.5 penalty (too specific)
- **Matches exactly 1 item**: +3.0 bonus
- **Matches > 10 items**: -2.0 penalty (too broad for individual items)

Pick the top-scoring selector for each field (name, age, location, etc.).

**Tracking:**
- [ ] Started
- [ ] Completed
- Deviation:

### Task 1.5: Implement TOML Merger

**Prompt:** Implement `TomlMerger` in `toml_updater.rs`. It updates broker definition files atomically:

```rust
pub struct TomlMerger {
    broker_dir: PathBuf,
}

impl TomlMerger {
    pub fn new(broker_dir: PathBuf) -> Self;

    /// Update selectors in a broker TOML file atomically.
    pub fn update_selectors(
        &self,
        broker_id: &BrokerId,
        selectors: &DiscoveredSelectors,
    ) -> Result<UpdateResult>;

    /// Create a backup before overwriting.
    fn backup_file(&self, path: &Path) -> Result<PathBuf>;

    /// Write TOML to .tmp file, then rename atomically.
    fn atomic_write(&self, path: &Path, content: &str) -> Result<()>;
}

pub struct UpdateResult {
    pub broker_id: BrokerId,
    pub changed: bool,
    pub changes: Vec<String>,
    pub backup_path: Option<PathBuf>,
}
```

The `update_selectors` method should:
1. Read current TOML
2. Apply discovered selectors to the appropriate sections
3. Update `broker.last_verified` to current date
4. Write backup of old version
5. Atomically write new version
6. Return diff of changes

**Tracking:**
- [ ] Started
- [ ] Completed
- Deviation:

---

## Phase 2: Orchestrator Integration

Integrate discovery into the scanner orchestrator as a pre-flight step.

### Task 2.1: Add discovery pre-flight hook

**Prompt:** Modify `ScanOrchestrator::scan_single_broker` in `crates/spectral-scanner/src/orchestrator.rs` to add a pre-flight selector check. Before executing the scan:

1. Load the broker TOML and check `broker.last_verified`
2. If `last_verified` is > 7 days old, or is missing, or is "never":
   - Create a `DiscoveryEngine` (or reuse shared browser engine)
   - Call `discovery_engine.discover(broker, profile, vault_key)`
   - If selectors changed: call `toml_merger.update_selectors()` to persist
   - If discovery failed: log warning, mark broker as "selector refresh needed"
3. If selectors are fresh: skip discovery, proceed with scan
4. Log all actions at `tracing::info!` level with broker name

The pre-flight should have its own timeout (30s) and not block the scan indefinitely.

**Tracking:**
- [ ] Started
- [ ] Completed
- Deviation:

### Task 2.2: Add discovery post-failure trigger

**Prompt:** Add logic to trigger discovery when a scan returns zero results:

In `process_scan_result`, after a broker scan completes:
1. If `findings_count == 0` and no error occurred:
   - Log `[post-scan] Zero findings for {broker}, triggering selector refresh`
   - Launch a background discovery task (don't block the scan)
   - If discovery finds updated selectors, log them but don't apply automatically
   - This gives Evan visibility into what changed without risking auto-application on a false positive

**Tracking:**
- [ ] Started
- [ ] Completed
- Deviation:

### Task 2.3: Add health check cron job

**Prompt:** Create a cron job that runs nightly (or configurable interval) against all Tier-1 brokers. This is a background health check, not a real scan.

The job should:
1. Load all Tier-1 broker definitions
2. For each broker, run a lightweight discovery pass with test PII
3. Compare discovered selectors against current TOML
4. Generate a report: `Broker X: selectors fresh`, `Broker Y: 3 selectors need update`
5. Write report to `~/.local/share/spectral/selector-health-report.json`
6. Optionally notify Evan via the messaging channel

The health check uses synthetic/test PII data — no real user data needed.

**Tracking:**
- [ ] Started
- [ ] Completed
- Deviation:

---

## Phase 3: CLI Tool (Standalone)

Build a standalone CLI for manual selector discovery and debugging.

### Task 3.1: Implement `spectral-discover` binary

**Prompt:** Create a `spectral-discover` binary crate with CLI interface:

```bash
spectral-discover --broker fastpeoplesearch --first Evan --last Doe --state CA
spectral-discover --broker fastpeoplesearch --dry-run     # show what would change
spectral-discover --broker fastpeoplesearch --apply        # write updated TOML
spectral-discover --broker fastpeoplesearch --diff         # show diff only
spectral-discover --all --health                           # run health check on all brokers
```

Use `clap` for argument parsing. The binary should:
1. Parse CLI args
2. Load the specified broker definition
3. If PII fields provided, run discovery with that profile data
4. If `--dry-run` or `--diff`, output what selectors would change
5. If `--apply`, write updated TOML files
6. If `--health`, check all brokers and output report

**Tracking:**
- [ ] Started
- [ ] Completed
- Deviation:

### Task 3.2: Add `--output` flag for TOML generation

**Prompt:** Add support for outputting fresh TOML snippets to stdout:

```bash
spectral-discover --broker fastpeoplesearch --first Evan --last Doe --state CA --output toml
```

This outputs just the `[search.result_selectors]` and `[search.form_selectors]` sections, ready to paste into a broker TOML file.

**Tracking:**
- [ ] Started
- [ ] Completed
- Deviation:

---

## Phase 4: UI Integration (Optional, Future)

Add a UI panel showing selector health status and manual refresh.

### Task 4.1: Add selector health endpoint

**Prompt:** Add a Tauri command that returns selector health status for all brokers:

```rust
#[tauri::command]
async fn get_selector_health(broker_id: Option<String>) -> Result<Vec<SelectorHealthEntry>> {
    // Return list of brokers with their selector freshness status
}

pub struct SelectorHealthEntry {
    pub broker_id: String,
    pub broker_name: String,
    pub last_verified: Option<String>,
    pub selectors_stale: bool,
    pub last_scan_findings: usize,
    pub needs_refresh: bool,
}
```

**Tracking:**
- [ ] Started
- [ ] Completed
- Deviation:

### Task 4.2: Add settings panel for discovery

**Prompt:** Add a settings panel in the frontend where Evan can:
1. Configure the selector freshness threshold (default 7 days)
2. See which brokers have stale selectors
3. Manually trigger discovery for a specific broker
4. Review selector diffs before applying
5. View historical selector change log

This is a lower-priority feature — the pre-flight auto-discovery (Phase 2) makes this less critical since most updates happen silently.

**Tracking:**
- [ ] Started
- [ ] Completed
- Deviation:

---

# Deviation Tracking

When any task is implemented with deviations from this plan, record them below. Downstream tasks should be reviewed for impact.

| Task | Deviation | Impacted Tasks |
|------|-----------|---------------|
| 1.1 | — | — |
| 1.2 | — | — |
| 1.3 | — | — |
| 1.4 | — | — |
| 1.5 | — | — |
| 2.1 | — | — |
| 2.2 | — | — |
| 2.3 | — | — |
| 3.1 | — | — |
| 3.2 | — | — |
| 4.1 | — | — |
| 4.2 | — | — |

---

# Cross-Cutting Concerns

## Security
- All discovery happens with the same stealth patches already in use
- No PII is logged — only selector data
- TOML updates are atomic (no partial writes)
- No external services called during discovery

## Platform Compatibility
- All file operations use `std::path::Path` (cross-platform)
- Browser launch respects existing platform config (WSL2, Windows, macOS, Linux)
- TOML parsing uses `toml` crate (platform-independent)

## Performance
- Discovery runs in parallel with scan (separate browser sessions if possible)
- Timeout guards on all browser operations (30s default)
- DOM analysis JS is optimized for speed (no deep recursion)
- Selector scoring is O(n log n) where n = number of candidate selectors

## Testing
- Unit tests for `SelectorScorer` with known-good selectors
- Integration tests with mock HTML responses (no browser needed)
- E2E tests against real broker sites (requires Chrome installed)
- Snapshot tests for TOML output format
