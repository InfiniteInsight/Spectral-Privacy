# Profile Setup UI - Phase 2 Design

**Date**: 2026-02-11
**Status**: Approved
**Phase**: 2 of 2

## Overview

Phase 2 expands the Profile Setup wizard with additional fields to improve data broker matching and removal effectiveness:
- **Phone numbers** (multiple, with type: Mobile/Home/Work)
- **Previous addresses** (multiple, with date ranges)
- **Aliases** (former names, nicknames)
- **Relatives** (family members for relationship-based matching)

**Key Philosophy**: ALL fields remain optional. The UI provides guidance on how completeness affects removal quality, educating users rather than forcing data entry.

## Architecture & Data Model

### Backend Schema Updates

Updated `UserProfile` struct in `crates/spectral-vault/src/profile.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    // Phase 1 fields - NOW ALL OPTIONAL
    pub first_name: Option<EncryptedField<String>>,
    pub middle_name: Option<EncryptedField<String>>,
    pub last_name: Option<EncryptedField<String>>,
    pub email: Option<EncryptedField<String>>,
    pub date_of_birth: Option<NaiveDate>,
    pub address_line1: Option<EncryptedField<String>>,
    pub address_line2: Option<EncryptedField<String>>,
    pub city: Option<EncryptedField<String>>,
    pub state: Option<EncryptedField<String>>,
    pub zip_code: Option<EncryptedField<String>>,

    // Phase 2 fields - NEW
    pub phone_numbers: Vec<PhoneNumber>,
    pub previous_addresses: Vec<PreviousAddress>,
    pub aliases: Vec<EncryptedField<String>>,
    pub relatives: Vec<Relative>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneNumber {
    pub number: EncryptedField<String>,
    pub phone_type: PhoneType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PhoneType {
    Mobile,
    Home,
    Work,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviousAddress {
    pub address_line1: EncryptedField<String>,
    pub address_line2: Option<EncryptedField<String>>,
    pub city: EncryptedField<String>,
    pub state: EncryptedField<String>,
    pub zip_code: EncryptedField<String>,
    pub lived_from: Option<NaiveDate>,
    pub lived_to: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relative {
    pub name: EncryptedField<String>,
    pub relationship: RelationshipType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RelationshipType {
    Spouse,
    Partner,
    Parent,
    Child,
    Sibling,
    Other,
}
```

### Profile Completeness Scoring

Weighted scoring system (0-100 points) with 4 tiers:

```rust
impl UserProfile {
    pub fn completeness_score(&self) -> ProfileCompleteness {
        let mut score = 0;

        // Core identity (40 points)
        if self.first_name.is_some() { score += 15; }
        if self.last_name.is_some() { score += 15; }
        if self.email.is_some() { score += 10; }

        // Current location (30 points)
        if self.address_line1.is_some() { score += 10; }
        if self.city.is_some() { score += 10; }
        if self.state.is_some() && self.zip_code.is_some() { score += 10; }

        // Enhanced matching (30 points)
        if !self.phone_numbers.is_empty() { score += 10; }
        if !self.previous_addresses.is_empty() { score += 10; }
        if self.date_of_birth.is_some() { score += 5; }
        if !self.aliases.is_empty() { score += 3; }
        if !self.relatives.is_empty() { score += 2; }

        ProfileCompleteness {
            score,
            max_score: 100,
            percentage: score,
            tier: Self::score_to_tier(score),
            message: Self::tier_message(Self::score_to_tier(score)),
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
            CompletenessTier::Minimal => "Limited removal coverage - consider adding more information".to_string(),
            CompletenessTier::Basic => "Basic removal coverage - adding contact info and addresses will improve results".to_string(),
            CompletenessTier::Good => "Good removal coverage - you've provided solid information for effective removal".to_string(),
            CompletenessTier::Excellent => "Excellent removal coverage - comprehensive information enables maximum removal effectiveness".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileCompleteness {
    pub score: u32,
    pub max_score: u32,
    pub percentage: u32,
    pub tier: CompletenessTier,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CompletenessTier {
    Minimal,
    Basic,
    Good,
    Excellent,
}
```

### Tauri Commands

New command to get profile completeness:

```rust
#[tauri::command]
pub async fn get_profile_completeness(vault: State<'_, Arc<RwLock<Option<Vault>>>>) -> Result<ProfileCompleteness, String> {
    let vault_lock = vault.read().await;
    let vault_ref = vault_lock.as_ref().ok_or("Vault not initialized")?;

    let profile = vault_ref.get_profile().map_err(|e| e.to_string())?;
    Ok(profile.completeness_score())
}
```

## Frontend Implementation

### TypeScript Types

Updated types in `src/lib/api/profile.ts`:

```typescript
export interface ProfileInput {
  // ALL OPTIONAL
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
  lived_from?: string;
  lived_to?: string;
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

### Wizard Flow

Updated 5-step wizard structure:

1. **Basic Info** - Name and date of birth
2. **Contact** - Email and phone numbers (EXPANDED)
3. **Addresses** - Current and previous addresses (EXPANDED)
4. **Additional Info** - Aliases and relatives (NEW STEP)
5. **Review** - Verify and submit (UPDATED)

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

### Completeness Indicator Component

New component: `src/lib/components/profile/shared/CompletenessIndicator.svelte`

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
    <span class="text-2xl">{config.icon}</span>
    <div class="flex-1">
      <div class="flex items-center justify-between mb-2">
        <h3 class="font-semibold">Profile Completeness</h3>
        <span class="text-sm font-medium">{completeness.percentage}%</span>
      </div>
      <div class="w-full bg-white/50 rounded-full h-2 mb-2">
        <div
          class="{config.barColor} h-2 rounded-full transition-all duration-300"
          style="width: {completeness.percentage}%"
        />
      </div>
      <p class="text-sm">{completeness.message}</p>
    </div>
  </div>
</div>
```

## Component Updates

### ContactInfoStep

Expands from email-only to include phone numbers with dynamic rows:

**Key Changes**:
- Add phone numbers array with `$state`
- Dynamic "Add Another" interface for phones
- Type selector (Mobile/Home/Work) for each phone
- Remove button for each phone entry

### AddressInfoStep

Expands to include previous addresses with modal dialog:

**Key Changes**:
- Keep existing current address form
- Add previous addresses section
- Modal dialog for complex address entry (5 fields + dates)
- List view of added previous addresses with edit/remove

### AdditionalInfoStep (NEW)

New step for aliases and relatives:

**Aliases Section**:
- Dynamic rows with simple text input
- "Add Another Alias" button
- Remove button for each alias

**Relatives Section**:
- List of added relatives
- Modal dialog for complex entry (name + relationship)
- Relationship selector (Spouse/Partner/Parent/Child/Sibling/Other)

### ReviewStep

Updated to display all Phase 2 fields:

**New Sections**:
- Phone numbers list with types
- Previous addresses with date ranges
- Aliases list
- Relatives list with relationships

## Input Patterns

### Dynamic Rows (Simple Fields)
Used for: Aliases, Phone Numbers

- Inline "Add Another" button
- Each row has remove button
- No modal required

### Modal Dialog (Complex Fields)
Used for: Previous Addresses, Relatives

- "Add [Item]" button opens modal
- Modal contains multiple fields
- Save/Cancel buttons
- Edit mode reuses same modal

## Migration Strategy

### Breaking Changes

Phase 1 fields changing from required to optional:
- `first_name`, `last_name`, `email` no longer required
- Address fields no longer required
- Need database migration for existing profiles

### Migration Steps

1. Add Phase 2 fields to schema (non-breaking - new Vec fields default to empty)
2. Update validation to make Phase 1 fields optional (breaking change)
3. Run migration to ensure existing profiles compatible
4. Update frontend validation

## Testing Strategy

### Backend Tests
- Completeness scoring calculation
- Profile with Phase 2 fields serialization
- Encryption/decryption of new field types
- Validation with all optional fields

### Frontend Tests
- Dynamic row addition/removal
- Modal open/close/save
- Completeness indicator updates
- Step navigation with new 5-step flow

### Integration Tests
- Full wizard flow with Phase 2 fields
- Profile save with all field types
- Profile load and display
- Completeness calculation accuracy

## Success Criteria

1. ✅ Users can add multiple phone numbers with types
2. ✅ Users can add previous addresses with date ranges
3. ✅ Users can add aliases via simple rows
4. ✅ Users can add relatives with relationship types
5. ✅ Completeness indicator updates in real-time
6. ✅ All fields remain optional
7. ✅ Existing Phase 1 profiles migrate successfully
8. ✅ All Phase 2 data encrypts properly
9. ✅ Review step displays all Phase 2 fields
10. ✅ Profile saves and loads with Phase 2 data

## Implementation Plan

Will be created in separate implementation plan document after design approval.
