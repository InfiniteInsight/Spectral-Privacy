## 22. Proactive Broker Scanning Model

Spectral does not wait for the user to name specific brokers. It ships with a curated, region-aware broker database and automatically scans relevant brokers on the user's behalf.

### 22.1 Broker Classification

Every broker definition includes scan behavior and regional relevance:

```rust
// /crates/spectral-broker/src/definition.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerDefinition {
    pub id: BrokerId,
    pub name: String,
    pub domain: String,
    pub category: BrokerCategory,

    // ── Scan behavior ───────────────────────────────────────
    pub scan_priority: ScanPriority,

    // ── Regional relevance ──────────────────────────────────
    pub region_relevance: Vec<BrokerRegion>,
    pub registered_state: Option<String>,
    pub california_registered_data_broker: bool,
    pub home_country: CountryCode,

    // ── Legal context ───────────────────────────────────────
    pub applicable_regulations: Vec<PrivacyRegulation>,
    pub opt_out_method: OptOutMethod,
    pub typical_response_days: Option<u32>,
    pub requires_captcha: bool,
    pub requires_email_verification: bool,

    // ── Automation details ──────────────────────────────────
    pub search_fields_needed: Vec<PiiField>,   // what PII is needed to search
    pub optout_fields_needed: Vec<PiiField>,    // what PII the opt-out form needs
    pub automation_script: Option<PathBuf>,     // browser automation script
    pub opt_out_url: Option<String>,
    pub privacy_policy_url: Option<String>,
    pub contact_email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanPriority {
    /// Auto-scanned on first run for matching region users
    AutoScanTier1,
    /// Auto-scanned on second scheduled pass or user request
    AutoScanTier2,
    /// Scanned only on explicit user request or discovery
    OnRequest,
    /// Manual-only (no automation available, user guided through steps)
    ManualOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrokerRegion {
    US,             // United States people-search sites
    EU,             // EU data processors / GDPR-subject brokers
    UK,             // UK-specific brokers
    Canada,         // Canadian brokers (PIPEDA)
    Australia,      // Australian brokers
    Global,         // Operates globally (e.g., Acxiom, Oracle)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrokerCategory {
    /// Traditional data broker / people-search (Spokeo, BeenVerified, etc.)
    PeopleSearch,
    /// Marketing data broker (Acxiom, Epsilon, Oracle/AddThis)
    MarketingBroker,
    /// Background check provider (LexisNexis, Pipl)
    BackgroundCheck,
    /// Ad network / tracker with data broker characteristics
    AdNetworkBroker,
    /// Platform with significant PII (see Section 22.3)
    Platform,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptOutMethod {
    /// Web form that can be automated
    WebForm { url: String },
    /// Email-based opt-out request
    Email { address: String },
    /// Requires calling a phone number
    PhoneCall { number: String },
    /// Must mail a physical letter
    PostalMail { address: String },
    /// Has a dedicated privacy portal / API
    PrivacyPortal { url: String },
    /// Multiple methods available
    Multiple(Vec<OptOutMethod>),
}
```

### 22.2 Scan Flow

```
User completes onboarding
       │
       ▼
Filter broker database by:
  - User's region (BrokerRegion matches UserJurisdiction)
  - Scan priority (AutoScanTier1 first)
  - Available PII (user provided enough fields?)
       │
       ▼
┌──────────────────────────────────────────────────────────────┐
│  FIRST SCAN (~15-30 minutes)                                  │
│                                                               │
│  For each Tier 1 broker matching user's region:               │
│  1. Navigate to broker site via browser automation            │
│  2. Search for user using provided PII                        │
│  3. Screenshot the results page                               │
│  4. If profile found:                                         │
│     a. Record what PII the broker exposes                     │
│     b. Mark broker as "Profile Found — pending removal"       │
│     c. Queue for opt-out submission                           │
│  5. If no profile found:                                      │
│     a. Mark broker as "Not Found"                             │
│     b. Schedule periodic re-check (brokers re-add people)     │
│  6. If CAPTCHA encountered:                                   │
│     a. Pause automation                                       │
│     b. Notify user: "Spokeo needs verification — click to     │
│        complete" (see Section 24.2)                           │
│     c. Resume after user completes CAPTCHA                    │
│  7. If site unreachable / layout changed:                     │
│     a. Mark broker definition as "possibly broken"            │
│     b. Log for community reporting                            │
└──────────────────────────────────────────────────────────────┘
       │
       ▼ (scheduled, e.g., weekly)
┌──────────────────────────────────────────────────────────────┐
│  SUBSEQUENT SCANS                                             │
│                                                               │
│  - Re-scan Tier 1 brokers previously marked "Not Found"       │
│    (brokers constantly refresh databases)                     │
│  - Scan Tier 2 brokers for matching region                    │
│  - Verify previous removals still effective                   │
│  - Process any newly discovered brokers from telemetry        │
└──────────────────────────────────────────────────────────────┘
```

### 22.3 Platform Category (Non-Broker Major Services)

Major platforms that aren't traditional data brokers but hold significant user PII. Spectral can't auto-remove users from these (account deletion requires authentication), but can:

- Detect the user has an account (via email/browser discovery)
- Link directly to the platform's privacy/deletion settings page
- Track whether the user has taken action
- Generate GDPR/CCPA deletion request emails for the platform

```toml
# /broker-definitions/platforms/google.toml
[platform]
id = "google"
name = "Google"
domain = "google.com"
category = "Platform"
scan_priority = "OnRequest"
region_relevance = ["Global"]

[platform.privacy_settings]
account_deletion_url = "https://myaccount.google.com/delete-services-or-account"
privacy_checkup_url = "https://myaccount.google.com/privacycheckup"
data_download_url = "https://takeout.google.com/"
activity_controls_url = "https://myaccount.google.com/activitycontrols"

[platform.detection]
# How Spectral discovers the user has a Google account
email_domain_match = ["gmail.com", "googlemail.com"]
browser_cookie_domains = ["google.com", "youtube.com", "gmail.com"]
```

Platforms tracked: Google, Facebook/Meta, Amazon, Apple, Microsoft, LinkedIn, Twitter/X, Instagram, TikTok, Reddit, Pinterest, Snapchat, Discord, WhatsApp, and others as definitions are contributed.

### 22.4 Broker Coverage Tiers (Updated)

**Tier 1 — Auto-scan on first run (US users):**
Spokeo, BeenVerified, WhitePages, FastPeopleSearch, TruePeopleSearch,
Intelius, PeopleFinder, Radaris, USSearch, MyLife, Instant Checkmate,
ThatsThem, Nuwber, ClustrMaps, Cyberbackgroundchecks, Publicrecords,
PublicDataCheck, Addresses.com, AnyWho, 411.com

**Tier 2 — Auto-scan on second pass (US users):**
Acxiom, Oracle/AddThis, TowerData, Epsilon, LexisNexis, Pipl,
ZabaSearch, PeopleSmart, Truthfinder, Checkpeople, SearchPeopleFree,
PeopleSearchNow, AdvancedBackgroundChecks, IDTrue, Veriforia,
FamilyTreeNow, Locatefamily, Neighbor.report, OfficialUSA, VoterRecords

**Tier 3+ — On-request or discovery-based:**
100+ additional brokers, added continuously via community contributions.

**EU-specific brokers (auto-scan for EU users):**
192.com (UK), dastelefonbuch.de (DE), pagesjaunes.fr (FR),
paginebianche.it (IT), and region-specific equivalents.

---
