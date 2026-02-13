# Phase 3: Scan UI Components - Design Document

**Date:** 2026-02-13
**Status:** Approved
**Phase:** Phase 3 of Manual Scan Trigger (Task 1.7)

## Overview

Design for user-facing scan workflow UI: initiate scans, track progress, review findings, and queue removals.

**Dependencies:**
- Phase 1: Scan Orchestrator (merged) ✅
- Phase 2: Browser Integration (merged) ✅

**Goal:** Enable users to discover their PII on data broker sites and queue removal requests.

## Architecture

### Page Structure

```
/scan/start              - Scan initiation page
/scan/progress/[id]      - Real-time progress tracking
/scan/review/[id]        - Findings verification interface
/removals                - Removal queue placeholder
```

### Navigation Flow

```
Dashboard
  ↓ (Click "Scan for Your Data")
/scan/start
  ↓ (Select profile, click "Start Scan")
/scan/progress/[jobId]
  ↓ (Poll every 2s, auto-advance when complete)
/scan/review/[jobId]
  ↓ (Confirm/reject findings, click "Submit Removals")
/removals
  ↓ (Show success, Phase 4 will add tracking)
Dashboard
```

### State Management

**Scan Store** (`src/lib/stores/scan.ts`):
```typescript
interface ScanStore {
  currentScanId: string | null;
  scanStatus: ScanJobStatus | null;
  findings: Finding[];
  loading: boolean;
  error: string | null;

  // Actions
  startScan(profileId: string): Promise<string>;
  pollStatus(scanJobId: string): void;
  stopPolling(): void;
  loadFindings(scanJobId: string): Promise<void>;
  verifyFinding(findingId: string, isMatch: boolean): Promise<void>;
  submitRemovals(scanJobId: string): Promise<void>;
}
```

### Polling Strategy

**Progress page only:**
- Start polling on mount: `setInterval(checkStatus, 2000)`
- Stop polling on unmount or when status is terminal (Completed/Failed/Cancelled)
- Show loading spinner while fetching
- Auto-navigate to review page when status = "Completed"

**No polling elsewhere** - other pages fetch once on mount.

## Component Design

### 1. Scan Start Page (`/scan/start/+page.svelte`)

**Purpose:** Initiate a new scan job

**UI Elements:**
- Page title: "Scan for Your Data"
- Profile selector (dropdown if multiple profiles exist)
- "Start Scan" button (primary CTA)
- Info box explaining what will happen:
  - "We'll search X data brokers for your information"
  - "This takes 2-5 minutes"
  - "You'll review results before any removal requests"

**Behavior:**
1. Load profiles from profileStore
2. Pre-select first profile (or show dropdown if multiple)
3. On "Start Scan":
   - Call `scanAPI.start(profileId, "All")`
   - Navigate to `/scan/progress/[jobId]`
   - Handle errors (show error message inline)

**Edge Cases:**
- No profile exists → Show message + button to create profile
- Scan already in progress → Show existing scan status + link to progress page
- Vault locked → Redirect to unlock screen

### 2. Progress Page (`/scan/progress/[id]/+page.svelte`)

**Purpose:** Show real-time scan progress

**UI Elements:**
- Progress header: "Scanning X of Y brokers..."
- Overall progress bar (completed_brokers / total_brokers)
- Broker-by-broker status list:
  - Broker name + logo/icon
  - Status badge (Pending, In Progress, Success, Failed, Skipped)
  - Findings count (if available)
  - Error message (if failed)
- Auto-refresh indicator: "Updating..." (shown during polls)

**Status Badge Colors:**
- Pending: Gray
- In Progress: Blue (animated spinner)
- Success: Green (✓)
- Failed: Red (✗)
- Skipped: Yellow (⊘)

**Behavior:**
1. Load scan job ID from route params
2. Fetch initial status: `scanAPI.getStatus(jobId)`
3. Start polling every 2 seconds
4. Update UI with latest status
5. When status = "Completed":
   - Stop polling
   - Show success message: "Scan complete! Found X results."
   - Auto-navigate to review page after 2 seconds
6. When status = "Failed":
   - Stop polling
   - Show error message
   - Offer "Try Again" button

**Performance:**
- Only poll while page is visible (use document visibility API)
- Clear interval on component unmount
- Debounce rapid status updates

### 3. Review Page (`/scan/review/[id]/+page.svelte`)

**Purpose:** User verifies which findings are accurate matches

**UI Structure:**

```
Header:
  "Review Findings (X total)"
  "Select which results are you, then submit for removal"

Grouped Table:
  Group 1: BeenVerified (Y findings)
    ├── Finding 1: [Name, Age, Location] | [View Details] [Confirm] [Reject]
    ├── Finding 2: [Name, Age, Location] | [View Details] [Confirm] [Reject]
    └── ...
  Group 2: Whitepages (Z findings)
    ├── Finding 1: ...
    └── ...

Footer:
  "X of Y confirmed" | [Submit Removals] (disabled until at least 1 confirmed)
```

**Findings Display:**
- **Compact view** (default): Name, age, city/state, phone numbers (first 2)
- **Expanded view** (click "View Details"): Full extracted data + listing URL (external link)

**Verification Actions:**
- **Confirm button** (green): Mark as verified match
- **Reject button** (gray): Mark as not a match
- **Bulk actions** (per broker group):
  - "Confirm All" / "Reject All" buttons at group header
  - Checkbox select mode (optional, nice-to-have)

**State Tracking:**
- Track verification_status per finding (local state)
- Batch API calls: `scanAPI.verify(findingId, isMatch)` on each action
- Count confirmed findings for footer display
- Enable "Submit Removals" button when at least 1 confirmed

**Behavior:**
1. Load findings: `scanAPI.getFindings(jobId, 'PendingVerification')`
2. Group findings by broker_id
3. Show count per broker group
4. On verify action:
   - Call API to update finding
   - Update local state immediately (optimistic update)
   - Update counts
5. On "Submit Removals":
   - Call `scanAPI.submitRemovals(jobId)`
   - Navigate to `/removals` with success message

**Edge Cases:**
- No findings → Show "No results found" message + button to return to dashboard
- All rejected → Disable submit button, show message
- Network error during verify → Show error, allow retry

### 4. Removals Placeholder (`/removals/+page.svelte`)

**Purpose:** Acknowledge removal submission (Phase 4 will expand this)

**UI Elements:**
- Success icon (✓)
- Header: "Removal Requests Submitted"
- Message: "We've queued X removal requests. You'll be notified when they're processed."
- Info box: "This feature is in early access. Removal tracking coming soon."
- Button: "Return to Dashboard"

**Behavior:**
- Static page, no API calls in Phase 3
- Pass removal count via query param: `/removals?count=5`
- Extract count from URL and display

## Dashboard Integration

**Update `/routes/+page.svelte`:**

Replace "Coming Soon" section with:

```svelte
<div class="mt-6">
  <a
    href="/scan/start"
    class="block px-6 py-4 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 transition-colors text-center"
  >
    Scan for Your Data
  </a>
  <p class="text-sm text-gray-500 mt-2 text-center">
    Search data brokers for your information
  </p>
</div>
```

## Component Reuse

**From ProfileWizard pattern:**
- Multi-step navigation (adapt for scan states)
- Progress indicators
- Form validation patterns
- Error handling (inline errors, toast notifications)

**New Reusable Components:**
- `ScanProgressBar.svelte` - Animated progress bar with percentage
- `BrokerStatusBadge.svelte` - Colored status badges
- `FindingCard.svelte` - Individual finding display with actions
- `BrokerGroup.svelte` - Collapsible group header + findings list

## Error Handling

**Scan Start Errors:**
- No profile → Redirect to profile setup
- Scan already running → Show existing scan link
- Browser initialization failed → Show error + retry button

**Progress Page Errors:**
- Polling fails → Show warning, keep retrying (exponential backoff)
- Scan job not found → Redirect to scan start
- Network offline → Show offline indicator

**Review Page Errors:**
- Findings fetch fails → Show error + retry button
- Verify action fails → Show inline error, allow retry
- Submit fails → Show error modal, keep findings state

**Global Error Strategy:**
- Use toast notifications for non-critical errors
- Use inline errors for form validation
- Use error pages for critical failures (404, 500)

## Accessibility

- Keyboard navigation for all interactive elements
- ARIA labels for status badges and icons
- Focus management (auto-focus primary actions)
- Screen reader announcements for poll updates (aria-live)
- High contrast mode support

## Testing Strategy

**Unit Tests:**
- Scan store actions (start, poll, verify, submit)
- Component rendering with various states
- Error handling paths

**Integration Tests:**
- Full scan workflow (start → progress → review → submit)
- Polling behavior (start, stop, auto-advance)
- Findings verification flow

**Manual Testing:**
- Test with 0, 1, 5, 20+ findings
- Test with all brokers succeeding
- Test with some brokers failing
- Test network interruptions during scan
- Test concurrent scans (multiple tabs)

## Phase 4 Preview

**Removal Tracking (Phase 4):**
- `/removals` becomes full tracking page
- Real-time status per removal request
- Email notification integration
- Retry failed removals
- Removal history/timeline

**Advanced Features (Phase 4+):**
- Scan scheduling (weekly/monthly scans)
- Scan history (view past scans)
- Comparison view (what changed since last scan)
- Export findings (CSV/PDF)
- Custom broker selection (advanced mode)

## Open Questions

None - design approved and ready for implementation.

---

**Next Steps:**
1. Create implementation plan with task breakdown
2. Set up git worktree for Phase 3 development
3. Execute with subagent-driven development
4. Review and merge
