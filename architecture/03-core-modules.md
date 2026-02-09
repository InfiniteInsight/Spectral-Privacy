## 3. Core Modules — Detailed Design

### 3.1 Encrypted Vault (`spectral-vault`)

The vault is the heart of the security model. All PII and application state is stored in an encrypted SQLCipher database. The encryption key is derived from the user's master password using Argon2id.

```rust
// /crates/spectral-vault/src/lib.rs

pub struct Vault {
    db: SqlCipherConnection,
    cipher: VaultCipher,
}

pub struct VaultCipher {
    // Argon2id-derived key, held in memory only while unlocked
    key: Zeroizing<[u8; 32]>,
}

pub struct UserProfile {
    pub id: Uuid,
    pub full_name: EncryptedField<String>,
    pub email_addresses: EncryptedField<Vec<String>>,
    pub phone_numbers: EncryptedField<Vec<String>>,
    pub physical_addresses: EncryptedField<Vec<Address>>,
    pub date_of_birth: EncryptedField<Option<NaiveDate>>,
    pub aliases: EncryptedField<Vec<String>>,
    pub ssn_last_four: EncryptedField<Option<String>>,
    // Additional PII fields as needed
}

// Field-level encryption for sensitive data columns
pub struct EncryptedField<T> {
    ciphertext: Vec<u8>,
    nonce: [u8; 12],
    _phantom: PhantomData<T>,
}
```

**Security properties:**
- Database encrypted at rest with AES-256-GCM via SQLCipher
- Field-level encryption for PII columns using ChaCha20-Poly1305 (defense in depth)
- Key derived via Argon2id (memory-hard, resistant to GPU/ASIC attacks)
- Keys zeroized from memory on vault lock using the `zeroize` crate
- Auto-lock after configurable idle timeout
- No PII ever written to logs or temp files

### 3.2 LLM Router & Adapter (`spectral-llm`)

A unified abstraction layer that supports multiple LLM backends with a privacy-aware request pipeline.

```rust
// /crates/spectral-llm/src/lib.rs

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
    async fn stream(&self, request: CompletionRequest) -> Result<CompletionStream>;
    fn capabilities(&self) -> ProviderCapabilities;
    fn provider_id(&self) -> &str;
}

pub struct ProviderCapabilities {
    pub max_context_tokens: usize,
    pub supports_vision: bool,
    pub supports_tool_use: bool,
    pub supports_structured_output: bool,
    pub is_local: bool,           // critical for PII routing decisions
}

pub struct LlmRouter {
    providers: Vec<Box<dyn LlmProvider>>,
    pii_filter: PiiFilter,
    preference: RoutingPreference,
}

pub enum RoutingPreference {
    LocalOnly,                    // never send data to cloud APIs
    PreferLocal {                 // use local if capable, fallback to cloud
        cloud_allowed_tasks: Vec<TaskType>,
    },
    BestAvailable,                // use the most capable provider
}

// PII sanitization pipeline — runs BEFORE any LLM call
pub struct PiiFilter {
    patterns: Vec<PiiPattern>,    // regex + NER-based detection
    strategy: FilterStrategy,
}

pub enum FilterStrategy {
    Redact,                       // replace PII with [REDACTED_EMAIL], [REDACTED_NAME], etc.
    Tokenize {                    // replace with reversible tokens for re-injection
        token_map: HashMap<String, String>,
    },
    Block,                        // refuse to send if PII detected
}
```

**Supported providers (initial):**

| Provider | Type | Notes |
|----------|------|-------|
| Anthropic (Claude) | Cloud | Tool use for structured broker interaction |
| OpenAI (GPT-4o) | Cloud | Fallback, vision for CAPTCHA-adjacent tasks |
| Ollama | Local | Easy setup, supports many models |
| llama.cpp server | Local | Direct GGUF model loading, low overhead |
| LM Studio | Local | User-friendly, OpenAI-compatible API |
| vLLM | Local | High-throughput for users with serious hardware |

**PII-aware request pipeline:**

```
User Query → PII Filter (tokenize/redact) → Route to Provider
                                                    ↓
User Response ← PII Detokenize (if tokenized) ← Raw Response
```

The key insight: when asking an LLM "how do I opt out of Spokeo?", no PII needs to be sent at all. PII only touches the LLM when generating personalized opt-out emails, and even then, the local-preferred routing should handle it. For cloud providers, the tokenization strategy replaces real PII with placeholders, gets the response template, then re-injects locally.

### 3.3 LLM-Optional Architecture

Every feature in Spectral must work without any LLM. The LLM enhances the experience but is never a hard dependency. This is enforced architecturally — the core engine never calls LLM code directly; it goes through capability gates.

```rust
// /crates/spectral-core/src/capabilities.rs

/// Central capability registry — all features query this before attempting LLM calls.
/// This is the single source of truth for what's enabled.
pub struct CapabilityRegistry {
    llm_enabled: bool,
    llm_provider: Option<Arc<dyn LlmProvider>>,
    local_discovery_enabled: bool,
    browser_automation_enabled: bool,
    plugin_runtime_enabled: bool,
    // Per-feature LLM toggles (all default to false until explicitly enabled)
    features: HashMap<FeatureId, FeatureConfig>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum FeatureId {
    // Conversational
    ChatInterface,
    NaturalLanguageCommands,

    // Scanning & removal
    LlmGuidedBrowserSessions,
    SmartMatchConfidence,
    AutoBrokerDefinitionRepair,
    OptOutEmailGeneration,

    // Local discovery
    FileSystemPiiScan,
    EmailPiiScan,
    BrowserDataPiiScan,

    // Reporting
    NaturalLanguageSummaries,
    ThreatNarrativeGeneration,
}

#[derive(Debug, Clone)]
pub struct FeatureConfig {
    pub enabled: bool,
    pub requires_llm: bool,
    pub llm_routing: LlmRoutingOverride,
    pub permissions: FeaturePermissions,
}

#[derive(Debug, Clone)]
pub enum LlmRoutingOverride {
    /// Use the global routing preference
    Default,
    /// Force local-only for this feature regardless of global setting
    ForceLocal,
    /// Force a specific provider
    ForceProvider(String),
    /// Disable LLM for this feature even if globally enabled
    Disabled,
}

impl CapabilityRegistry {
    /// Every callsite that might use an LLM goes through this gate.
    /// Returns None if LLM is disabled or unavailable for this feature.
    pub fn llm_for_feature(&self, feature: FeatureId) -> Option<&dyn LlmProvider> {
        if !self.llm_enabled {
            return None;
        }

        let config = self.features.get(&feature)?;
        if !config.enabled || matches!(config.llm_routing, LlmRoutingOverride::Disabled) {
            return None;
        }

        self.llm_provider.as_deref()
    }

    /// Check if a feature is available (may or may not use LLM)
    pub fn is_feature_available(&self, feature: FeatureId) -> bool {
        match self.features.get(&feature) {
            Some(config) => {
                if config.requires_llm && !self.llm_enabled {
                    return false;
                }
                config.enabled
            }
            None => false,
        }
    }
}
```

### 3.4 Broker Engine (`spectral-broker`)

The broker engine manages the database of known data brokers and their opt-out procedures.

```rust
// /crates/spectral-broker/src/lib.rs

pub struct BrokerDefinition {
    pub id: String,
    pub name: String,
    pub url: String,
    pub category: BrokerCategory,
    pub search_method: SearchMethod,
    pub removal_method: RemovalMethod,
    pub typical_removal_time: Duration,
    pub difficulty: RemovalDifficulty,
    pub recheck_interval: Duration,
    pub last_verified: DateTime<Utc>,     // when this definition was last confirmed working
    pub notes: String,
}

pub enum BrokerCategory {
    PeopleSearch,         // Spokeo, BeenVerified, WhitePages, etc.
    DataAggregator,       // Acxiom, Oracle Data Cloud
    MarketingList,        // direct mail lists
    PublicRecords,        // court records aggregators
    SocialMedia,          // profile scraping sites
    BackgroundCheck,      // checkr-style services
    Other(String),
}

pub enum SearchMethod {
    /// Direct URL pattern — plug in name/location and fetch
    UrlTemplate {
        template: String,     // e.g., "https://www.spokeo.com/{first}-{last}/{state}/{city}"
        requires_fields: Vec<PiiField>,
    },
    /// Form-based search
    FormSearch {
        url: String,
        form_selector: String,
        field_mappings: HashMap<PiiField, String>,
    },
    /// API endpoint (some brokers expose search APIs)
    ApiSearch {
        endpoint: String,
        method: HttpMethod,
        payload_template: String,
    },
    /// LLM-guided — for brokers with complex/dynamic UIs
    LlmGuided {
        start_url: String,
        instructions: String,  // natural language instructions for the LLM
    },
}

pub enum RemovalMethod {
    /// Direct opt-out form
    WebForm {
        url: String,
        form_selector: String,
        field_mappings: HashMap<String, FormFieldValue>,
        confirmation_type: ConfirmationType,
    },
    /// Email-based removal
    Email {
        to: String,
        subject_template: String,
        body_template: String,
        requires_fields: Vec<PiiField>,
    },
    /// API-based removal
    Api {
        endpoint: String,
        method: HttpMethod,
        payload_template: String,
        auth: Option<ApiAuth>,
    },
    /// Requires multiple steps (e.g., submit form, then confirm via email link)
    MultiStep(Vec<RemovalStep>),
    /// LLM-guided for complex or frequently-changing procedures
    LlmGuided {
        start_url: String,
        instructions: String,
    },
    /// Manual — provide user with instructions
    Manual {
        instructions: String,
        estimated_time: Duration,
    },
}

pub enum ConfirmationType {
    None,                         // form submission is sufficient
    EmailVerification,            // click a link sent to user's email
    AccountRequired,              // must create account to remove
    PhotoId,                      // requires ID upload (flag to user)
    Mail,                         // physical mail verification
}
```

**Broker definition files** are stored as TOML in a community-maintained repository:

```toml
# /brokers/people-search/spokeo.toml
[broker]
id = "spokeo"
name = "Spokeo"
url = "https://www.spokeo.com"
category = "PeopleSearch"
difficulty = "Easy"
typical_removal_days = 3
recheck_interval_days = 30
last_verified = "2025-05-01"

[search]
method = "UrlTemplate"
template = "https://www.spokeo.com/{first}-{last}/{state}/{city}"
requires_fields = ["first_name", "last_name", "state", "city"]

[removal]
method = "WebForm"
url = "https://www.spokeo.com/optout"
confirmation = "EmailVerification"
notes = "Requires email verification. Link expires after 72 hours."

[removal.fields]
listing_url = "{found_listing_url}"
email = "{user_email}"
```

### 3.5 Browser Automation (`spectral-browser`)

Headless browser automation for scanning broker sites and submitting opt-out forms.

```rust
// /crates/spectral-browser/src/lib.rs

pub struct BrowserEngine {
    browser: Browser,             // chromiumoxide browser instance
    fingerprint: BrowserFingerprint,
    proxy: Option<ProxyConfig>,
    rate_limiter: RateLimiter,
}

pub struct BrowserFingerprint {
    // Anti-fingerprinting measures to avoid bot detection
    pub user_agent: String,
    pub viewport: Viewport,
    pub timezone: String,
    pub language: String,
    pub webgl_noise: bool,
}

pub struct ScanResult {
    pub broker_id: String,
    pub found: bool,
    pub listing_url: Option<String>,
    pub found_data_summary: Option<DataSummary>,
    pub screenshot: Option<Vec<u8>>,      // evidence screenshot
    pub confidence: f32,                   // 0.0-1.0 match confidence
    pub scanned_at: DateTime<Utc>,
}

impl BrowserEngine {
    pub async fn scan_broker(
        &self,
        broker: &BrokerDefinition,
        profile: &UserProfile,
    ) -> Result<ScanResult>;

    pub async fn submit_removal(
        &self,
        broker: &BrokerDefinition,
        listing: &BrokerResult,
        profile: &UserProfile,
    ) -> Result<RemovalAction>;

    /// For LLM-guided interactions — the LLM controls the browser
    pub async fn llm_guided_session(
        &self,
        llm: &dyn LlmProvider,
        start_url: &str,
        objective: &str,
        pii_context: &SanitizedContext,
    ) -> Result<SessionResult>;
}
```

**Rate limiting & stealth:**
- Per-broker rate limits (configurable, conservative defaults)
- Randomized delays between actions (human-like timing)
- Browser fingerprint rotation
- Optional proxy/VPN support (user-provided SOCKS5/HTTP proxy)
- Respect robots.txt as an ethical default (with user override for opt-out pages, which are meant for humans)

### 3.6 Plugin System (`spectral-plugins`)

WASM-based plugin system using Extism for safe, sandboxed extensibility.

```rust
// /crates/spectral-plugins/src/lib.rs

/// Plugins can extend Spectral in several ways
pub enum PluginType {
    /// Add a new broker with custom scan/removal logic
    BrokerPlugin {
        broker_id: String,
        scan: WasmFunction,
        remove: WasmFunction,
    },
    /// Add a new LLM provider
    LlmProviderPlugin {
        provider_id: String,
        complete: WasmFunction,
        capabilities: ProviderCapabilities,
    },
    /// Add notification integrations (email, Slack, webhook, etc.)
    NotificationPlugin {
        channel_id: String,
        send: WasmFunction,
    },
    /// Post-processing hooks (custom reporting, analytics, etc.)
    HookPlugin {
        events: Vec<EventType>,
        handler: WasmFunction,
    },
}

pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: semver::Version,
    pub author: String,
    pub description: String,
    pub plugin_type: PluginType,
    pub permissions: PluginPermissions,
    pub checksum: String,          // SHA-256 of the WASM binary
}

pub struct PluginPermissions {
    pub network_access: Vec<String>,   // allowed domains only
    pub pii_access: Vec<PiiField>,     // which PII fields it can read
    pub filesystem_access: bool,        // should almost always be false
}
```

**Plugin security model:**
- WASM sandbox prevents arbitrary code execution
- Explicit permission grants (network domains, PII fields)
- Plugins are signed and checksummed
- Community review process for the official plugin registry
- Users can install unreviewed plugins with a warning

### 3.7 Conversational Interface (`spectral-chat`)

The LLM-powered conversational interface sits on top of all other modules.

```rust
// /crates/spectral-chat/src/lib.rs

pub struct ConversationEngine {
    llm: LlmRouter,
    vault: Arc<Vault>,
    broker_engine: Arc<BrokerEngine>,
    scheduler: Arc<Scheduler>,
    tool_registry: ToolRegistry,
}

/// Tools available to the LLM during conversation
pub enum ChatTool {
    ScanBroker { broker_id: String },
    ScanAll,
    GetStatus { broker_id: Option<String> },
    SubmitRemoval { broker_result_id: String },
    UpdateProfile { fields: Vec<ProfileUpdate> },
    SearchBrokerDb { query: String },
    ExplainBroker { broker_id: String },
    GetTimeline,
    ExportReport { format: ReportFormat },
}
```

**Example conversation flows:**

```
User: "Hey, can you check if I'm on Spokeo?"
→ LLM parses intent → triggers ScanBroker("spokeo") → returns result

User: "Remove me from everywhere you found me"
→ LLM parses intent → iterates broker_results with status='found'
→ triggers SubmitRemoval for each → streams progress updates

User: "What's my removal status?"
→ LLM calls GetStatus(None) → formats a natural language summary
→ "You're currently listed on 3 of 47 brokers scanned. Spokeo and
    BeenVerified removals are pending (submitted 2 days ago).
    FastPeopleSearch removal was confirmed yesterday."

User: "How does Acxiom work and why is it hard to remove from?"
→ LLM calls ExplainBroker("acxiom") → provides context from broker DB
→ No PII sent to LLM for this query
```

---
