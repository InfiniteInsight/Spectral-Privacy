## 20. User Onboarding & PII Profile Setup

Spectral must guide the user through a structured onboarding flow before any scanning or removal can begin. PII is never discovered indiscriminately — the app needs to know what to look for.

### 20.1 Onboarding Flow

```
┌─────────────────────────────────────────────────────────────────┐
│  Step 1 of 6: Create Vault                                      │
│                                                                  │
│  Your vault encrypts everything Spectral stores.                 │
│  Choose a strong master password.                                │
│                                                                  │
│  Password:     [••••••••••••••••]                                │
│  Confirm:      [••••••••••••••••]                                │
│                                                                  │
│  ℹ This password cannot be recovered. If you lose it,           │
│    your vault data is gone permanently.                          │
│                                                        [Next →] │
├─────────────────────────────────────────────────────────────────┤
│  Step 2 of 6: Your Location                                     │
│                                                                  │
│  Your location determines which privacy laws protect you         │
│  and which data brokers are most relevant.                       │
│                                                                  │
│  Country:      [United States        ▼]                          │
│  State:        [Maryland             ▼]                          │
│                                                                  │
│  Detected privacy laws:                                          │
│  ✓ Maryland Online Data Privacy Act (MODPA)                     │
│  ✓ CCPA/CPRA (CA brokers must honor nationwide)                 │
│  ✓ Federal: CAN-SPAM, FCRA                                     │
│                                                                  │
│  ℹ Some laws like GDPR only apply if you're in the EU.         │
│    Spectral tailors templates and timelines to your rights.      │
│                                                        [Next →] │
├─────────────────────────────────────────────────────────────────┤
│  Step 3 of 6: Who Are You?                                       │
│                                                                  │
│  Tell Spectral what PII to search for.                           │
│  Fields marked * are required for basic broker scanning.         │
│                                                                  │
│  ── Required for basic scanning ──────────────────────────────  │
│  First name *:      [_________________]                          │
│  Last name *:       [_________________]                          │
│  State/Region *:    [auto-filled from Step 2]                    │
│  City *:            [_________________]                          │
│                                                                  │
│  ── Improves match accuracy ──────────────────────────────────  │
│  Middle name:       [_________________]                          │
│  Previous names:    [+ Add]     (maiden name, former names)      │
│  Date of birth:     [__/__/____]                                 │
│  Age range:         [__] - [__]  (if you prefer not to give DOB) │
│                                                                  │
│  ── Contact information ──────────────────────────────────────  │
│  Email addresses:   [_________________] [+ Add more]             │
│  Phone numbers:     [_________________] [+ Add more]             │
│                                                                  │
│  ── Physical addresses (current + previous) ─────────────────  │
│  Current address:   [_________________] [+ Add more]             │
│  Previous addresses:[_________________] [+ Add more]             │
│                                                                  │
│  ── Advanced (only if needed) ────────────────────────────────  │
│  SSN last 4:        [____]  ℹ Only for brokers that require     │
│                                identity verification             │
│  Aliases/Nicknames: [_________________] [+ Add]                  │
│                                                                  │
│  Each field shows which brokers/features use it:                 │
│  📍 Name + City + State → used by 47 brokers for search         │
│  📧 Email → used for opt-out form submissions, verification     │
│  📱 Phone → used by 12 brokers as alternate search, match conf. │
│                                                        [Next →] │
├─────────────────────────────────────────────────────────────────┤
│  Step 4 of 6: Privacy Level                                      │
│                                                                  │
│  (Permission preset selection — see Section 8.3)                 │
│  Paranoid / Local Privacy / Balanced / Custom                    │
│                                                        [Next →] │
├─────────────────────────────────────────────────────────────────┤
│  Step 5 of 6: Email Setup                                        │
│                                                                  │
│  How should Spectral send removal requests and communicate       │
│  with data brokers?                                              │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ 📋 Draft Mode                             [RECOMMENDED] │    │
│  │    Spectral composes emails and opens them in your       │    │
│  │    email client for review & sending. You stay in        │    │
│  │    full control. No credentials needed.                  │    │
│  │                                                          │    │
│  │    Trade-off: You must manually send each email.         │    │
│  │    Verification emails must be handled by you.           │    │
│  └─────────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ 🤖 Automated Mode                                       │    │
│  │    Spectral sends emails directly via SMTP and can       │    │
│  │    monitor your inbox (IMAP) for verification emails     │    │
│  │    and broker replies.                                   │    │
│  │                                                          │    │
│  │    Requires: SMTP/IMAP credentials (stored in vault)     │    │
│  │    Benefit: Fully hands-off removal process.             │    │
│  │    Trade-off: Spectral needs email access.               │    │
│  └─────────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ 🔀 Hybrid Mode                                          │    │
│  │    Spectral sends emails via SMTP but does NOT           │    │
│  │    monitor your inbox. You handle verification           │    │
│  │    emails yourself.                                      │    │
│  │                                                          │    │
│  │    Good balance of automation + privacy.                 │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                  │
│  ℹ You can change this at any time in Settings.                 │
│                                                        [Next →] │
├─────────────────────────────────────────────────────────────────┤
│  Step 6 of 6: Ready to Scan                                      │
│                                                                  │
│  Spectral is configured and ready.                               │
│                                                                  │
│  Based on your location (Maryland, US), Spectral will            │
│  automatically scan 47 data brokers that are most likely to      │
│  have your information.                                          │
│                                                                  │
│  Estimated first scan time: ~15-30 minutes                       │
│                                                                  │
│  [Start First Scan]     [Go to Dashboard — scan later]           │
└─────────────────────────────────────────────────────────────────┘
```

### 20.2 Profile Data Model

```rust
// /crates/spectral-vault/src/profile.rs

/// The user's PII profile — everything Spectral knows about the user.
/// Stored encrypted in the vault. Never leaves the device unencrypted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // ── Jurisdiction (from onboarding Step 2) ───────────────
    pub jurisdiction: UserJurisdiction,

    // ── Core Identity (from onboarding Step 3) ──────────────
    pub first_name: String,
    pub last_name: String,
    pub middle_name: Option<String>,
    pub previous_names: Vec<PreviousName>,   // maiden name, former names
    pub aliases: Vec<String>,                // nicknames, alternate spellings
    pub date_of_birth: Option<NaiveDate>,
    pub age_range: Option<(u8, u8)>,         // if user prefers not to give DOB

    // ── Contact ─────────────────────────────────────────────
    pub email_addresses: Vec<EmailEntry>,
    pub phone_numbers: Vec<PhoneEntry>,

    // ── Physical Addresses ──────────────────────────────────
    pub current_address: Option<PhysicalAddress>,
    pub previous_addresses: Vec<PhysicalAddress>,

    // ── Advanced (optional) ─────────────────────────────────
    pub ssn_last_four: Option<EncryptedField>,  // double-encrypted
    pub additional_fields: HashMap<String, EncryptedField>,

    // ── Email Configuration (from onboarding Step 5) ────────
    pub email_mode: EmailMode,
    pub smtp_config: Option<EncryptedSmtpConfig>,
    pub imap_config: Option<EncryptedImapConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviousName {
    pub first_name: String,
    pub last_name: String,
    pub approximate_year_range: Option<(u16, u16)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailEntry {
    pub address: String,
    pub label: String,              // "personal", "work", "opt-out dedicated"
    pub is_primary: bool,
    pub use_for_optout: bool,       // safe to give to brokers?
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneEntry {
    pub number: String,
    pub phone_type: PhoneType,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmailMode {
    /// Compose drafts, user sends manually
    DraftOnly,
    /// Send via SMTP, no inbox monitoring
    SmtpOnly,
    /// Full automation: SMTP sending + IMAP monitoring
    FullAutomation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalAddress {
    pub street: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub country: String,
    pub approximate_years: Option<(u16, u16)>,  // when user lived there
}
```

### 20.3 PII Field Usage Transparency

Every PII field is annotated with exactly what uses it:

```rust
pub struct PiiFieldUsage {
    pub field: PiiField,
    /// Which brokers need this field to search for the user
    pub used_by_brokers_search: Vec<BrokerId>,
    /// Which brokers need this field for opt-out submission
    pub used_by_brokers_optout: Vec<BrokerId>,
    /// Which features use this field
    pub used_by_features: Vec<FeatureId>,
    /// Human-readable explanation shown in onboarding
    pub explanation: String,
    /// Is this required for basic functionality?
    pub required: bool,
}

// Example usage annotations shown to user:
// first_name + last_name + state + city → "Required. Used to search 47 data brokers."
// email → "Used for opt-out form submissions and to receive verification emails."
// phone → "Optional. Used by 12 brokers as an alternate search method."
// date_of_birth → "Optional. Improves match accuracy on brokers that list age."
// ssn_last_four → "Optional. Only used if a broker demands identity verification
//                   to process your removal. Never sent to LLMs."
```

---
