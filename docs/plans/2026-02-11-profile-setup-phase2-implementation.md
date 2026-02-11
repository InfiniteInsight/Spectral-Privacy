# Profile Setup UI - Phase 2 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add phone numbers, previous addresses, aliases, and relatives to the profile wizard with completeness scoring.

**Architecture:** Extend existing UserProfile schema with Phase 2 collections, add completeness scoring system, expand wizard from 4 to 5 steps with hybrid input patterns (dynamic rows for simple fields, modals for complex fields).

**Tech Stack:** Rust (spectral-vault), TypeScript, SvelteKit, Svelte 5 runes, ChaCha20-Poly1305 encryption, SQLCipher

---

## Task 1: Add Phase 2 Types to Backend Schema

**Goal:** Define new Rust types for phone numbers, previous addresses, and relatives with encryption support.

**Files:**
- Modify: `crates/spectral-vault/src/profile.rs:1-60`
- Test: `crates/spectral-vault/src/profile.rs:214-402`

### Step 1: Write failing test for PhoneNumber type

Add to `crates/spectral-vault/src/profile.rs` after line 401:

```rust
#[test]
fn test_phone_number_serialization() {
    let key = test_key();
    let phone = PhoneNumber {
        number: encrypt_string("555-123-4567", &key).expect("encrypt"),
        phone_type: PhoneType::Mobile,
    };

    let json = serde_json::to_string(&phone).expect("serialize");
    let deserialized: PhoneNumber = serde_json::from_str(&json).expect("deserialize");

    let decrypted = deserialized.number.decrypt(&key).expect("decrypt");
    assert_eq!(decrypted, "555-123-4567");
    assert_eq!(deserialized.phone_type, PhoneType::Mobile);
}
```

### Step 2: Run test to verify it fails

Run: `cargo test test_phone_number_serialization`

Expected: Compilation error "cannot find type `PhoneNumber` in this scope"

### Step 3: Add PhoneType enum

Add after line 60 in `crates/spectral-vault/src/profile.rs`:

```rust
/// Type of phone number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PhoneType {
    /// Mobile/cell phone
    Mobile,
    /// Home landline
    Home,
    /// Work phone
    Work,
}
```

### Step 4: Add PhoneNumber struct

Add after PhoneType enum:

```rust
/// Phone number with type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneNumber {
    /// Encrypted phone number
    pub number: EncryptedField<String>,
    /// Type of phone number
    pub phone_type: PhoneType,
}
```

### Step 5: Run test to verify it passes

Run: `cargo test test_phone_number_serialization`

Expected: `test test_phone_number_serialization ... ok`

### Step 6: Write failing test for PreviousAddress type

Add after previous test:

```rust
#[test]
fn test_previous_address_serialization() {
    let key = test_key();
    let addr = PreviousAddress {
        address_line1: encrypt_string("123 Main St", &key).expect("encrypt"),
        address_line2: Some(encrypt_string("Apt 4", &key).expect("encrypt")),
        city: encrypt_string("Springfield", &key).expect("encrypt"),
        state: encrypt_string("IL", &key).expect("encrypt"),
        zip_code: encrypt_string("62701", &key).expect("encrypt"),
        lived_from: Some("2020-01-01".to_string()),
        lived_to: Some("2022-12-31".to_string()),
    };

    let json = serde_json::to_string(&addr).expect("serialize");
    let deserialized: PreviousAddress = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(deserialized.lived_from, Some("2020-01-01".to_string()));
}
```

### Step 7: Run test to verify it fails

Run: `cargo test test_previous_address_serialization`

Expected: Compilation error "cannot find type `PreviousAddress` in this scope"

### Step 8: Add PreviousAddress struct

Add after PhoneNumber struct:

```rust
/// Previous address with date range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviousAddress {
    /// Address line 1 (street)
    pub address_line1: EncryptedField<String>,
    /// Address line 2 (apt/suite) - optional
    pub address_line2: Option<EncryptedField<String>>,
    /// City
    pub city: EncryptedField<String>,
    /// State/province
    pub state: EncryptedField<String>,
    /// ZIP/postal code
    pub zip_code: EncryptedField<String>,
    /// Start date (YYYY-MM-DD format)
    pub lived_from: Option<String>,
    /// End date (YYYY-MM-DD format)
    pub lived_to: Option<String>,
}
```

### Step 9: Run test to verify it passes

Run: `cargo test test_previous_address_serialization`

Expected: `test test_previous_address_serialization ... ok`

### Step 10: Write failing test for Relative type

Add after previous test:

```rust
#[test]
fn test_relative_serialization() {
    let key = test_key();
    let relative = Relative {
        name: encrypt_string("Jane Doe", &key).expect("encrypt"),
        relationship: RelationshipType::Spouse,
    };

    let json = serde_json::to_string(&relative).expect("serialize");
    let deserialized: Relative = serde_json::from_str(&json).expect("deserialize");

    let decrypted_name = deserialized.name.decrypt(&key).expect("decrypt");
    assert_eq!(decrypted_name, "Jane Doe");
    assert_eq!(deserialized.relationship, RelationshipType::Spouse);
}
```

### Step 11: Run test to verify it fails

Run: `cargo test test_relative_serialization`

Expected: Compilation error "cannot find type `Relative` in this scope"

### Step 12: Add RelationshipType enum

Add after PreviousAddress struct:

```rust
/// Type of family relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum RelationshipType {
    /// Spouse (married)
    Spouse,
    /// Partner (not married)
    Partner,
    /// Parent
    Parent,
    /// Child
    Child,
    /// Sibling
    Sibling,
    /// Other relationship
    Other,
}
```

### Step 13: Add Relative struct

Add after RelationshipType enum:

```rust
/// Family member or relative.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relative {
    /// Encrypted name
    pub name: EncryptedField<String>,
    /// Type of relationship
    pub relationship: RelationshipType,
}
```

### Step 14: Run test to verify it passes

Run: `cargo test test_relative_serialization`

Expected: `test test_relative_serialization ... ok`

### Step 15: Run all tests

Run: `cargo test --package spectral-vault`

Expected: All tests pass

### Step 16: Commit Phase 2 types

```bash
git add crates/spectral-vault/src/profile.rs
git commit -m "feat(vault): add Phase 2 profile types (PhoneNumber, PreviousAddress, Relative)"
```

---

## Task 2: Update UserProfile Schema with Phase 2 Fields

**Goal:** Add Phase 2 collection fields to UserProfile struct.

**Files:**
- Modify: `crates/spectral-vault/src/profile.rs:12-60`
- Test: `crates/spectral-vault/src/profile.rs:245-286`

### Step 1: Write failing test for Phase 2 fields

Add to test section:

```rust
#[tokio::test]
async fn test_profile_with_phase2_fields() {
    let key = test_key();
    let db = Database::new(":memory:", key.to_vec())
        .await
        .expect("create database");
    db.run_migrations().await.expect("run migrations");

    let id = ProfileId::generate();
    let mut profile = UserProfile::new(id.clone());

    // Add Phase 2 fields
    profile.phone_numbers = vec![PhoneNumber {
        number: encrypt_string("555-1234", &key).expect("encrypt"),
        phone_type: PhoneType::Mobile,
    }];

    profile.aliases = vec![encrypt_string("Johnny", &key).expect("encrypt")];

    profile.relatives = vec![Relative {
        name: encrypt_string("Jane", &key).expect("encrypt"),
        relationship: RelationshipType::Spouse,
    }];

    // Save and reload
    profile.save(&db, &key).await.expect("save");
    let loaded = UserProfile::load(&db, &id, &key).await.expect("load");

    assert_eq!(loaded.phone_numbers.len(), 1);
    assert_eq!(loaded.aliases.len(), 1);
    assert_eq!(loaded.relatives.len(), 1);
}
```

### Step 2: Run test to verify it fails

Run: `cargo test test_profile_with_phase2_fields`

Expected: Compilation error "no field `phone_numbers` on type `UserProfile`"

### Step 3: Add Phase 2 fields to UserProfile struct

Replace lines 12-60 with:

```rust
/// User profile with encrypted PII fields.
///
/// All personally identifiable information is stored as `EncryptedField<T>`,
/// ensuring that PII is encrypted at rest in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    /// Unique profile identifier
    pub id: ProfileId,
    /// Full name
    pub full_name: Option<EncryptedField<String>>,
    /// First name
    pub first_name: Option<EncryptedField<String>>,
    /// Middle name
    pub middle_name: Option<EncryptedField<String>>,
    /// Last name
    pub last_name: Option<EncryptedField<String>>,
    /// Email address
    pub email: Option<EncryptedField<String>>,
    /// Phone number (legacy single phone - deprecated in favor of phone_numbers)
    #[deprecated(note = "Use phone_numbers instead")]
    pub phone: Option<EncryptedField<String>>,
    /// Street address
    pub address: Option<EncryptedField<String>>,
    /// City
    pub city: Option<EncryptedField<String>>,
    /// State/province
    pub state: Option<EncryptedField<String>>,
    /// ZIP/postal code
    pub zip_code: Option<EncryptedField<String>>,
    /// Country
    pub country: Option<EncryptedField<String>>,
    /// Date of birth (ISO 8601 format)
    pub date_of_birth: Option<EncryptedField<String>>,
    /// Social Security Number
    pub ssn: Option<EncryptedField<String>>,
    /// Employer/company
    pub employer: Option<EncryptedField<String>>,
    /// Job title
    pub job_title: Option<EncryptedField<String>>,
    /// Educational institution
    pub education: Option<EncryptedField<String>>,
    /// Social media usernames (JSON array)
    pub social_media: Option<EncryptedField<Vec<String>>>,
    /// Previous addresses (legacy - deprecated in favor of previous_addresses)
    #[deprecated(note = "Use previous_addresses instead")]
    pub previous_addresses: Option<EncryptedField<Vec<String>>>,

    // Phase 2 fields
    /// Multiple phone numbers with types
    #[serde(default)]
    pub phone_numbers: Vec<PhoneNumber>,
    /// Previous addresses with date ranges
    #[serde(default, rename = "previous_addresses_v2")]
    pub previous_addresses_v2: Vec<PreviousAddress>,
    /// Aliases (former names, nicknames)
    #[serde(default)]
    pub aliases: Vec<EncryptedField<String>>,
    /// Family members and relatives
    #[serde(default)]
    pub relatives: Vec<Relative>,

    /// Profile creation timestamp
    pub created_at: Timestamp,
    /// Profile last update timestamp
    pub updated_at: Timestamp,
}
```

### Step 4: Update UserProfile::new() to initialize Phase 2 fields

Update the `new()` method (around line 65):

```rust
#[must_use]
pub fn new(id: ProfileId) -> Self {
    let now = Timestamp::now();
    Self {
        id,
        full_name: None,
        first_name: None,
        middle_name: None,
        last_name: None,
        email: None,
        phone: None,
        address: None,
        city: None,
        state: None,
        zip_code: None,
        country: None,
        date_of_birth: None,
        ssn: None,
        employer: None,
        job_title: None,
        education: None,
        social_media: None,
        previous_addresses: None,
        // Phase 2 fields
        phone_numbers: Vec::new(),
        previous_addresses_v2: Vec::new(),
        aliases: Vec::new(),
        relatives: Vec::new(),
        created_at: now,
        updated_at: now,
    }
}
```

### Step 5: Run test to verify it passes

Run: `cargo test test_profile_with_phase2_fields`

Expected: `test test_profile_with_phase2_fields ... ok`

### Step 6: Run all vault tests

Run: `cargo test --package spectral-vault`

Expected: All tests pass

### Step 7: Commit schema update

```bash
git add crates/spectral-vault/src/profile.rs
git commit -m "feat(vault): add Phase 2 fields to UserProfile schema"
```

---

## Task 3: Implement Profile Completeness Scoring

**Goal:** Add completeness calculation with weighted scoring system.

**Files:**
- Modify: `crates/spectral-vault/src/profile.rs` (add after struct definitions)
- Test: `crates/spectral-vault/src/profile.rs:214-402`

### Step 1: Write failing test for completeness tiers

Add to test section:

```rust
#[test]
fn test_completeness_tier_minimal() {
    let profile = UserProfile::new(ProfileId::generate());
    let completeness = profile.completeness_score();

    assert_eq!(completeness.tier, CompletenessTier::Minimal);
    assert_eq!(completeness.score, 0);
    assert_eq!(completeness.percentage, 0);
}

#[test]
fn test_completeness_tier_basic() {
    let key = test_key();
    let mut profile = UserProfile::new(ProfileId::generate());

    profile.first_name = Some(encrypt_string("John", &key).expect("encrypt"));
    profile.last_name = Some(encrypt_string("Doe", &key).expect("encrypt"));
    profile.email = Some(encrypt_string("john@example.com", &key).expect("encrypt"));

    let completeness = profile.completeness_score();

    assert_eq!(completeness.tier, CompletenessTier::Basic);
    assert_eq!(completeness.score, 40); // 15+15+10
}

#[test]
fn test_completeness_tier_excellent() {
    let key = test_key();
    let mut profile = UserProfile::new(ProfileId::generate());

    // Core identity (40 points)
    profile.first_name = Some(encrypt_string("John", &key).expect("encrypt"));
    profile.last_name = Some(encrypt_string("Doe", &key).expect("encrypt"));
    profile.email = Some(encrypt_string("john@example.com", &key).expect("encrypt"));

    // Current location (30 points)
    profile.address = Some(encrypt_string("123 Main", &key).expect("encrypt"));
    profile.city = Some(encrypt_string("Chicago", &key).expect("encrypt"));
    profile.state = Some(encrypt_string("IL", &key).expect("encrypt"));
    profile.zip_code = Some(encrypt_string("60601", &key).expect("encrypt"));

    // Enhanced matching (30 points)
    profile.phone_numbers = vec![PhoneNumber {
        number: encrypt_string("555-1234", &key).expect("encrypt"),
        phone_type: PhoneType::Mobile,
    }];
    profile.previous_addresses_v2 = vec![PreviousAddress {
        address_line1: encrypt_string("456 Oak", &key).expect("encrypt"),
        address_line2: None,
        city: encrypt_string("Seattle", &key).expect("encrypt"),
        state: encrypt_string("WA", &key).expect("encrypt"),
        zip_code: encrypt_string("98101", &key).expect("encrypt"),
        lived_from: Some("2020-01-01".to_string()),
        lived_to: Some("2022-12-31".to_string()),
    }];
    profile.date_of_birth = Some(encrypt_string("1990-01-01", &key).expect("encrypt"));
    profile.aliases = vec![encrypt_string("Johnny", &key).expect("encrypt")];
    profile.relatives = vec![Relative {
        name: encrypt_string("Jane", &key).expect("encrypt"),
        relationship: RelationshipType::Spouse,
    }];

    let completeness = profile.completeness_score();

    assert_eq!(completeness.tier, CompletenessTier::Excellent);
    assert_eq!(completeness.score, 100);
    assert_eq!(completeness.percentage, 100);
}
```

### Step 2: Run tests to verify they fail

Run: `cargo test test_completeness`

Expected: Compilation errors for missing types

### Step 3: Add CompletenessTier enum

Add after Relative struct:

```rust
/// Profile completeness tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CompletenessTier {
    /// 0-30 points: Limited information
    Minimal,
    /// 31-60 points: Basic information
    Basic,
    /// 61-85 points: Good information
    Good,
    /// 86-100 points: Excellent information
    Excellent,
}
```

### Step 4: Add ProfileCompleteness struct

Add after CompletenessTier:

```rust
/// Profile completeness metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileCompleteness {
    /// Raw score (0-100)
    pub score: u32,
    /// Maximum possible score
    pub max_score: u32,
    /// Percentage (0-100)
    pub percentage: u32,
    /// Completeness tier
    pub tier: CompletenessTier,
    /// User-friendly message
    pub message: String,
}
```

### Step 5: Implement completeness_score method

Add to `impl UserProfile` block:

```rust
/// Calculate profile completeness score.
///
/// Scoring breakdown:
/// - Core identity (40 points): first_name (15), last_name (15), email (10)
/// - Current location (30 points): address (10), city (10), state+zip (10)
/// - Enhanced matching (30 points): phones (10), prev_addresses (10), dob (5), aliases (3), relatives (2)
pub fn completeness_score(&self) -> ProfileCompleteness {
    let mut score = 0u32;

    // Core identity (40 points)
    if self.first_name.is_some() {
        score += 15;
    }
    if self.last_name.is_some() {
        score += 15;
    }
    if self.email.is_some() {
        score += 10;
    }

    // Current location (30 points)
    if self.address.is_some() {
        score += 10;
    }
    if self.city.is_some() {
        score += 10;
    }
    if self.state.is_some() && self.zip_code.is_some() {
        score += 10;
    }

    // Enhanced matching (30 points)
    if !self.phone_numbers.is_empty() {
        score += 10;
    }
    if !self.previous_addresses_v2.is_empty() {
        score += 10;
    }
    if self.date_of_birth.is_some() {
        score += 5;
    }
    if !self.aliases.is_empty() {
        score += 3;
    }
    if !self.relatives.is_empty() {
        score += 2;
    }

    let tier = Self::score_to_tier(score);

    ProfileCompleteness {
        score,
        max_score: 100,
        percentage: score,
        tier,
        message: Self::tier_message(tier),
    }
}

fn score_to_tier(score: u32) -> CompletenessTier {
    match score {
        0..=30 => CompletenessTier::Minimal,
        31..=60 => CompletenessTier::Basic,
        61..=85 => CompletenessTier::Good,
        _ => CompletenessTier::Excellent,
    }
}

fn tier_message(tier: CompletenessTier) -> String {
    match tier {
        CompletenessTier::Minimal => {
            "Limited removal coverage - consider adding more information".to_string()
        }
        CompletenessTier::Basic => {
            "Basic removal coverage - adding contact info and addresses will improve results"
                .to_string()
        }
        CompletenessTier::Good => {
            "Good removal coverage - you've provided solid information for effective removal"
                .to_string()
        }
        CompletenessTier::Excellent => {
            "Excellent removal coverage - comprehensive information enables maximum removal effectiveness"
                .to_string()
        }
    }
}
```

### Step 6: Run tests to verify they pass

Run: `cargo test test_completeness`

Expected: All completeness tests pass

### Step 7: Run all vault tests

Run: `cargo test --package spectral-vault`

Expected: All tests pass

### Step 8: Commit completeness scoring

```bash
git add crates/spectral-vault/src/profile.rs
git commit -m "feat(vault): implement profile completeness scoring system"
```

---

## Task 4: Add get_profile_completeness Tauri Command

**Goal:** Expose completeness scoring to frontend via Tauri command.

**Files:**
- Modify: `src-tauri/src/commands/vault.rs`
- Test: Integration test in later task

### Step 1: Add get_profile_completeness command

Add to `src-tauri/src/commands/vault.rs` after existing vault commands:

```rust
/// Get profile completeness score.
#[tauri::command]
pub async fn get_profile_completeness(
    vault: State<'_, Arc<RwLock<Option<Vault>>>>,
) -> Result<spectral_vault::ProfileCompleteness, String> {
    let vault_lock = vault.read().await;
    let vault_ref = vault_lock
        .as_ref()
        .ok_or("Vault not initialized")?;

    let profile = vault_ref.get_profile().map_err(|e| e.to_string())?;
    Ok(profile.completeness_score())
}
```

### Step 2: Register command in main.rs

Add `get_profile_completeness` to the invoke handler in `src-tauri/src/lib.rs`:

Find the `.invoke_handler(tauri::generate_handler![` section and add:

```rust
commands::vault::get_profile_completeness,
```

### Step 3: Build to verify compilation

Run: `cargo build --package spectral-app`

Expected: Successful compilation

### Step 4: Commit Tauri command

```bash
git add src-tauri/src/commands/vault.rs src-tauri/src/lib.rs
git commit -m "feat(tauri): add get_profile_completeness command"
```

---

## Task 5: Update Frontend TypeScript Types

**Goal:** Add Phase 2 types to match Rust backend.

**Files:**
- Modify: `src/lib/api/profile.ts`

### Step 1: Add Phase 2 interfaces

Update `src/lib/api/profile.ts` to add new interfaces:

```typescript
export interface PhoneNumber {
  number: string;
  phone_type: 'Mobile' | 'Home' | 'Work';
}

export interface PreviousAddress {
  address_line1: string;
  address_line2?: string;
  city: string;
  state: string;
  zip_code: string;
  lived_from?: string; // YYYY-MM-DD
  lived_to?: string; // YYYY-MM-DD
}

export interface Relative {
  name: string;
  relationship: 'Spouse' | 'Partner' | 'Parent' | 'Child' | 'Sibling' | 'Other';
}

export interface ProfileCompleteness {
  score: number;
  max_score: number;
  percentage: number;
  tier: 'Minimal' | 'Basic' | 'Good' | 'Excellent';
  message: string;
}
```

### Step 2: Update ProfileInput interface

Update the existing ProfileInput interface to add Phase 2 fields:

```typescript
export interface ProfileInput {
  // Phase 1 fields (all optional)
  first_name?: string;
  middle_name?: string;
  last_name?: string;
  email?: string;
  date_of_birth?: string;
  address_line1?: string;
  address_line2?: string;
  city?: string;
  state?: string;
  zip_code?: string;

  // Phase 2 fields
  phone_numbers?: PhoneNumber[];
  previous_addresses?: PreviousAddress[];
  aliases?: string[];
  relatives?: Relative[];
}
```

### Step 3: Add getProfileCompleteness function

Add to `src/lib/api/profile.ts`:

```typescript
/**
 * Get profile completeness score
 */
export async function getProfileCompleteness(): Promise<ProfileCompleteness> {
  return invoke<ProfileCompleteness>('get_profile_completeness');
}
```

### Step 4: Verify TypeScript compilation

Run: `npm run check`

Expected: No type errors

### Step 5: Commit frontend types

```bash
git add src/lib/api/profile.ts
git commit -m "feat(frontend): add Phase 2 TypeScript types and completeness API"
```

---

## Task 6: Create CompletenessIndicator Component

**Goal:** Build real-time completeness indicator with tier-based styling.

**Files:**
- Create: `src/lib/components/profile/shared/CompletenessIndicator.svelte`

### Step 1: Create CompletenessIndicator component

Create `src/lib/components/profile/shared/CompletenessIndicator.svelte`:

```svelte
<script lang="ts">
  import type { ProfileCompleteness } from '$lib/api/profile';

  interface Props {
    completeness: ProfileCompleteness;
  }

  let { completeness }: Props = $props();

  const tierConfig = {
    Minimal: {
      color: 'bg-red-100 text-red-800 border-red-200',
      barColor: 'bg-red-500',
      icon: '⚠️'
    },
    Basic: {
      color: 'bg-yellow-100 text-yellow-800 border-yellow-200',
      barColor: 'bg-yellow-500',
      icon: '📝'
    },
    Good: {
      color: 'bg-blue-100 text-blue-800 border-blue-200',
      barColor: 'bg-blue-500',
      icon: '👍'
    },
    Excellent: {
      color: 'bg-green-100 text-green-800 border-green-200',
      barColor: 'bg-green-500',
      icon: '✨'
    },
  };

  const config = $derived(tierConfig[completeness.tier]);
</script>

<div class="border rounded-lg p-4 {config.color}">
  <div class="flex items-start gap-3">
    <span class="text-2xl" aria-hidden="true">{config.icon}</span>
    <div class="flex-1">
      <div class="flex items-center justify-between mb-2">
        <h3 class="font-semibold">Profile Completeness</h3>
        <span class="text-sm font-medium">{completeness.percentage}%</span>
      </div>
      <div class="w-full bg-white/50 rounded-full h-2 mb-2" role="progressbar" aria-valuenow={completeness.percentage} aria-valuemin={0} aria-valuemax={100}>
        <div
          class="{config.barColor} h-2 rounded-full transition-all duration-300"
          style:width="{completeness.percentage}%"
        />
      </div>
      <p class="text-sm">{completeness.message}</p>
    </div>
  </div>
</div>
```

### Step 2: Build to verify no errors

Run: `npm run build`

Expected: Successful build

### Step 3: Commit completeness indicator

```bash
git add src/lib/components/profile/shared/CompletenessIndicator.svelte
git commit -m "feat(ui): add CompletenessIndicator component with tier-based styling"
```

---

## Task 7: Create PreviousAddressModal Component

**Goal:** Build modal for complex previous address input.

**Files:**
- Create: `src/lib/components/profile/shared/PreviousAddressModal.svelte`

### Step 1: Create PreviousAddressModal component

Create `src/lib/components/profile/shared/PreviousAddressModal.svelte`:

```svelte
<script lang="ts">
  import type { PreviousAddress } from '$lib/api/profile';

  interface Props {
    address?: PreviousAddress;
    onSave: (address: PreviousAddress) => void;
    onCancel: () => void;
  }

  let { address, onSave, onCancel }: Props = $props();

  let formData = $state<PreviousAddress>(
    address || {
      address_line1: '',
      address_line2: '',
      city: '',
      state: '',
      zip_code: '',
      lived_from: '',
      lived_to: '',
    }
  );

  function handleSave() {
    // Validate required fields
    if (!formData.address_line1 || !formData.city || !formData.state || !formData.zip_code) {
      return;
    }

    // Clean up empty optional fields
    const cleaned: PreviousAddress = {
      address_line1: formData.address_line1,
      city: formData.city,
      state: formData.state,
      zip_code: formData.zip_code,
    };

    if (formData.address_line2) cleaned.address_line2 = formData.address_line2;
    if (formData.lived_from) cleaned.lived_from = formData.lived_from;
    if (formData.lived_to) cleaned.lived_to = formData.lived_to;

    onSave(cleaned);
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onCancel();
    }
  }
</script>

<div
  class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
  onclick={handleBackdropClick}
  role="dialog"
  aria-modal="true"
  aria-labelledby="modal-title"
>
  <div class="bg-white rounded-lg p-6 max-w-md w-full mx-4">
    <h2 id="modal-title" class="text-xl font-semibold mb-4">
      {address ? 'Edit' : 'Add'} Previous Address
    </h2>

    <div class="space-y-4">
      <div>
        <label class="block text-sm font-medium mb-1" for="addr1">
          Street Address <span class="text-red-500">*</span>
        </label>
        <input
          id="addr1"
          type="text"
          bind:value={formData.address_line1}
          class="w-full px-3 py-2 border rounded-md"
          placeholder="123 Main Street"
          required
        />
      </div>

      <div>
        <label class="block text-sm font-medium mb-1" for="addr2">
          Apt/Suite (optional)
        </label>
        <input
          id="addr2"
          type="text"
          bind:value={formData.address_line2}
          class="w-full px-3 py-2 border rounded-md"
          placeholder="Apt 4B"
        />
      </div>

      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="block text-sm font-medium mb-1" for="city">
            City <span class="text-red-500">*</span>
          </label>
          <input
            id="city"
            type="text"
            bind:value={formData.city}
            class="w-full px-3 py-2 border rounded-md"
            placeholder="Chicago"
            required
          />
        </div>

        <div>
          <label class="block text-sm font-medium mb-1" for="state">
            State <span class="text-red-500">*</span>
          </label>
          <input
            id="state"
            type="text"
            bind:value={formData.state}
            class="w-full px-3 py-2 border rounded-md"
            placeholder="IL"
            maxlength="2"
            required
          />
        </div>
      </div>

      <div>
        <label class="block text-sm font-medium mb-1" for="zip">
          ZIP Code <span class="text-red-500">*</span>
        </label>
        <input
          id="zip"
          type="text"
          bind:value={formData.zip_code}
          class="w-full px-3 py-2 border rounded-md"
          placeholder="60601"
          maxlength="10"
          required
        />
      </div>

      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="block text-sm font-medium mb-1" for="from">
            Lived From
          </label>
          <input
            id="from"
            type="date"
            bind:value={formData.lived_from}
            class="w-full px-3 py-2 border rounded-md"
          />
        </div>

        <div>
          <label class="block text-sm font-medium mb-1" for="to">
            Lived To
          </label>
          <input
            id="to"
            type="date"
            bind:value={formData.lived_to}
            class="w-full px-3 py-2 border rounded-md"
          />
        </div>
      </div>
    </div>

    <div class="flex justify-end gap-3 mt-6">
      <button
        onclick={onCancel}
        class="px-4 py-2 border rounded-md hover:bg-gray-50"
      >
        Cancel
      </button>
      <button
        onclick={handleSave}
        class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700"
        disabled={!formData.address_line1 || !formData.city || !formData.state || !formData.zip_code}
      >
        Save
      </button>
    </div>
  </div>
</div>
```

### Step 2: Build to verify no errors

Run: `npm run build`

Expected: Successful build

### Step 3: Commit previous address modal

```bash
git add src/lib/components/profile/shared/PreviousAddressModal.svelte
git commit -m "feat(ui): add PreviousAddressModal for complex address input"
```

---

## Task 8: Create RelativeModal Component

**Goal:** Build modal for relative input with relationship selector.

**Files:**
- Create: `src/lib/components/profile/shared/RelativeModal.svelte`

### Step 1: Create RelativeModal component

Create `src/lib/components/profile/shared/RelativeModal.svelte`:

```svelte
<script lang="ts">
  import type { Relative } from '$lib/api/profile';

  interface Props {
    relative?: Relative;
    onSave: (relative: Relative) => void;
    onCancel: () => void;
  }

  let { relative, onSave, onCancel }: Props = $props();

  let formData = $state<Relative>(
    relative || {
      name: '',
      relationship: 'Other',
    }
  );

  function handleSave() {
    if (!formData.name.trim()) {
      return;
    }

    onSave({
      name: formData.name.trim(),
      relationship: formData.relationship,
    });
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onCancel();
    }
  }
</script>

<div
  class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
  onclick={handleBackdropClick}
  role="dialog"
  aria-modal="true"
  aria-labelledby="modal-title"
>
  <div class="bg-white rounded-lg p-6 max-w-md w-full mx-4">
    <h2 id="modal-title" class="text-xl font-semibold mb-4">
      {relative ? 'Edit' : 'Add'} Relative
    </h2>

    <div class="space-y-4">
      <div>
        <label class="block text-sm font-medium mb-1" for="name">
          Name <span class="text-red-500">*</span>
        </label>
        <input
          id="name"
          type="text"
          bind:value={formData.name}
          class="w-full px-3 py-2 border rounded-md"
          placeholder="Jane Doe"
          required
        />
      </div>

      <div>
        <label class="block text-sm font-medium mb-1" for="relationship">
          Relationship <span class="text-red-500">*</span>
        </label>
        <select
          id="relationship"
          bind:value={formData.relationship}
          class="w-full px-3 py-2 border rounded-md"
          required
        >
          <option value="Spouse">Spouse</option>
          <option value="Partner">Partner</option>
          <option value="Parent">Parent</option>
          <option value="Child">Child</option>
          <option value="Sibling">Sibling</option>
          <option value="Other">Other</option>
        </select>
      </div>
    </div>

    <div class="flex justify-end gap-3 mt-6">
      <button
        onclick={onCancel}
        class="px-4 py-2 border rounded-md hover:bg-gray-50"
      >
        Cancel
      </button>
      <button
        onclick={handleSave}
        class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700"
        disabled={!formData.name.trim()}
      >
        Save
      </button>
    </div>
  </div>
</div>
```

### Step 2: Build to verify no errors

Run: `npm run build`

Expected: Successful build

### Step 3: Commit relative modal

```bash
git add src/lib/components/profile/shared/RelativeModal.svelte
git commit -m "feat(ui): add RelativeModal for family member input"
```

---

## Task 9: Update ContactInfoStep with Phone Numbers

**Goal:** Expand ContactInfoStep to include dynamic phone number rows.

**Files:**
- Modify: `src/lib/components/profile/steps/ContactInfoStep.svelte`

### Step 1: Read current ContactInfoStep

Run: `cat src/lib/components/profile/steps/ContactInfoStep.svelte`

### Step 2: Update ContactInfoStep with phone numbers

Replace contents with:

```svelte
<script lang="ts">
  import type { ProfileInput, PhoneNumber } from '$lib/api/profile';

  interface Props {
    profile: ProfileInput;
    onUpdate: (updates: Partial<ProfileInput>) => void;
  }

  let { profile, onUpdate }: Props = $props();

  let phoneNumbers = $state<PhoneNumber[]>(profile.phone_numbers || []);

  function addPhoneNumber() {
    phoneNumbers = [...phoneNumbers, { number: '', phone_type: 'Mobile' }];
  }

  function removePhoneNumber(index: number) {
    phoneNumbers = phoneNumbers.filter((_, i) => i !== index);
    onUpdate({ phone_numbers: phoneNumbers });
  }

  function updatePhoneNumber(index: number, field: keyof PhoneNumber, value: any) {
    phoneNumbers = phoneNumbers.map((phone, i) =>
      i === index ? { ...phone, [field]: value } : phone
    );
    onUpdate({ phone_numbers: phoneNumbers });
  }

  export function validate(): boolean {
    // All fields optional - always valid
    return true;
  }
</script>

<div class="space-y-6">
  <div>
    <label class="block text-sm font-medium mb-2" for="email">
      Email Address
    </label>
    <input
      id="email"
      type="email"
      value={profile.email || ''}
      oninput={(e) => onUpdate({ email: e.currentTarget.value })}
      class="w-full px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500"
      placeholder="your.email@example.com"
    />
    <p class="text-xs text-gray-600 mt-1">
      Used to match records on data broker sites
    </p>
  </div>

  <div>
    <label class="block text-sm font-medium mb-2">
      Phone Numbers
    </label>
    <p class="text-sm text-gray-600 mb-3">
      Adding phone numbers helps identify records across more data brokers
    </p>

    {#if phoneNumbers.length > 0}
      <div class="space-y-2 mb-3">
        {#each phoneNumbers as phone, i (i)}
          <div class="flex gap-2">
            <input
              type="tel"
              value={phone.number}
              oninput={(e) => updatePhoneNumber(i, 'number', e.currentTarget.value)}
              class="flex-1 px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500"
              placeholder="(555) 123-4567"
            />
            <select
              value={phone.phone_type}
              onchange={(e) => updatePhoneNumber(i, 'phone_type', e.currentTarget.value)}
              class="px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500"
            >
              <option value="Mobile">Mobile</option>
              <option value="Home">Home</option>
              <option value="Work">Work</option>
            </select>
            <button
              onclick={() => removePhoneNumber(i)}
              class="px-3 py-2 text-red-600 hover:bg-red-50 rounded-md"
              aria-label="Remove phone number"
            >
              Remove
            </button>
          </div>
        {/each}
      </div>
    {/if}

    <button
      onclick={addPhoneNumber}
      class="text-blue-600 hover:text-blue-700 text-sm font-medium"
    >
      + Add Phone Number
    </button>
  </div>
</div>
```

### Step 3: Build to verify no errors

Run: `npm run build`

Expected: Successful build

### Step 4: Commit ContactInfoStep update

```bash
git add src/lib/components/profile/steps/ContactInfoStep.svelte
git commit -m "feat(ui): add phone numbers to ContactInfoStep with dynamic rows"
```

---

## Task 10: Update AddressInfoStep with Previous Addresses

**Goal:** Expand AddressInfoStep to include previous addresses with modal.

**Files:**
- Modify: `src/lib/components/profile/steps/AddressInfoStep.svelte`

### Step 1: Read current AddressInfoStep

Run: `cat src/lib/components/profile/steps/AddressInfoStep.svelte`

### Step 2: Update AddressInfoStep with previous addresses

Replace contents (keeping existing current address fields, adding previous addresses section):

```svelte
<script lang="ts">
  import type { ProfileInput, PreviousAddress } from '$lib/api/profile';
  import PreviousAddressModal from '../shared/PreviousAddressModal.svelte';

  interface Props {
    profile: ProfileInput;
    onUpdate: (updates: Partial<ProfileInput>) => void;
  }

  let { profile, onUpdate }: Props = $props();

  let previousAddresses = $state<PreviousAddress[]>(profile.previous_addresses || []);
  let showAddressModal = $state(false);
  let editingAddressIndex = $state<number | null>(null);

  function openAddressModal(index?: number) {
    editingAddressIndex = index ?? null;
    showAddressModal = true;
  }

  function saveAddress(address: PreviousAddress) {
    if (editingAddressIndex !== null) {
      previousAddresses = previousAddresses.map((a, i) =>
        i === editingAddressIndex ? address : a
      );
    } else {
      previousAddresses = [...previousAddresses, address];
    }
    onUpdate({ previous_addresses: previousAddresses });
    showAddressModal = false;
  }

  function removeAddress(index: number) {
    previousAddresses = previousAddresses.filter((_, i) => i !== index);
    onUpdate({ previous_addresses: previousAddresses });
  }

  export function validate(): boolean {
    // All fields optional - always valid
    return true;
  }
</script>

<div class="space-y-6">
  <!-- Current Address Section -->
  <div>
    <h3 class="text-lg font-semibold mb-4">Current Address</h3>

    <div class="space-y-4">
      <div>
        <label class="block text-sm font-medium mb-2" for="address1">
          Street Address
        </label>
        <input
          id="address1"
          type="text"
          value={profile.address_line1 || ''}
          oninput={(e) => onUpdate({ address_line1: e.currentTarget.value })}
          class="w-full px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500"
          placeholder="123 Main Street"
        />
      </div>

      <div>
        <label class="block text-sm font-medium mb-2" for="address2">
          Apt/Suite (optional)
        </label>
        <input
          id="address2"
          type="text"
          value={profile.address_line2 || ''}
          oninput={(e) => onUpdate({ address_line2: e.currentTarget.value })}
          class="w-full px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500"
          placeholder="Apt 4B"
        />
      </div>

      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="block text-sm font-medium mb-2" for="city">
            City
          </label>
          <input
            id="city"
            type="text"
            value={profile.city || ''}
            oninput={(e) => onUpdate({ city: e.currentTarget.value })}
            class="w-full px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500"
            placeholder="Chicago"
          />
        </div>

        <div>
          <label class="block text-sm font-medium mb-2" for="state">
            State
          </label>
          <input
            id="state"
            type="text"
            value={profile.state || ''}
            oninput={(e) => onUpdate({ state: e.currentTarget.value })}
            class="w-full px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500"
            placeholder="IL"
            maxlength="2"
          />
        </div>
      </div>

      <div>
        <label class="block text-sm font-medium mb-2" for="zip">
          ZIP Code
        </label>
        <input
          id="zip"
          type="text"
          value={profile.zip_code || ''}
          oninput={(e) => onUpdate({ zip_code: e.currentTarget.value })}
          class="w-full px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500"
          placeholder="60601"
          maxlength="10"
        />
      </div>
    </div>
  </div>

  <!-- Previous Addresses Section -->
  <div>
    <h3 class="text-lg font-semibold mb-2">Previous Addresses</h3>
    <p class="text-sm text-gray-600 mb-3">
      Past addresses help remove outdated records from data brokers
    </p>

    {#if previousAddresses.length > 0}
      <div class="space-y-2 mb-3">
        {#each previousAddresses as addr, i (i)}
          <div class="flex items-start justify-between p-3 bg-gray-50 rounded-md">
            <div class="text-sm">
              <div class="font-medium">{addr.address_line1}</div>
              <div class="text-gray-600">
                {addr.city}, {addr.state} {addr.zip_code}
              </div>
              {#if addr.lived_from || addr.lived_to}
                <div class="text-gray-500 text-xs mt-1">
                  {addr.lived_from ?? 'Unknown'} – {addr.lived_to ?? 'Unknown'}
                </div>
              {/if}
            </div>
            <div class="flex gap-2">
              <button
                onclick={() => openAddressModal(i)}
                class="text-blue-600 hover:text-blue-700 text-sm"
              >
                Edit
              </button>
              <button
                onclick={() => removeAddress(i)}
                class="text-red-600 hover:text-red-700 text-sm"
              >
                Remove
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}

    <button
      onclick={() => openAddressModal()}
      class="text-blue-600 hover:text-blue-700 text-sm font-medium"
    >
      + Add Previous Address
    </button>
  </div>
</div>

{#if showAddressModal}
  <PreviousAddressModal
    address={editingAddressIndex !== null ? previousAddresses[editingAddressIndex] : undefined}
    onSave={saveAddress}
    onCancel={() => (showAddressModal = false)}
  />
{/if}
```

### Step 3: Build to verify no errors

Run: `npm run build`

Expected: Successful build

### Step 4: Commit AddressInfoStep update

```bash
git add src/lib/components/profile/steps/AddressInfoStep.svelte
git commit -m "feat(ui): add previous addresses to AddressInfoStep with modal"
```

---

## Task 11: Create AdditionalInfoStep Component

**Goal:** Create new wizard step for aliases and relatives.

**Files:**
- Create: `src/lib/components/profile/steps/AdditionalInfoStep.svelte`

### Step 1: Create AdditionalInfoStep component

Create `src/lib/components/profile/steps/AdditionalInfoStep.svelte`:

```svelte
<script lang="ts">
  import type { ProfileInput, Relative } from '$lib/api/profile';
  import RelativeModal from '../shared/RelativeModal.svelte';

  interface Props {
    profile: ProfileInput;
    onUpdate: (updates: Partial<ProfileInput>) => void;
  }

  let { profile, onUpdate }: Props = $props();

  let aliases = $state<string[]>(profile.aliases || []);
  let relatives = $state<Relative[]>(profile.relatives || []);
  let showRelativeModal = $state(false);
  let editingRelativeIndex = $state<number | null>(null);

  function addAlias() {
    aliases = [...aliases, ''];
  }

  function updateAlias(index: number, value: string) {
    aliases = aliases.map((a, i) => (i === index ? value : a));
    // Filter out empty aliases before updating
    const nonEmpty = aliases.filter((a) => a.trim());
    onUpdate({ aliases: nonEmpty });
  }

  function removeAlias(index: number) {
    aliases = aliases.filter((_, i) => i !== index);
    onUpdate({ aliases });
  }

  function openRelativeModal(index?: number) {
    editingRelativeIndex = index ?? null;
    showRelativeModal = true;
  }

  function saveRelative(relative: Relative) {
    if (editingRelativeIndex !== null) {
      relatives = relatives.map((r, i) => (i === editingRelativeIndex ? relative : r));
    } else {
      relatives = [...relatives, relative];
    }
    onUpdate({ relatives });
    showRelativeModal = false;
  }

  function removeRelative(index: number) {
    relatives = relatives.filter((_, i) => i !== index);
    onUpdate({ relatives });
  }

  export function validate(): boolean {
    // All fields optional - always valid
    return true;
  }
</script>

<div class="space-y-6">
  <!-- Aliases Section -->
  <div>
    <label class="block text-sm font-medium mb-2">
      Aliases & Former Names
    </label>
    <p class="text-sm text-gray-600 mb-3">
      Include nicknames, maiden names, or other names you've used
    </p>

    {#if aliases.length > 0}
      <div class="space-y-2 mb-3">
        {#each aliases as alias, i (i)}
          <div class="flex gap-2">
            <input
              type="text"
              value={alias}
              oninput={(e) => updateAlias(i, e.currentTarget.value)}
              class="flex-1 px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500"
              placeholder="Former or alternate name"
            />
            <button
              onclick={() => removeAlias(i)}
              class="px-3 py-2 text-red-600 hover:bg-red-50 rounded-md"
              aria-label="Remove alias"
            >
              Remove
            </button>
          </div>
        {/each}
      </div>
    {/if}

    <button
      onclick={addAlias}
      class="text-blue-600 hover:text-blue-700 text-sm font-medium"
    >
      + Add Alias
    </button>
  </div>

  <!-- Relatives Section -->
  <div>
    <label class="block text-sm font-medium mb-2">
      Family Members & Relatives
    </label>
    <p class="text-sm text-gray-600 mb-3">
      Data brokers often list relatives - adding them helps identify these records
    </p>

    {#if relatives.length > 0}
      <div class="space-y-2 mb-3">
        {#each relatives as relative, i (i)}
          <div class="flex items-center justify-between p-3 bg-gray-50 rounded-md">
            <div class="text-sm">
              <div class="font-medium">{relative.name}</div>
              <div class="text-gray-600">{relative.relationship}</div>
            </div>
            <div class="flex gap-2">
              <button
                onclick={() => openRelativeModal(i)}
                class="text-blue-600 hover:text-blue-700 text-sm"
              >
                Edit
              </button>
              <button
                onclick={() => removeRelative(i)}
                class="text-red-600 hover:text-red-700 text-sm"
              >
                Remove
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}

    <button
      onclick={() => openRelativeModal()}
      class="text-blue-600 hover:text-blue-700 text-sm font-medium"
    >
      + Add Relative
    </button>
  </div>
</div>

{#if showRelativeModal}
  <RelativeModal
    relative={editingRelativeIndex !== null ? relatives[editingRelativeIndex] : undefined}
    onSave={saveRelative}
    onCancel={() => (showRelativeModal = false)}
  />
{/if}
```

### Step 2: Build to verify no errors

Run: `npm run build`

Expected: Successful build

### Step 3: Commit AdditionalInfoStep

```bash
git add src/lib/components/profile/steps/AdditionalInfoStep.svelte
git commit -m "feat(ui): create AdditionalInfoStep for aliases and relatives"
```

---

## Task 12: Update ProfileWizard with 5-Step Flow

**Goal:** Expand wizard from 4 to 5 steps, add completeness indicator.

**Files:**
- Modify: `src/lib/components/profile/ProfileWizard.svelte`

### Step 1: Read current ProfileWizard

Run: `cat src/lib/components/profile/ProfileWizard.svelte`

### Step 2: Update wizard configuration

Find the `steps` array and update to 5 steps:

```typescript
const steps = [
  {
    number: 1,
    title: 'Basic Info',
    subtitle: 'Name and date of birth',
    component: BasicInfoStep
  },
  {
    number: 2,
    title: 'Contact',
    subtitle: 'Email and phone numbers',
    component: ContactInfoStep
  },
  {
    number: 3,
    title: 'Addresses',
    subtitle: 'Current and previous addresses',
    component: AddressInfoStep
  },
  {
    number: 4,
    title: 'Additional Info',
    subtitle: 'Aliases and relatives',
    component: AdditionalInfoStep
  },
  {
    number: 5,
    title: 'Review',
    subtitle: 'Verify and submit',
    component: ReviewStep
  },
];
```

### Step 3: Import AdditionalInfoStep

Add to imports:

```typescript
import AdditionalInfoStep from './steps/AdditionalInfoStep.svelte';
```

### Step 4: Add completeness indicator state

Add after profile state:

```typescript
import { getProfileCompleteness } from '$lib/api/profile';
import CompletenessIndicator from './shared/CompletenessIndicator.svelte';
import type { ProfileCompleteness } from '$lib/api/profile';

let completeness = $state<ProfileCompleteness | null>(null);

async function updateCompleteness() {
  try {
    completeness = await getProfileCompleteness();
  } catch (error) {
    console.error('Failed to get completeness:', error);
  }
}
```

### Step 5: Call updateCompleteness on profile update

Update the `handleStepUpdate` function to call `updateCompleteness()` after updating profile:

```typescript
function handleStepUpdate(updates: Partial<ProfileInput>) {
  profile = { ...profile, ...updates };
  updateCompleteness();
}
```

### Step 6: Add completeness indicator to UI

Add before the wizard steps:

```svelte
{#if completeness}
  <div class="mb-6">
    <CompletenessIndicator {completeness} />
  </div>
{/if}
```

### Step 7: Build to verify no errors

Run: `npm run build`

Expected: Successful build

### Step 8: Commit wizard update

```bash
git add src/lib/components/profile/ProfileWizard.svelte
git commit -m "feat(ui): expand wizard to 5 steps with completeness indicator"
```

---

## Task 13: Update ReviewStep with Phase 2 Fields

**Goal:** Display all Phase 2 fields in review step.

**Files:**
- Modify: `src/lib/components/profile/steps/ReviewStep.svelte`

### Step 1: Read current ReviewStep

Run: `cat src/lib/components/profile/steps/ReviewStep.svelte`

### Step 2: Add Phase 2 sections to ReviewStep

Add after existing sections:

```svelte
<!-- Phone Numbers -->
{#if profile.phone_numbers && profile.phone_numbers.length > 0}
  <div>
    <dt class="text-sm font-medium text-gray-600">Phone Numbers</dt>
    <dd class="mt-1">
      {#each profile.phone_numbers as phone}
        <div class="text-sm">
          {phone.number} ({phone.phone_type})
        </div>
      {/each}
    </dd>
  </div>
{/if}

<!-- Previous Addresses -->
{#if profile.previous_addresses && profile.previous_addresses.length > 0}
  <div>
    <dt class="text-sm font-medium text-gray-600">Previous Addresses</dt>
    <dd class="mt-1">
      {#each profile.previous_addresses as addr}
        <div class="text-sm mb-2">
          <div>{addr.address_line1}</div>
          {#if addr.address_line2}
            <div>{addr.address_line2}</div>
          {/if}
          <div>{addr.city}, {addr.state} {addr.zip_code}</div>
          {#if addr.lived_from || addr.lived_to}
            <div class="text-xs text-gray-500">
              {addr.lived_from ?? '?'} – {addr.lived_to ?? '?'}
            </div>
          {/if}
        </div>
      {/each}
    </dd>
  </div>
{/if}

<!-- Aliases -->
{#if profile.aliases && profile.aliases.length > 0}
  <div>
    <dt class="text-sm font-medium text-gray-600">Aliases</dt>
    <dd class="mt-1">
      {#each profile.aliases as alias}
        <div class="text-sm">{alias}</div>
      {/each}
    </dd>
  </div>
{/if}

<!-- Relatives -->
{#if profile.relatives && profile.relatives.length > 0}
  <div>
    <dt class="text-sm font-medium text-gray-600">Family Members</dt>
    <dd class="mt-1">
      {#each profile.relatives as relative}
        <div class="text-sm">
          {relative.name} ({relative.relationship})
        </div>
      {/each}
    </dd>
  </div>
{/if}
```

### Step 3: Build to verify no errors

Run: `npm run build`

Expected: Successful build

### Step 4: Commit ReviewStep update

```bash
git add src/lib/components/profile/steps/ReviewStep.svelte
git commit -m "feat(ui): add Phase 2 fields to ReviewStep display"
```

---

## Task 14: Integration Testing

**Goal:** Verify complete wizard flow with Phase 2 fields.

**Files:**
- Create: `docs/test-results/profile-setup-phase2-integration.md`

### Step 1: Start dev server

Run: `npm run tauri dev`

### Step 2: Manual test checklist

Test the following scenarios:

1. **Phone Numbers**
   - Add phone number
   - Edit phone number
   - Remove phone number
   - Multiple phone numbers with different types

2. **Previous Addresses**
   - Open modal
   - Fill all fields
   - Save address
   - Edit address
   - Remove address
   - Multiple previous addresses

3. **Aliases**
   - Add alias
   - Remove alias
   - Multiple aliases

4. **Relatives**
   - Open modal
   - Fill name and relationship
   - Save relative
   - Edit relative
   - Remove relative
   - Multiple relatives

5. **Completeness Indicator**
   - Verify starts at 0%
   - Verify updates as fields filled
   - Verify tier changes (Minimal → Basic → Good → Excellent)
   - Verify message changes

6. **Wizard Navigation**
   - Navigate through all 5 steps
   - Verify step 4 (Additional Info) accessible
   - Verify Review shows all Phase 2 fields

7. **Profile Save/Load**
   - Complete wizard with Phase 2 fields
   - Submit profile
   - Reload app
   - Verify Phase 2 fields persisted

### Step 3: Document test results

Create `docs/test-results/profile-setup-phase2-integration.md`:

```markdown
# Profile Setup UI - Phase 2 Integration Test Results

**Date**: 2026-02-11
**Tester**: [Name]
**Environment**: [OS, Browser/Tauri version]

## Test Summary

- Total Scenarios: 7
- Passed: [X]
- Failed: [X]
- Notes: [Any issues found]

## Detailed Results

### 1. Phone Numbers
- [ ] Add phone number
- [ ] Edit phone number type
- [ ] Remove phone number
- [ ] Multiple phone numbers
- Notes:

### 2. Previous Addresses
- [ ] Open modal
- [ ] Fill all fields
- [ ] Save address
- [ ] Edit address
- [ ] Remove address
- [ ] Multiple addresses
- Notes:

### 3. Aliases
- [ ] Add alias
- [ ] Remove alias
- [ ] Multiple aliases
- Notes:

### 4. Relatives
- [ ] Open modal
- [ ] Fill and save
- [ ] Edit relative
- [ ] Remove relative
- [ ] All relationship types
- Notes:

### 5. Completeness Indicator
- [ ] Starts at 0%
- [ ] Updates on field change
- [ ] Tier progression correct
- [ ] Messages accurate
- Notes:

### 6. Wizard Navigation
- [ ] All 5 steps accessible
- [ ] Step 4 renders correctly
- [ ] Review shows Phase 2 data
- Notes:

### 7. Persistence
- [ ] Profile saves
- [ ] Profile loads
- [ ] Phase 2 fields retained
- Notes:

## Issues Found

[List any bugs or issues]

## Recommendations

[Any improvements needed]
```

### Step 4: Run automated tests

Run: `cargo test --workspace`

Expected: All tests pass

### Step 5: Commit test results

```bash
git add docs/test-results/profile-setup-phase2-integration.md
git commit -m "test: document Phase 2 integration test results"
```

---

## Task 15: Final Cleanup and Merge

**Goal:** Clean up code, run linters, prepare for merge.

**Files:**
- Various

### Step 1: Run Rust formatter

Run: `cargo fmt --all`

### Step 2: Run Rust linter

Run: `cargo clippy --all-targets --all-features -- -D warnings`

Expected: No warnings

### Step 3: Run frontend linter

Run: `npm run lint`

Expected: No errors

### Step 4: Run frontend type check

Run: `npm run check`

Expected: No type errors

### Step 5: Run all tests

Run: `cargo test --workspace --all-features`

Expected: All tests pass

### Step 6: Build production

Run: `npm run build && cargo build --release`

Expected: Successful build

### Step 7: Final commit

```bash
git add -A
git commit -m "chore: final cleanup and formatting for Phase 2"
```

### Step 8: Push branch

Run: `git push -u origin task-1.6-phase2`

---

## Success Criteria Checklist

- [ ] Backend: PhoneNumber, PreviousAddress, Relative types defined with tests
- [ ] Backend: UserProfile updated with Phase 2 fields
- [ ] Backend: Profile completeness scoring implemented
- [ ] Backend: get_profile_completeness Tauri command working
- [ ] Frontend: TypeScript types match Rust backend
- [ ] Frontend: CompletenessIndicator component renders correctly
- [ ] Frontend: PreviousAddressModal functional
- [ ] Frontend: RelativeModal functional
- [ ] Frontend: ContactInfoStep includes phone numbers
- [ ] Frontend: AddressInfoStep includes previous addresses
- [ ] Frontend: AdditionalInfoStep created for aliases/relatives
- [ ] Frontend: ProfileWizard expanded to 5 steps
- [ ] Frontend: ReviewStep displays all Phase 2 fields
- [ ] Integration: Complete wizard flow tested
- [ ] Integration: Completeness indicator updates correctly
- [ ] Integration: Profile saves/loads with Phase 2 data
- [ ] All tests passing
- [ ] Code formatted and linted
- [ ] Ready for code review

---

## Notes

- All fields remain optional per design decision
- Completeness scoring guides users without enforcing requirements
- Hybrid input pattern: simple fields use dynamic rows, complex fields use modals
- Phase 2 fields use `#[serde(default)]` for backward compatibility
- Legacy `previous_addresses` field kept for migration compatibility
