# Discovery Findings Filtering Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add filter chips to Local PII Discovery page allowing users to filter findings by PII type (Email, Phone, SSN) and risk level (Critical, Medium, Informational).

**Architecture:** Add `pii_type` column to discovery_findings table, update backend to populate it from PiiMatch enum, implement client-side filtering with clickable chip UI in Svelte.

**Tech Stack:** Rust, SQLite, sqlx, Svelte 5, TypeScript, Tauri

---

## Task 1: Add Database Migration for pii_type Column

**Files:**
- Create: `crates/spectral-db/migrations/012_discovery_pii_type.sql`
- Modify: `crates/spectral-db/src/lib.rs` (update migration count test)
- Modify: `crates/spectral-db/src/migrations.rs` (update migration count tests)

**Step 1: Create migration file**

Create `crates/spectral-db/migrations/012_discovery_pii_type.sql`:

```sql
-- Add pii_type column to discovery_findings table
-- This allows efficient filtering by PII type (email, phone, ssn)

-- Add nullable column first
ALTER TABLE discovery_findings ADD COLUMN pii_type TEXT;

-- Backfill existing findings by parsing description
UPDATE discovery_findings
SET pii_type = 'email'
WHERE description LIKE 'Email address%';

UPDATE discovery_findings
SET pii_type = 'phone'
WHERE description LIKE 'Phone number%';

UPDATE discovery_findings
SET pii_type = 'ssn'
WHERE description LIKE 'Social Security Number%';

-- Make column NOT NULL (safe after backfill)
-- SQLite doesn't support ADD COLUMN ... NOT NULL directly
-- so we use this workaround: create new table, copy data, swap
CREATE TABLE discovery_findings_new (
    id TEXT PRIMARY KEY,
    vault_id TEXT NOT NULL,
    source TEXT NOT NULL,
    source_detail TEXT NOT NULL,
    finding_type TEXT NOT NULL,
    risk_level TEXT NOT NULL,
    description TEXT NOT NULL,
    recommended_action TEXT,
    pii_type TEXT NOT NULL,
    remediated INTEGER NOT NULL DEFAULT 0,
    found_at TEXT NOT NULL,
    FOREIGN KEY (vault_id) REFERENCES vaults(id)
);

INSERT INTO discovery_findings_new
SELECT id, vault_id, source, source_detail, finding_type, risk_level,
       description, recommended_action, pii_type, remediated, found_at
FROM discovery_findings;

DROP TABLE discovery_findings;
ALTER TABLE discovery_findings_new RENAME TO discovery_findings;
```

**Step 2: Update migration count in test (lib.rs)**

Modify `crates/spectral-db/src/lib.rs` around line 233:

```rust
assert_eq!(migrations.len(), 12); // Was 11, now 12
```

**Step 3: Update migration count in migrations.rs tests**

Modify `crates/spectral-db/src/migrations.rs`:

Lines around 114 and 134:

```rust
assert_eq!(migrations.len(), 12); // Was 11, now 12
```

**Step 4: Run database tests**

Run: `cargo test --package spectral-db`

Expected: All tests pass with updated migration count.

**Step 5: Commit**

```bash
git add crates/spectral-db/migrations/012_discovery_pii_type.sql \
        crates/spectral-db/src/lib.rs \
        crates/spectral-db/src/migrations.rs
git commit -m "feat(db): add pii_type column to discovery_findings table

Add migration to add pii_type column for efficient PII type filtering.
Backfills existing findings by parsing description field.

Migration includes:
- Add pii_type TEXT column
- Backfill email/phone/ssn from descriptions
- Enforce NOT NULL constraint

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 2: Update Database Layer to Include pii_type

**Files:**
- Modify: `crates/spectral-db/src/discovery_findings.rs`

**Step 1: Add pii_type to CreateDiscoveryFinding struct**

Modify `crates/spectral-db/src/discovery_findings.rs` around line 10-20:

```rust
pub struct CreateDiscoveryFinding {
    pub vault_id: String,
    pub source: String,
    pub source_detail: String,
    pub finding_type: String,
    pub risk_level: String,
    pub description: String,
    pub recommended_action: Option<String>,
    pub pii_type: String, // ADD THIS LINE
}
```

**Step 2: Update insert query to include pii_type**

Find the `insert_discovery_finding` function (around line 30-60) and update:

```rust
pub async fn insert_discovery_finding(
    pool: &SqlitePool,
    finding: CreateDiscoveryFinding,
) -> Result<DiscoveryFinding, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO discovery_findings (
            id, vault_id, source, source_detail, finding_type,
            risk_level, description, recommended_action, pii_type,
            found_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&finding.vault_id)
    .bind(&finding.source)
    .bind(&finding.source_detail)
    .bind(&finding.finding_type)
    .bind(&finding.risk_level)
    .bind(&finding.description)
    .bind(&finding.recommended_action)
    .bind(&finding.pii_type) // ADD THIS BIND
    .bind(&now)
    .execute(pool)
    .await?;

    // Return the created finding
    get_discovery_finding(pool, &id).await
}
```

**Step 3: Add pii_type to DiscoveryFinding struct**

Find the `DiscoveryFinding` struct (around line 5-15) and update:

```rust
pub struct DiscoveryFinding {
    pub id: String,
    pub vault_id: String,
    pub source: String,
    pub source_detail: String,
    pub finding_type: String,
    pub risk_level: String,
    pub description: String,
    pub recommended_action: Option<String>,
    pub pii_type: String, // ADD THIS LINE
    pub remediated: bool,
    pub found_at: String,
}
```

**Step 4: Update get_discovery_findings query to SELECT pii_type**

Find the query in `get_discovery_findings` (around line 80):

```rust
let findings = sqlx::query_as::<_, (String, String, String, String, String, String, String, Option<String>, String, i64, String)>(
    "SELECT id, vault_id, source, source_detail, finding_type, risk_level,
            description, recommended_action, pii_type, remediated, found_at
     FROM discovery_findings
     WHERE vault_id = ?
     ORDER BY found_at DESC"
)
.bind(vault_id)
.fetch_all(pool)
.await?;

findings
    .into_iter()
    .map(|(id, vault_id, source, source_detail, finding_type, risk_level, description, recommended_action, pii_type, remediated, found_at)| {
        DiscoveryFinding {
            id,
            vault_id,
            source,
            source_detail,
            finding_type,
            risk_level,
            description,
            recommended_action,
            pii_type, // ADD THIS FIELD
            remediated: remediated != 0,
            found_at,
        }
    })
    .collect()
```

**Step 5: Update get_discovery_finding query similarly**

Find `get_discovery_finding` function and update the same way.

**Step 6: Run tests**

Run: `cargo test --package spectral-db`

Expected: All tests pass.

**Step 7: Commit**

```bash
git add crates/spectral-db/src/discovery_findings.rs
git commit -m "feat(db): add pii_type field to discovery findings structs

Update CreateDiscoveryFinding and DiscoveryFinding structs to include
pii_type field. Update all queries to SELECT and INSERT pii_type.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 3: Update Backend Command to Set pii_type from PiiMatch

**Files:**
- Modify: `src-tauri/src/commands/discovery.rs`

**Step 1: Add helper function to map PiiMatch to pii_type string**

Add this function after the `DiscoveryFinding` struct (around line 23):

```rust
/// Map PiiMatch enum to pii_type string for database storage
fn pii_match_to_type(pii_match: &PiiMatch) -> &'static str {
    match pii_match {
        PiiMatch::Email => "email",
        PiiMatch::Phone => "phone",
        PiiMatch::Ssn => "ssn",
    }
}
```

**Step 2: Update insert_pii_finding to set pii_type**

Modify the `insert_pii_finding` function (around line 47-79):

```rust
async fn insert_pii_finding(
    file_path: &Path,
    pii_match: PiiMatch,
    pool: &sqlx::SqlitePool,
    vault_id: &str,
) -> Result<(), sqlx::Error> {
    let file_name = match file_path.file_name() {
        Some(name) => name.to_string_lossy().to_string(),
        None => {
            tracing::warn!("Could not extract filename from path: {:?}", file_path);
            file_path.to_string_lossy().to_string()
        }
    };

    let description = format!("{} found in file: {}", pii_match.description(), file_name);
    let pii_type = pii_match_to_type(&pii_match); // ADD THIS LINE

    spectral_db::discovery_findings::insert_discovery_finding(
        pool,
        spectral_db::discovery_findings::CreateDiscoveryFinding {
            vault_id: vault_id.to_string(),
            source: "filesystem".to_string(),
            source_detail: file_path.to_string_lossy().to_string(),
            finding_type: "pii_exposure".to_string(),
            risk_level: pii_match.risk_level().to_string(),
            description,
            recommended_action: Some(
                "Review file and remove sensitive information if no longer needed".to_string(),
            ),
            pii_type: pii_type.to_string(), // ADD THIS LINE
        },
    )
    .await
    .map(|_| ())
}
```

**Step 3: Update DiscoveryFinding response struct**

Find the `DiscoveryFinding` struct in this file (around line 11-22):

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoveryFinding {
    pub id: String,
    pub source: String,
    pub source_detail: String,
    pub finding_type: String,
    pub risk_level: String,
    pub description: String,
    pub recommended_action: Option<String>,
    pub pii_type: String, // ADD THIS LINE
    pub remediated: bool,
    pub found_at: String,
}
```

**Step 4: Update get_discovery_findings mapping**

Find the mapping in `get_discovery_findings` command (around line 199-212):

```rust
let response: Vec<DiscoveryFinding> = findings
    .into_iter()
    .map(|f| DiscoveryFinding {
        id: f.id,
        source: f.source,
        source_detail: f.source_detail,
        finding_type: f.finding_type,
        risk_level: f.risk_level,
        description: f.description,
        recommended_action: f.recommended_action,
        pii_type: f.pii_type, // ADD THIS LINE
        remediated: f.remediated,
        found_at: f.found_at,
    })
    .collect();
```

**Step 5: Build backend**

Run: `cargo build --package spectral-app`

Expected: Clean build with no errors.

**Step 6: Commit**

```bash
git add src-tauri/src/commands/discovery.rs
git commit -m "feat(discovery): set pii_type when creating findings

Map PiiMatch enum to pii_type string (email/phone/ssn) and store
in database when creating discovery findings.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 4: Update Frontend TypeScript Interface

**Files:**
- Modify: `src/lib/api/discovery.ts`

**Step 1: Add pii_type to DiscoveryFinding interface**

Modify `src/lib/api/discovery.ts` around line 7-17:

```typescript
export interface DiscoveryFinding {
	id: string;
	source: 'filesystem' | 'browser' | 'email';
	source_detail: string;
	finding_type: 'pii_exposure' | 'broker_contact' | 'broker_account';
	risk_level: 'critical' | 'medium' | 'informational';
	description: string;
	recommended_action: string | null;
	pii_type: 'email' | 'phone' | 'ssn'; // ADD THIS LINE
	remediated: boolean;
	found_at: string;
}
```

**Step 2: Run frontend type check**

Run: `npm run check`

Expected: No TypeScript errors.

**Step 3: Commit**

```bash
git add src/lib/api/discovery.ts
git commit -m "feat(discovery): add pii_type to DiscoveryFinding interface

Add pii_type field to TypeScript interface matching backend schema.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 5: Add Filter Chip UI to Discovery Page

**Files:**
- Modify: `src/routes/discovery/+page.svelte`

**Step 1: Add filter state variables**

Add after the existing state variables (around line 11-15):

```svelte
<script lang="ts">
	// ... existing imports and state ...

	// Filter state
	let piiTypeFilter = $state<Set<string>>(new Set());
	let riskLevelFilter = $state<Set<string>>(new Set());
</script>
```

**Step 2: Add filter toggle functions**

Add before the existing functions (around line 32):

```svelte
// Toggle PII type filter
function togglePiiType(type: string) {
	const newFilter = new Set(piiTypeFilter);
	if (newFilter.has(type)) {
		newFilter.delete(type);
	} else {
		newFilter.add(type);
	}
	piiTypeFilter = newFilter;
}

// Toggle risk level filter
function toggleRiskLevel(level: string) {
	const newFilter = new Set(riskLevelFilter);
	if (newFilter.has(level)) {
		newFilter.delete(level);
	} else {
		newFilter.add(level);
	}
	riskLevelFilter = newFilter;
}
```

**Step 3: Add filteredFindings derived state**

Add after the existing derived states (around line 30):

```svelte
// Filtered findings based on active filters
const filteredFindings = $derived(
	findings.filter((f) => {
		// Never show remediated findings
		if (f.remediated) return false;

		// PII type filter (OR logic within group)
		const piiMatch = piiTypeFilter.size === 0 || piiTypeFilter.has(f.pii_type);

		// Risk level filter (OR logic within group)
		const riskMatch = riskLevelFilter.size === 0 || riskLevelFilter.has(f.risk_level);

		// Must match ALL filter groups (AND logic between groups)
		return piiMatch && riskMatch;
	})
);
```

**Step 4: Update summary counts to use filteredFindings**

Replace the existing derived counts (around line 18-26):

```svelte
// Computed summary counts from filtered findings
const criticalCount = $derived(
	filteredFindings.filter((f) => f.risk_level === 'critical').length
);
const mediumCount = $derived(
	filteredFindings.filter((f) => f.risk_level === 'medium').length
);
const informationalCount = $derived(
	filteredFindings.filter((f) => f.risk_level === 'informational').length
);
```

**Step 5: Run frontend check**

Run: `npm run check`

Expected: No TypeScript errors.

**Step 6: Commit**

```bash
git add src/routes/discovery/+page.svelte
git commit -m "feat(discovery): add filter state and logic

Add piiTypeFilter and riskLevelFilter state, toggle functions,
and filteredFindings derived state. Update summary counts to use
filtered findings.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 6: Add Filter Chip UI Components

**Files:**
- Modify: `src/routes/discovery/+page.svelte`

**Step 1: Add helper function for chip styling**

Add after the existing helper functions (around line 115):

```svelte
function chipClass(isSelected: boolean): string {
	if (isSelected) {
		return 'px-3 py-1 rounded-full text-sm font-medium bg-indigo-600 text-white cursor-pointer hover:bg-indigo-700 transition-colors';
	}
	return 'px-3 py-1 rounded-full text-sm font-medium bg-gray-200 text-gray-700 cursor-pointer hover:bg-gray-300 transition-colors';
}
```

**Step 2: Add filter chip UI above summary cards**

Add after the error div and before summary cards (around line 155):

```svelte
	{#if !loading && findings.length > 0}
		<!-- Filter Chips -->
		<div class="mb-6 space-y-3">
			<!-- PII Type Filters -->
			<div class="flex items-center gap-2 flex-wrap">
				<span class="text-sm font-medium text-gray-700">PII Type:</span>
				<button
					onclick={() => togglePiiType('email')}
					class={chipClass(piiTypeFilter.has('email'))}
				>
					Email
				</button>
				<button
					onclick={() => togglePiiType('phone')}
					class={chipClass(piiTypeFilter.has('phone'))}
				>
					Phone
				</button>
				<button
					onclick={() => togglePiiType('ssn')}
					class={chipClass(piiTypeFilter.has('ssn'))}
				>
					SSN
				</button>
			</div>

			<!-- Risk Level Filters -->
			<div class="flex items-center gap-2 flex-wrap">
				<span class="text-sm font-medium text-gray-700">Risk Level:</span>
				<button
					onclick={() => toggleRiskLevel('critical')}
					class={chipClass(riskLevelFilter.has('critical'))}
				>
					Critical
				</button>
				<button
					onclick={() => toggleRiskLevel('medium')}
					class={chipClass(riskLevelFilter.has('medium'))}
				>
					Medium
				</button>
				<button
					onclick={() => toggleRiskLevel('informational')}
					class={chipClass(riskLevelFilter.has('informational'))}
				>
					Informational
				</button>
			</div>

			<!-- Findings count -->
			<div class="text-sm text-gray-600">
				Showing {filteredFindings.length} of {findings.filter((f) => !f.remediated).length} findings
			</div>
		</div>
	{/if}
```

**Step 3: Run frontend check**

Run: `npm run check`

Expected: No TypeScript errors.

**Step 4: Run dev server and test manually**

Run: `npm run tauri dev`

Test:
1. Navigate to Local PII Discovery
2. Run a scan (if no findings exist)
3. Verify filter chips appear
4. Click chips to verify they toggle selected state
5. Verify findings list updates when filtering

**Step 5: Commit**

```bash
git add src/routes/discovery/+page.svelte
git commit -m "feat(discovery): add filter chip UI

Add clickable filter chips for PII type (Email, Phone, SSN) and
risk level (Critical, Medium, Informational). Chips toggle selection
and update filtered findings list. Shows count of filtered results.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 7: Update Finding Lists to Use Filtered Results

**Files:**
- Modify: `src/routes/discovery/+page.svelte`

**Step 1: Add helper to group filtered findings by source**

Replace the existing grouping (around line 29-31):

```svelte
// Group filtered findings by source
const filesystemFindings = $derived(filteredFindings.filter((f) => f.source === 'filesystem'));
const browserFindings = $derived(filteredFindings.filter((f) => f.source === 'browser'));
const emailFindings = $derived(filteredFindings.filter((f) => f.source === 'email'));
```

**Step 2: Add empty state for filtered results**

Add before the filesystem findings section (around line 235):

```svelte
	{:else}
		<!-- Filter Chips (shown even if no results after filtering) -->
		<div class="mb-6 space-y-3">
			<!-- ... same filter chip UI from Task 6 ... -->
		</div>

		<!-- Summary Cards -->
		<div class="mb-6 grid grid-cols-1 gap-4 md:grid-cols-3">
			<!-- ... existing summary cards ... -->
		</div>

		<!-- Empty state for filtered results -->
		{#if findings.length > 0 && filteredFindings.length === 0}
			<div class="rounded-md bg-gray-50 p-8 text-center">
				<p class="text-gray-600">
					No findings match the selected filters. Try adjusting your selection.
				</p>
			</div>
		{/if}

		<!-- Filesystem Findings -->
		{#if filesystemFindings.length > 0}
			<!-- ... existing filesystem findings rendering ... -->
		{/if}
```

**Step 3: Run frontend check**

Run: `npm run check`

Expected: No TypeScript errors.

**Step 4: Test filtering behavior**

Run: `npm run tauri dev`

Test cases:
1. No filters active → shows all non-remediated findings
2. Select "Email" → shows only email findings
3. Select "Critical" → shows only critical findings
4. Select "Email" + "Critical" → shows only critical email findings
5. Select all filters then deselect all → shows empty state message
6. Verify summary cards update with filtered counts

**Step 5: Commit**

```bash
git add src/routes/discovery/+page.svelte
git commit -m "feat(discovery): apply filters to findings display

Update finding groups to use filteredFindings. Add empty state
message when filters result in zero findings.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 8: Run Full Test Suite

**Step 1: Run backend tests**

Run: `cargo test --workspace`

Expected: All tests pass.

**Step 2: Run frontend tests**

Run: `npm run check`

Expected: No TypeScript errors.

**Step 3: Build full application**

Run: `npm run tauri build`

Expected: Clean build with no errors.

**Step 4: Commit if any fixes were needed**

If fixes were needed in previous steps, commit them:

```bash
git add <fixed-files>
git commit -m "fix: address test failures in discovery filtering

<description of fixes>

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 9: Manual End-to-End Testing

**Step 1: Start development server**

Run: `npm run tauri dev`

**Step 2: Test complete flow**

1. Create/unlock a vault
2. Navigate to Local PII Discovery
3. Run a discovery scan
4. Wait for scan to complete
5. Verify findings appear with pii_type populated
6. Test filter chips:
   - Click "Email" → verify only email findings shown
   - Click "Phone" → verify email + phone findings shown
   - Deselect "Email" → verify only phone findings shown
   - Click "Critical" → verify only critical phone findings shown
   - Deselect all → verify empty state message
7. Verify summary cards update correctly with filters
8. Verify "Showing X of Y findings" count is accurate
9. Test marking finding as remediated → verify it disappears from filtered list

**Step 3: Document any issues found**

If issues are found, fix them and commit:

```bash
git add <fixed-files>
git commit -m "fix: <issue description>

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Verification Checklist

- [ ] Migration 012 exists and backfills pii_type correctly
- [ ] Database tests pass with updated migration count
- [ ] Backend sets pii_type when creating findings
- [ ] Frontend interface includes pii_type field
- [ ] Filter chips render and toggle selection
- [ ] Filtered findings update reactively
- [ ] Summary cards reflect filtered counts
- [ ] Empty state shows when filters result in zero findings
- [ ] Remediated findings never shown (regardless of filters)
- [ ] "Showing X of Y" count is accurate
- [ ] All workspace tests pass
- [ ] Frontend type check passes
- [ ] Manual E2E testing confirms expected behavior

---

## Critical Files

- `crates/spectral-db/migrations/012_discovery_pii_type.sql` - Migration to add pii_type column
- `crates/spectral-db/src/discovery_findings.rs` - Database layer structs and queries
- `src-tauri/src/commands/discovery.rs` - Backend command to set pii_type
- `src/lib/api/discovery.ts` - Frontend TypeScript interface
- `src/routes/discovery/+page.svelte` - Filter chip UI and filtering logic

---

## Edge Cases Handled

1. **Empty filters** - Shows all non-remediated findings
2. **All filters active then all deselected** - Shows empty state message
3. **Remediated findings** - Never shown regardless of filter state
4. **Missing pii_type** - Shouldn't happen after migration, but would be excluded from filtered results
5. **No findings at all** - Shows "No findings yet" message (existing behavior)
6. **Scan running** - Shows loading spinner (existing behavior)
