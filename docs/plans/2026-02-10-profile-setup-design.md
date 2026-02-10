# Profile Setup UI Design - Task 1.6

**Date:** 2026-02-10
**Status:** Draft
**Dependencies:** Task 1.5 (Unlock Screen UI) ✅

## Overview

Design for the multi-step profile setup wizard that collects user PII after vault creation. The profile data enables automated data broker searches and removal requests.

## Architecture Decisions

### Profile Data Structure

**Decision:** Start with minimal required fields, show all future fields as disabled in UI.

**Rationale:**
- YAGNI: Current 5 brokers only need: first_name, last_name, email, address (city, state, zip)
- UX transparency: Show future fields (greyed out) so users know what might be collected later
- Easy evolution: Encrypted blob storage makes schema changes straightforward
- Privacy-first: Users see exactly what data is stored

**Profile Schema (Rust):**
```rust
use serde::{Deserialize, Serialize};
use crate::types::{ProfileId, Timestamp};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: ProfileId,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,

    // Required fields (Phase 1)
    pub first_name: String,
    pub last_name: String,
    pub email: String,

    // Address (required)
    pub street_address: String,
    pub city: String,
    pub state: String,        // US state code (e.g., "CA", "NY")
    pub zip_code: String,

    // Optional but shown (Phase 1)
    pub middle_name: Option<String>,
    pub date_of_birth: Option<String>,  // YYYY-MM-DD format

    // Future fields (disabled in UI for now)
    pub phone_numbers: Vec<PhoneNumber>,
    pub previous_addresses: Vec<Address>,
    pub aliases: Vec<String>,
    pub relatives: Vec<Relative>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneNumber {
    pub number: String,
    pub phone_type: PhoneType,  // Mobile, Home, Work
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub state: String,
    pub zip_code: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relative {
    pub name: String,
    pub relationship: String,  // Mother, Father, Spouse, etc.
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PhoneType {
    Mobile,
    Home,
    Work,
    Other,
}
```

### Wizard Flow

**Decision:** 4-step wizard with validation at each step.

**Steps:**
1. **Basic Info** - Name (first, middle, last), DOB
2. **Contact** - Email, phone (disabled), previous names (disabled)
3. **Address** - Current address, previous addresses (disabled)
4. **Review** - Summary with edit links

**Navigation:**
- "Back" button saves progress and goes to previous step
- "Next" validates current step before proceeding
- "Skip" button (only on optional steps) - saves null/empty values
- Progress indicator shows 1/4, 2/4, 3/4, 4/4

### Data Storage

**Decision:** Single encrypted blob in `profiles` table (already exists).

The profile struct is serialized to JSON, encrypted with ChaCha20-Poly1305, and stored in the `data` column:

```sql
-- From 001_initial.sql (already exists)
CREATE TABLE profiles (
    id TEXT PRIMARY KEY,
    data BLOB NOT NULL,           -- Encrypted JSON profile
    nonce BLOB NOT NULL,           -- 12-byte nonce for AEAD
    created_at TEXT NOT NULL,      -- RFC3339 timestamp
    updated_at TEXT NOT NULL       -- RFC3339 timestamp
);
```

### Validation Rules

**Field validation:**
- **first_name, last_name**: 1-100 chars, letters/spaces/hyphens only
- **middle_name**: 0-100 chars (optional)
- **email**: RFC 5322 basic validation (regex)
- **date_of_birth**: YYYY-MM-DD format, must be 13-120 years ago
- **street_address**: 1-200 chars
- **city**: 1-100 chars, letters/spaces only
- **state**: Must be valid US state code (2-letter abbreviation)
- **zip_code**: 5 digits or 5+4 format (12345 or 12345-6789)

**Privacy validation:**
- No SSN, credit card, or password-like strings detected
- No URLs or script tags (XSS protection)
- Warn if suspicious patterns detected

## Component Architecture

### Tauri Commands (src-tauri/src/commands/profile.rs)

```rust
use tauri::State;
use crate::state::AppState;
use crate::error::CommandError;

#[tauri::command]
async fn profile_create(
    state: State<'_, AppState>,
    vault_id: String,
    profile: ProfileInput,
) -> Result<ProfileId, CommandError> {
    // 1. Get unlocked vault from state
    // 2. Serialize profile to JSON
    // 3. Encrypt with vault key
    // 4. Insert into profiles table
    // 5. Return generated profile ID
}

#[tauri::command]
async fn profile_get(
    state: State<'_, AppState>,
    vault_id: String,
    profile_id: String,
) -> Result<Option<Profile>, CommandError> {
    // 1. Get unlocked vault from state
    // 2. Query profiles table by ID
    // 3. Decrypt data blob
    // 4. Deserialize JSON to Profile
    // 5. Return profile or None
}

#[tauri::command]
async fn profile_update(
    state: State<'_, AppState>,
    vault_id: String,
    profile_id: String,
    updates: ProfileUpdate,
) -> Result<(), CommandError> {
    // 1. Get unlocked vault from state
    // 2. Load existing profile
    // 3. Apply updates (merge)
    // 4. Re-encrypt and save
    // 5. Update updated_at timestamp
}

#[tauri::command]
async fn profile_list(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<Vec<ProfileSummary>, CommandError> {
    // 1. Get unlocked vault from state
    // 2. Query all profiles for this vault
    // 3. Return list with id, first_name, last_name, email only
}

#[derive(Debug, Deserialize)]
pub struct ProfileInput {
    pub first_name: String,
    pub last_name: String,
    pub middle_name: Option<String>,
    pub email: String,
    pub date_of_birth: Option<String>,
    pub street_address: String,
    pub city: String,
    pub state: String,
    pub zip_code: String,
}

#[derive(Debug, Serialize)]
pub struct ProfileSummary {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}
```

### Frontend API (src/lib/api/profile.ts)

```typescript
import { invoke } from '@tauri-apps/api/core';

export interface Profile {
  id: string;
  created_at: string;
  updated_at: string;
  first_name: string;
  last_name: string;
  middle_name?: string;
  email: string;
  date_of_birth?: string;
  street_address: string;
  city: string;
  state: string;
  zip_code: string;
  // Future fields (for type completeness)
  phone_numbers?: PhoneNumber[];
  previous_addresses?: Address[];
  aliases?: string[];
  relatives?: Relative[];
}

export interface ProfileInput {
  first_name: string;
  last_name: string;
  middle_name?: string;
  email: string;
  date_of_birth?: string;
  street_address: string;
  city: string;
  state: string;
  zip_code: string;
}

export async function createProfile(
  vaultId: string,
  profile: ProfileInput
): Promise<string> {
  return invoke('profile_create', { vaultId, profile });
}

export async function getProfile(
  vaultId: string,
  profileId: string
): Promise<Profile | null> {
  return invoke('profile_get', { vaultId, profileId });
}

export async function updateProfile(
  vaultId: string,
  profileId: string,
  updates: Partial<ProfileInput>
): Promise<void> {
  return invoke('profile_update', { vaultId, profileId, updates });
}

export async function listProfiles(
  vaultId: string
): Promise<ProfileSummary[]> {
  return invoke('profile_list', { vaultId });
}
```

### Wizard Components

**Component structure:**
```
src/lib/components/profile/
├── ProfileWizard.svelte          # Container with step management
├── steps/
│   ├── BasicInfo.svelte          # Step 1: Name, DOB
│   ├── ContactInfo.svelte        # Step 2: Email, phone (disabled)
│   ├── AddressInfo.svelte        # Step 3: Current address, previous (disabled)
│   └── ReviewStep.svelte         # Step 4: Summary with edit links
└── shared/
    ├── FormField.svelte          # Reusable input with validation
    ├── StateSelect.svelte        # US state dropdown
    ├── ProgressBar.svelte        # 1/4, 2/4, 3/4, 4/4 indicator
    └── DisabledFieldNotice.svelte # "Coming soon" badge
```

**ProfileWizard.svelte state (Svelte 5 runes):**
```typescript
let currentStep = $state(1);
let formData = $state<Partial<ProfileInput>>({});
let errors = $state<Record<string, string>>({});
let loading = $state(false);

const steps = [
  { number: 1, title: 'Basic Info', component: BasicInfo },
  { number: 2, title: 'Contact', component: ContactInfo },
  { number: 3, title: 'Address', component: AddressInfo },
  { number: 4, title: 'Review', component: ReviewStep },
];

async function nextStep() {
  if (!validateCurrentStep()) return;
  if (currentStep < 4) {
    currentStep++;
  } else {
    await submitProfile();
  }
}

function prevStep() {
  if (currentStep > 1) currentStep--;
}

async function submitProfile() {
  loading = true;
  try {
    const profileId = await createProfile(vaultStore.currentVaultId!, formData as ProfileInput);
    // Navigate to dashboard
    goto('/dashboard');
  } catch (err) {
    errors.general = err.message;
  } finally {
    loading = false;
  }
}
```

### Form Validation

**Validation approach:**
- **Client-side**: Immediate feedback (on blur, real-time for some fields)
- **Backend validation**: Commands validate again before storing
- **Error display**: Inline below field, summary at top of step

**Example validation (email):**
```typescript
function validateEmail(email: string): string | null {
  if (!email) return 'Email is required';
  const regex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  if (!regex.test(email)) return 'Please enter a valid email address';
  if (email.length > 255) return 'Email is too long';
  return null;
}
```

**US State validation:**
```typescript
const US_STATES = [
  'AL', 'AK', 'AZ', 'AR', 'CA', 'CO', 'CT', 'DE', 'FL', 'GA',
  'HI', 'ID', 'IL', 'IN', 'IA', 'KS', 'KY', 'LA', 'ME', 'MD',
  'MA', 'MI', 'MN', 'MS', 'MO', 'MT', 'NE', 'NV', 'NH', 'NJ',
  'NM', 'NY', 'NC', 'ND', 'OH', 'OK', 'OR', 'PA', 'RI', 'SC',
  'SD', 'TN', 'TX', 'UT', 'VT', 'VA', 'WA', 'WV', 'WI', 'WY'
];

function validateState(state: string): string | null {
  if (!state) return 'State is required';
  if (!US_STATES.includes(state.toUpperCase())) {
    return 'Please select a valid US state';
  }
  return null;
}
```

## UI/UX Details

### Disabled Fields Display

**Visual treatment:**
```svelte
<!-- Phone number field (disabled for Phase 1) -->
<div class="relative">
  <label class="block text-sm font-medium text-gray-700 mb-2">
    Phone Number
    <span class="ml-2 text-xs text-blue-600 font-normal">Coming in Phase 2</span>
  </label>
  <input
    type="tel"
    disabled
    placeholder="(555) 123-4567"
    class="w-full px-3 py-2 border border-gray-200 rounded-md bg-gray-50 text-gray-400 cursor-not-allowed"
  />
  <div class="absolute top-0 right-0 mt-8 mr-3">
    <span class="text-xs text-gray-400">🔒</span>
  </div>
</div>
```

### Progress Indicator

```svelte
<div class="flex items-center justify-between mb-8">
  {#each steps as step}
    <div class="flex items-center">
      <div
        class="w-10 h-10 rounded-full flex items-center justify-center
               {currentStep === step.number ? 'bg-blue-600 text-white' :
                currentStep > step.number ? 'bg-green-600 text-white' :
                'bg-gray-200 text-gray-500'}"
      >
        {currentStep > step.number ? '✓' : step.number}
      </div>
      {#if step.number < steps.length}
        <div class="w-16 h-1 {currentStep > step.number ? 'bg-green-600' : 'bg-gray-200'}"></div>
      {/if}
    </div>
  {/each}
</div>
```

### Review Step

**Display format:**
```svelte
<div class="space-y-6">
  <h2 class="text-2xl font-bold">Review Your Profile</h2>

  <div class="bg-white rounded-lg border border-gray-200 p-6 space-y-4">
    <div class="flex justify-between items-start">
      <div>
        <h3 class="font-medium text-gray-900">Basic Information</h3>
        <dl class="mt-2 space-y-1">
          <div class="flex gap-2">
            <dt class="text-sm text-gray-500 w-24">Name:</dt>
            <dd class="text-sm text-gray-900">{formData.first_name} {formData.middle_name} {formData.last_name}</dd>
          </div>
          <div class="flex gap-2">
            <dt class="text-sm text-gray-500 w-24">DOB:</dt>
            <dd class="text-sm text-gray-900">{formData.date_of_birth || 'Not provided'}</dd>
          </div>
        </dl>
      </div>
      <button onclick={() => currentStep = 1} class="text-sm text-blue-600 hover:underline">
        Edit
      </button>
    </div>

    <!-- Similar sections for Contact and Address -->
  </div>

  <div class="bg-blue-50 border border-blue-200 rounded-lg p-4">
    <p class="text-sm text-blue-900">
      🔒 This information is encrypted and stored locally on your device.
      It will be used to search for your information on data broker sites and request removal.
    </p>
  </div>
</div>
```

## Routing Integration

### When to Show Wizard

**Decision:** Show wizard after vault creation OR when no profile exists.

**Implementation:**
```typescript
// In src/routes/+layout.svelte or similar
$effect(() => {
  if (vaultStore.isCurrentVaultUnlocked) {
    // Check if profile exists
    profileStore.loadProfiles(vaultStore.currentVaultId!);

    if (!profileStore.hasProfiles) {
      // Redirect to profile setup
      goto('/profile/setup');
    }
  }
});
```

**Routes:**
- `/profile/setup` - ProfileWizard (new users)
- `/profile/edit/:id` - Edit existing profile (reuse wizard components)
- `/profile/list` - List all profiles (family use case, future)

## Error Handling

### Backend Errors

**Error codes:**
- `VAULT_LOCKED` - Vault must be unlocked to access profiles
- `PROFILE_NOT_FOUND` - Profile ID doesn't exist
- `VALIDATION_ERROR` - Invalid profile data
- `ENCRYPTION_FAILED` - Failed to encrypt profile data
- `DATABASE_ERROR` - Failed to store profile

**Frontend handling:**
```typescript
try {
  await createProfile(vaultId, formData);
} catch (err: any) {
  if (err.code === 'VAULT_LOCKED') {
    // Redirect to unlock screen
    goto('/');
  } else if (err.code === 'VALIDATION_ERROR') {
    // Show field-level errors
    errors = parseValidationErrors(err.details);
  } else {
    // Generic error message
    errors.general = 'Failed to save profile. Please try again.';
  }
}
```

## Testing Strategy

### Unit Tests (Rust)

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_profile_create_and_retrieve() {
        // 1. Create vault and unlock
        // 2. Create profile with valid data
        // 3. Retrieve profile by ID
        // 4. Verify all fields match
    }

    #[tokio::test]
    async fn test_profile_encryption() {
        // 1. Create profile with known data
        // 2. Read raw database blob
        // 3. Verify it's encrypted (not readable)
        // 4. Decrypt and verify matches original
    }

    #[tokio::test]
    async fn test_profile_validation() {
        // 1. Try invalid email
        // 2. Try invalid state code
        // 3. Try empty required fields
        // 4. Verify all return validation errors
    }
}
```

### Integration Tests (Frontend)

```typescript
// Using Playwright or similar
test('profile wizard completes successfully', async ({ page }) => {
  // 1. Create and unlock vault
  // 2. Navigate to /profile/setup
  // 3. Fill out Step 1 (basic info)
  // 4. Click Next
  // 5. Fill out Step 2 (contact)
  // 6. Click Next
  // 7. Fill out Step 3 (address)
  // 8. Click Next
  // 9. Review and submit
  // 10. Verify redirect to dashboard
  // 11. Verify profile was created
});

test('wizard validates required fields', async ({ page }) => {
  // 1. Try to proceed without filling fields
  // 2. Verify error messages appear
  // 3. Fill fields
  // 4. Verify errors clear
});

test('back button preserves data', async ({ page }) => {
  // 1. Fill Step 1
  // 2. Go to Step 2
  // 3. Click Back
  // 4. Verify Step 1 data still there
});
```

## Acceptance Criteria

- [ ] Profile struct defined in spectral-core or spectral-vault
- [ ] Four Tauri commands implemented (create, get, update, list)
- [ ] TypeScript API wrappers with correct types
- [ ] ProfileWizard component with 4 steps
- [ ] All required fields have validation
- [ ] Disabled fields shown with "Coming soon" indicators
- [ ] Progress indicator shows current step
- [ ] Back button preserves entered data
- [ ] Review step shows all data with edit links
- [ ] Profile saves successfully to encrypted database
- [ ] Success redirects to dashboard
- [ ] All tests pass (Rust + frontend)
- [ ] No TypeScript errors
- [ ] Clippy passes with no warnings

## Follow-up Tasks

After Task 1.6 completion:
- **Task 1.7:** Broker Definitions Crate - Load and validate broker TOML files
- **Task 1.8:** LLM Abstraction Layer - Multi-provider LLM routing
- **Task 2.1:** Search Engine - Use profile data to search brokers
- **Phase 2:** Add phone numbers, previous addresses, aliases to profile

## Notes

- International address support is out of scope for Phase 1 (US only)
- Multiple profiles per vault is supported in backend but UI shows single profile for now
- SSN field intentionally omitted (not needed for data broker removal)
- Photo upload is future work (requires image storage design)
