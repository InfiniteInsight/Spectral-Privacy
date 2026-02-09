## 8. Granular Permission System (`spectral-permissions`)

Permissions are the backbone of trust in Spectral. Every component that accesses PII, local data, network, or system resources must request explicit permission.

### 8.1 Permission Architecture

```rust
// /crates/spectral-permissions/src/lib.rs

/// Every permission is a specific, auditable grant with a defined scope.
/// Permissions are stored in the vault and can be revoked at any time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: Uuid,
    pub grant_type: PermissionGrant,
    pub granted_to: PermissionSubject,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub granted_by: GrantSource,          // user_explicit, first_run_wizard, plugin_install
    pub revocable: bool,                  // always true except for core vault access
    pub last_used: Option<DateTime<Utc>>,
    pub use_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionSubject {
    CoreModule(String),          // "spectral-browser", "spectral-discovery", etc.
    LlmProvider(String),         // "anthropic", "ollama", etc.
    Plugin(PluginId),            // specific installed plugin
    Feature(FeatureId),          // specific feature toggle
}

/// Fine-grained permission types organized by category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionGrant {
    // ── PII Access ──────────────────────────────────────────
    /// Read specific PII fields from the vault
    PiiRead {
        fields: Vec<PiiFieldPermission>,
    },

    // ── Filesystem ──────────────────────────────────────────
    /// Scan files for PII (read-only)
    FileSystemRead {
        paths: Vec<ScopedPath>,
        file_types: Vec<String>,         // allowed extensions
        max_depth: Option<u32>,          // directory recursion limit
        exclude_patterns: Vec<String>,   // glob patterns to skip
    },
    /// Modify files (for redaction features)
    FileSystemWrite {
        paths: Vec<ScopedPath>,
        operations: Vec<FileOperation>,  // RedactInPlace, MoveToQuarantine, Delete
    },

    // ── Email ───────────────────────────────────────────────
    /// Connect to email via IMAP (read-only)
    EmailImapRead {
        server: String,
        mailboxes: Vec<String>,          // "INBOX", "Sent", "*" for all
        scope: EmailScanMode,
        max_age_days: Option<u32>,
    },
    /// Read local mailbox files
    EmailLocalRead {
        paths: Vec<PathBuf>,
    },

    // ── Browser Data ────────────────────────────────────────
    /// Read browser profile data
    BrowserRead {
        browsers: Vec<BrowserTarget>,
        scopes: Vec<BrowserScanScope>,
    },

    // ── Network ─────────────────────────────────────────────
    /// Make outbound HTTP requests to specific domains
    NetworkAccess {
        domains: Vec<DomainPermission>,
        purpose: String,                 // human-readable explanation
    },
    /// Send data to an LLM provider
    LlmApiAccess {
        provider: String,
        pii_filter_required: bool,       // must PII filter be active?
        allowed_tasks: Vec<TaskType>,
    },

    // ── Browser Automation ──────────────────────────────────
    /// Automate browser sessions on specific domains
    BrowserAutomation {
        domains: Vec<String>,
        actions: Vec<AutomationAction>,  // Navigate, FillForm, Click, Screenshot
    },

    // ── System ──────────────────────────────────────────────
    /// Send desktop notifications
    DesktopNotification,
    /// Access system clipboard (for copy-to-clipboard features)
    ClipboardWrite,
    /// Run on startup / background scheduling
    BackgroundExecution {
        schedule: Option<String>,        // cron expression
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiFieldPermission {
    pub field: PiiField,
    pub access_level: PiiAccessLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PiiField {
    FullName,
    FirstName,
    LastName,
    Email,
    Phone,
    PhysicalAddress,
    DateOfBirth,
    SsnFull,
    SsnLastFour,
    Aliases,
    // Composite
    AllIdentifiers,      // name + email + phone
    AllPii,              // everything
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PiiAccessLevel {
    /// Can use the data for matching/comparison but never see the raw value
    HashOnly,
    /// Can read the value but it's redacted in logs and LLM calls
    ReadRedacted,
    /// Full access to the raw value (needed for form filling, email generation)
    ReadFull,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopedPath {
    pub path: PathBuf,
    pub recursive: bool,
    pub rationale: String,    // shown to user: "Scan your Documents folder for tax forms"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainPermission {
    pub domain: String,       // e.g., "spokeo.com", "*.spokeo.com"
    pub ports: Vec<u16>,      // typically [443]
    pub methods: Vec<String>, // "GET", "POST"
}
```

### 8.2 Permission Request Flow

```
Component needs access
        │
        ▼
PermissionManager.check(subject, grant)
        │
        ├── Already granted & not expired? ──► Allow (log usage)
        │
        ├── Denied / Revoked? ──► Deny (return error)
        │
        └── Not yet requested? ──► Queue permission prompt
                                          │
                                          ▼
                                   ┌──────────────┐
                                   │  User Prompt  │
                                   │  (in the UI)  │
                                   └──────┬───────┘
                                          │
                              ┌───────────┼───────────┐
                              ▼           ▼           ▼
                           Allow      Allow Once    Deny
                           (saved)    (session)     (saved)
```

```rust
// /crates/spectral-permissions/src/manager.rs

pub struct PermissionManager {
    vault: Arc<Vault>,
    pending_prompts: broadcast::Sender<PermissionPrompt>,
    decisions: mpsc::Receiver<PermissionDecision>,
    audit: AuditLogger,
}

pub struct PermissionPrompt {
    pub id: Uuid,
    pub subject: PermissionSubject,
    pub requested_grant: PermissionGrant,
    pub rationale: String,              // human-readable explanation
    pub risk_assessment: RiskAssessment,
    pub alternatives: Vec<String>,      // what happens if denied
}

pub struct RiskAssessment {
    pub level: RiskLevel,
    pub pii_exposure: Vec<PiiField>,    // which PII this touches
    pub data_destination: DataDestination,
    pub explanation: String,
}

pub enum DataDestination {
    LocalOnly,                          // never leaves the machine
    LocalLlm { provider: String },      // sent to local LLM
    CloudLlm { provider: String },      // sent to cloud API (highest risk)
    ExternalSite { domain: String },    // sent to a broker site for opt-out
}

impl PermissionManager {
    /// Check permission — this is called at every access boundary
    pub async fn check(
        &self,
        subject: &PermissionSubject,
        grant: &PermissionGrant,
    ) -> PermissionResult {
        // 1. Check for existing grant
        if let Some(perm) = self.find_grant(subject, grant).await {
            if !perm.is_expired() {
                self.audit.log_access(subject, grant, "allowed").await;
                return PermissionResult::Allowed;
            }
        }

        // 2. Check for explicit denial
        if self.is_denied(subject, grant).await {
            self.audit.log_access(subject, grant, "denied").await;
            return PermissionResult::Denied;
        }

        // 3. Not yet decided — prompt user
        PermissionResult::NeedsPrompt(self.build_prompt(subject, grant).await)
    }
}
```

### 8.3 Permission Presets & First-Run Wizard

For usability, offer preset permission profiles while allowing full customization:

```rust
pub enum PermissionPreset {
    /// Maximum privacy — no LLM, no network scanning, manual everything
    /// Good for: users who want full control, air-gapped environments
    Paranoid,

    /// Local LLM only, filesystem/email scanning enabled, no cloud APIs
    /// Good for: users with capable local hardware who want AI features
    LocalPrivacy,

    /// Full features with cloud LLMs, PII filtering enforced
    /// Good for: users who want the best experience with reasonable privacy
    Balanced,

    /// Start from scratch — everything disabled, enable as needed
    /// Good for: advanced users who want to build their own permission set
    Custom,
}

impl PermissionPreset {
    pub fn to_default_config(&self) -> PermissionConfig {
        match self {
            Self::Paranoid => PermissionConfig {
                llm_enabled: false,
                file_system_scan: false,
                email_scan: false,
                browser_scan: false,
                network_access: NetworkPolicy::BrokerSitesOnly,
                browser_automation: false,
                background_execution: false,
            },
            Self::LocalPrivacy => PermissionConfig {
                llm_enabled: true,
                llm_routing: RoutingPreference::LocalOnly,
                file_system_scan: true,
                file_system_scope: vec![
                    ScopedPath::documents(),
                    ScopedPath::downloads(),
                    ScopedPath::desktop(),
                ],
                email_scan: true,
                email_scope: EmailScanMode::HeadersOnly,
                browser_scan: true,
                browser_scope: vec![BrowserScanScope::SavedPasswords],
                network_access: NetworkPolicy::BrokerSitesOnly,
                browser_automation: true,
                background_execution: true,
            },
            Self::Balanced => PermissionConfig {
                llm_enabled: true,
                llm_routing: RoutingPreference::PreferLocal {
                    cloud_allowed_tasks: vec![
                        TaskType::EmailGeneration,
                        TaskType::BrokerNavigation,
                    ],
                },
                pii_filter: FilterStrategy::Tokenize,
                file_system_scan: true,
                email_scan: true,
                email_scope: EmailScanMode::FullContent {
                    include_attachments: false,
                    max_age_days: Some(365),
                },
                browser_scan: true,
                network_access: NetworkPolicy::AllowWithPrompt,
                browser_automation: true,
                background_execution: true,
            },
            Self::Custom => PermissionConfig::all_disabled(),
        }
    }
}
```

On first launch, after vault creation, the user is guided through permission setup:

```
┌─────────────────────────────────────────────────────┐
│          Welcome to Spectral                        │
│                                                     │
│  Step 1 of 4: Choose your privacy level             │
│                                                     │
│  ┌─────────────────────────────────────────────┐    │
│  │ 🔒 Paranoid                                 │    │
│  │    No AI, no local scanning, manual only.   │    │
│  │    Maximum control, more manual work.       │    │
│  └─────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────┐    │
│  │ 🏠 Local Privacy              [RECOMMENDED] │    │
│  │    AI runs on YOUR hardware only.           │    │
│  │    Scans local files and email.             │    │
│  │    Nothing leaves your machine.             │    │
│  └─────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────┐    │
│  │ ⚖️  Balanced                                │    │
│  │    Cloud AI with PII redaction.             │    │
│  │    Best experience, good privacy.           │    │
│  └─────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────┐    │
│  │ 🔧 Custom                                   │    │
│  │    Start with everything off.               │    │
│  │    Enable features one by one.              │    │
│  └─────────────────────────────────────────────┘    │
│                                                     │
│  You can change any of these settings at any time.  │
│                                                     │
│                                    [Next →]         │
└─────────────────────────────────────────────────────┘
```

### 8.4 Audit & Transparency System

Every permission check, every PII access, every network call is logged.

```rust
// /crates/spectral-permissions/src/audit.rs

pub struct AuditEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub subject: PermissionSubject,
    pub detail: AuditDetail,
    pub pii_fields_accessed: Vec<PiiField>,  // never the values, just which fields
    pub data_destination: DataDestination,
    pub outcome: AuditOutcome,
}

pub enum AuditEventType {
    PermissionGranted,
    PermissionDenied,
    PermissionRevoked,
    PiiAccessed,
    PiiSentToLlm {
        provider: String,
        filter_applied: FilterStrategy,
    },
    PiiSentToSite {
        domain: String,
        purpose: String,             // "opt-out form submission"
    },
    FilesScanned {
        count: u64,
        paths_root: Vec<PathBuf>,
    },
    EmailsScanned {
        count: u64,
        source: String,
    },
    BrowserDataAccessed {
        browser: String,
        scope: BrowserScanScope,
    },
    NetworkRequest {
        domain: String,
        method: String,
    },
    VaultUnlocked,
    VaultLocked,
}

pub enum AuditOutcome {
    Success,
    Denied,
    Error(String),
}
```

The audit log is viewable in the UI under Settings → Privacy Audit Log, giving users full transparency into exactly what Spectral has accessed, when, and why.

---
