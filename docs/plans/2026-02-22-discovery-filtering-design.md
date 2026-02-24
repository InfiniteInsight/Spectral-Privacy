# Discovery Findings Filtering Design

## Overview

Add filter chip UI to the Local PII Discovery page allowing users to filter findings by PII type (Email, Phone, SSN) and risk level (Critical, Medium, Informational). Users can combine filters to narrow results, e.g., "show all critical phone number findings."

## Problem Statement

The discovery results page shows all findings in a flat list with no way to filter or search. Users need to manually scroll through to find specific types of PII or focus on high-priority items (critical findings).

Example user requests:
- "Show me all files with phone numbers"
- "Show me all critical findings"
- "Show me all email addresses that are medium risk"

## Solution Overview

Add a `pii_type` column to the database to store PII type explicitly (email, phone, ssn), then implement clickable filter chips in the UI that work client-side on the loaded findings list.

## Architecture

### Database Schema Change

Add `pii_type TEXT` column to `discovery_findings` table to store the type of PII detected.

**Migration strategy:**
- Add nullable `pii_type` column
- Backfill existing findings by parsing description field:
  - "Email address found in..." → pii_type = "email"
  - "Phone number found in..." → pii_type = "phone"
  - "Social Security Number found in..." → pii_type = "ssn"
- Make column NOT NULL after backfill

### Backend Changes

**File: `src-tauri/src/commands/discovery.rs`**
- Update `insert_pii_finding()` to set `pii_type` based on the `PiiMatch` enum variant
- Map `PiiMatch::Email` → "email", `PiiMatch::Phone` → "phone", `PiiMatch::Ssn` → "ssn"

**File: `crates/spectral-db/src/discovery_findings.rs`**
- Add `pii_type` field to `CreateDiscoveryFinding` struct
- Update insert query to include pii_type column

**No API changes needed** - frontend filters the full findings list client-side.

### Frontend Changes

**File: `src/lib/api/discovery.ts`**
- Add `pii_type: 'email' | 'phone' | 'ssn'` to `DiscoveryFinding` interface

**File: `src/routes/discovery/+page.svelte`**
- Add filter state:
  - `piiTypeFilter: Set<string>` - selected PII types
  - `riskLevelFilter: Set<string>` - selected risk levels
- Add filter chip UI above summary cards
- Add derived `filteredFindings` that applies active filters
- Update all findings iterations to use `filteredFindings` instead of raw `findings`

## Components

### Filter Chip Bar

**Layout:**
```
[PII Type:] [Email] [Phone] [SSN]    [Risk Level:] [Critical] [Medium] [Informational]

Showing X of Y findings
```

**Behavior:**
- Chips are toggleable (click to select/deselect)
- Selected chips have different styling (darker background, white text)
- Unselected chips are light gray with dark text
- Multiple chips within a group can be selected simultaneously
- Empty selection = show all (no filter applied)

**Filter logic:**
- Finding is visible if it matches ALL active filter groups
- Within a group, finding matches if it matches ANY selected chip (OR logic)
- Example: [Email, Phone] + [Critical] = show critical emails OR critical phones
- Example: [Phone] + [Critical, Medium] = show critical phones OR medium phones

### Derived Filtering

```typescript
const filteredFindings = $derived(
  findings.filter(f => {
    if (f.remediated) return false; // Never show remediated

    const piiMatch = piiTypeFilter.size === 0 || piiTypeFilter.has(f.pii_type);
    const riskMatch = riskLevelFilter.size === 0 || riskLevelFilter.has(f.risk_level);

    return piiMatch && riskMatch;
  })
);
```

### Summary Cards Update

Update the summary cards to reflect filtered counts:
- Critical count from `filteredFindings` not all `findings`
- Medium count from `filteredFindings`
- Informational count from `filteredFindings`

## Data Flow

1. User clicks "Run Discovery Scan"
2. Backend scans files, creates findings with `pii_type` set from `PiiMatch` enum
3. Frontend loads all findings via `getDiscoveryFindings()`
4. User clicks filter chips (e.g., "Phone" + "Critical")
5. Frontend updates `piiTypeFilter` and `riskLevelFilter` sets
6. Svelte reactivity recomputes `filteredFindings` derived state
7. UI updates to show only matching findings and summary counts

## UI Mockup

```
┌─────────────────────────────────────────────────────────┐
│ Local PII Discovery                  [Run Discovery Scan]│
├─────────────────────────────────────────────────────────┤
│ PII Type: [Email] [Phone] [SSN]                         │
│ Risk: [Critical] [Medium] [Informational]               │
│                                                          │
│ Showing 3 of 15 findings                                │
├─────────────────────────────────────────────────────────┤
│ ┌─────────┐ ┌─────────┐ ┌──────────────┐              │
│ │    2    │ │    1    │ │      0       │              │
│ │Critical │ │ Medium  │ │Informational │              │
│ └─────────┘ └─────────┘ └──────────────┘              │
├─────────────────────────────────────────────────────────┤
│ Filesystem                                               │
│ ┌───────────────────────────────────────────────────┐  │
│ │ Phone number found in: contacts.txt    [critical] │  │
│ │ /Users/evan/Documents/contacts.txt                │  │
│ └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

## Error Handling

**Empty state:**
- If filters result in zero findings, show:
  "No findings match the selected filters. Try adjusting your selection."

**Missing pii_type (shouldn't happen after migration):**
- Findings without `pii_type` are excluded from filter results
- Log warning to console

## Testing Strategy

**Database migration:**
- Test backfill correctly parses all three PII types from descriptions
- Verify existing findings get correct pii_type values
- Test new findings have pii_type set on creation

**Frontend filtering:**
- Test single filter (just "Email")
- Test combined filters ("Phone" + "Critical")
- Test empty filters (show all)
- Test "Select all" → "Deselect all" → nothing shown
- Test filter interaction with remediated findings (never shown)
- Test summary card counts update with filters

## Future Enhancements (Out of Scope)

- "Clear all filters" button
- Filter state persisted to localStorage
- "Show remediated" toggle
- Text search within file paths
- Export filtered results to CSV

## Success Criteria

- User can click PII type chips to filter findings
- User can click risk level chips to filter findings
- Filters can be combined (e.g., critical emails only)
- Summary cards reflect filtered counts
- All existing findings correctly backfilled with pii_type
- New findings automatically have pii_type set
