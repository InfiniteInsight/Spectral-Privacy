## 4. Local PII Discovery Engine (`spectral-discovery`)

A core crate that scans the user's local environment to find where their PII exists beyond data brokers — in files, emails, browser data, and application databases.

### 4.1 Discovery Architecture

```
┌─────────────────────────────────────────────────────┐
│              Discovery Orchestrator                  │
│  ┌─────────────┐ ┌──────────┐ ┌──────────────────┐  │
│  │  Scan Plan   │ │ Progress │ │  Result Merger   │  │
│  │  Builder     │ │ Tracker  │ │  & Deduplicator  │  │
│  └──────┬──────┘ └────┬─────┘ └────────┬─────────┘  │
│         │             │                │             │
├─────────┴─────────────┴────────────────┴─────────────┤
│              Scanner Interface (trait)                │
├─────────┬──────────────┬───────────────┬─────────────┤
│         │              │               │             │
│  ┌──────┴──────┐ ┌─────┴─────┐ ┌──────┴──────────┐  │
│  │ FileSystem  │ │   Email   │ │    Browser      │  │
│  │ Scanner     │ │  Scanner  │ │    Scanner      │  │
│  └──────┬──────┘ └─────┬─────┘ └──────┬──────────┘  │
│         │              │               │             │
│  ┌──────┴──────┐ ┌─────┴─────┐ ┌──────┴──────────┐  │
│  │ Regex/NER   │ │   IMAP    │ │ SQLite/LevelDB  │  │
│  │ + LLM(opt)  │ │  + mbox   │ │  Profile Parse  │  │
│  └─────────────┘ └───────────┘ └─────────────────┘  │
└─────────────────────────────────────────────────────┘
```

### 4.2 Core Types

```rust
// /crates/spectral-discovery/src/lib.rs

#[async_trait]
pub trait Scanner: Send + Sync {
    /// Unique identifier for this scanner
    fn scanner_id(&self) -> &str;

    /// What permissions does this scanner need?
    fn required_permissions(&self) -> ScannerPermissions;

    /// Execute the scan within the granted permission boundaries
    async fn scan(
        &self,
        profile: &UserProfile,
        permissions: &GrantedPermissions,
        progress: &dyn ProgressReporter,
        llm: Option<&dyn LlmProvider>,     // None if LLM disabled
    ) -> Result<Vec<DiscoveryFinding>>;

    /// Estimate scan scope (for progress bars and user expectations)
    async fn estimate_scope(
        &self,
        permissions: &GrantedPermissions,
    ) -> Result<ScanEstimate>;
}

pub struct DiscoveryFinding {
    pub id: Uuid,
    pub scanner_id: String,
    pub finding_type: FindingType,
    pub pii_types: Vec<PiiType>,
    pub location: FindingLocation,
    pub confidence: ConfidenceScore,
    pub context_snippet: Option<String>,   // redacted snippet showing where PII was found
    pub discovered_at: DateTime<Utc>,
    pub risk_level: RiskLevel,
    pub recommended_action: RecommendedAction,
}

pub enum FindingType {
    /// PII found in a document
    DocumentPii {
        file_path: PathBuf,
        file_type: String,
        byte_offset: Option<u64>,
    },
    /// PII found in an email
    EmailPii {
        mailbox: String,
        message_id: String,
        subject_redacted: String,        // subject with PII redacted
        sender: String,
        date: DateTime<Utc>,
    },
    /// Account/service that has your PII
    ServiceAccount {
        service_name: String,
        service_domain: String,
        evidence: AccountEvidence,
    },
    /// PII in browser data
    BrowserPii {
        browser: String,
        data_type: BrowserDataType,      // saved passwords, autofill, history
        profile_name: Option<String>,
    },
}

pub enum BrowserDataType {
    SavedPassword { domain: String },
    AutofillEntry { field_type: String },
    BrowsingHistory { domain: String },
    Cookie { domain: String },
    CachedForm { domain: String },
}

#[derive(Debug, Clone)]
pub enum RiskLevel {
    Critical,     // SSN, financial account numbers in plaintext files
    High,         // Full address + DOB together, unencrypted password stores
    Medium,       // Email + name combinations, phone numbers in documents
    Low,          // Name only, publicly known information
    Informational // Service accounts (not a risk, but useful to know)
}

pub enum RecommendedAction {
    DeleteFile { path: PathBuf },
    EncryptFile { path: PathBuf },
    RedactFromFile { path: PathBuf, ranges: Vec<ByteRange> },
    DeleteEmail { mailbox: String, message_id: String },
    CloseAccount { service: String, url: Option<String> },
    ChangePassword { service: String, url: Option<String> },
    ReviewManually { reason: String },
    ClearBrowserData { browser: String, data_type: BrowserDataType },
}

pub struct ConfidenceScore {
    /// 0.0 - 1.0 overall confidence this finding contains the user's PII
    pub overall: f32,
    /// How confident are we in the PII type classification?
    pub classification: f32,
    /// How was this determined?
    pub method: DetectionMethod,
}

pub enum DetectionMethod {
    ExactMatch,           // direct string match against known PII
    RegexPattern,         // structural pattern match (SSN format, phone format, etc.)
    FuzzyMatch {          // approximate string matching
        similarity: f32,
    },
    NerExtraction,        // named entity recognition (rule-based)
    LlmClassification {   // LLM determined this is PII
        model: String,
        reasoning: String,
    },
    Composite(Vec<DetectionMethod>),
}
```

### 4.3 Filesystem Scanner

```rust
// /crates/spectral-discovery/src/scanners/filesystem.rs

pub struct FileSystemScanner {
    /// Supported file types and their parsers
    parsers: HashMap<String, Box<dyn FileParser>>,
    /// PII detection pipeline
    detector: PiiDetector,
}

/// File type parsers — extract text content for PII scanning
#[async_trait]
pub trait FileParser: Send + Sync {
    fn supported_extensions(&self) -> &[&str];
    async fn extract_text(&self, path: &Path) -> Result<ExtractedContent>;
}

/// Built-in parsers
pub struct PlainTextParser;          // .txt, .csv, .tsv, .log, .json, .xml, .yaml, .toml
pub struct OfficeDocumentParser;     // .docx, .xlsx, .pptx (via zip + xml parsing)
pub struct PdfParser;                // .pdf (via poppler or pdf-extract)
pub struct ImageOcrParser;           // .jpg, .png (optional, requires Tesseract or LLM vision)
pub struct ArchiveParser;            // .zip, .tar.gz (scan contents)

/// PII detection pipeline — works with or without LLM
pub struct PiiDetector {
    /// Always available: pattern-based detection
    patterns: Vec<PiiPattern>,
    /// Optional: LLM-enhanced detection for unstructured text
    llm: Option<Arc<dyn LlmProvider>>,
}

pub struct PiiPattern {
    pub pii_type: PiiType,
    pub regex: Regex,
    pub validator: Option<Box<dyn Fn(&str) -> bool>>,  // e.g., Luhn check for CC numbers
    pub context_patterns: Vec<Regex>,                   // nearby text that increases confidence
}

/// What the regex/NER layer catches (no LLM needed):
/// - SSN patterns: \d{3}-\d{2}-\d{4} with Luhn-adjacent validation
/// - Phone numbers: various formats, validated against user's known numbers
/// - Email addresses: standard regex, matched against user's known emails
/// - Physical addresses: street patterns + ZIP, matched against known addresses
/// - Credit card numbers: \d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4} + Luhn check
/// - Date of birth: various date formats near context words ("born", "DOB", "birthday")
/// - Full name: exact and fuzzy match against profile name + aliases
///
/// What the LLM adds (when enabled):
/// - Contextual understanding: "my social is hidden in this note"
/// - Implicit PII: references that imply PII without containing it directly
/// - Document classification: "this is a tax return" → likely contains SSN
/// - Handwritten text in images (via vision model)
/// - Foreign language PII detection
```

### 4.4 Email Scanner

```rust
// /crates/spectral-discovery/src/scanners/email.rs

pub struct EmailScanner {
    detector: PiiDetector,
}

pub enum EmailSource {
    /// Connect to IMAP server (user provides credentials, stored in vault)
    Imap {
        server: String,
        port: u16,
        username: String,
        // password retrieved from vault at scan time
    },
    /// Scan local mailbox files
    LocalMbox {
        path: PathBuf,
    },
    /// Scan Thunderbird profile
    ThunderbirdProfile {
        profile_path: PathBuf,
    },
    /// Export file (e.g., Google Takeout mbox)
    ExportFile {
        path: PathBuf,
        format: ExportFormat,
    },
}

/// Email scanning strategy — two modes
pub enum EmailScanMode {
    /// Fast: scan headers + sender domains only
    /// Finds: services you have accounts with, newsletters, data brokers who email you
    /// PII exposure: minimal (just your email address is confirmed)
    HeadersOnly,

    /// Deep: scan full email bodies
    /// Finds: PII in email content, attachments with sensitive data
    /// PII exposure: email content is processed locally
    FullContent {
        include_attachments: bool,
        max_age_days: Option<u32>,       // limit scope to recent emails
    },
}

/// Key discovery target: what services have your PII?
/// By scanning email, we can identify services you've signed up for
/// and help you understand your PII surface area.
pub struct ServiceDiscovery {
    /// Known service domain mappings
    known_services: HashMap<String, ServiceInfo>,
}

pub struct ServiceInfo {
    pub name: String,
    pub domain: String,
    pub category: ServiceCategory,
    pub has_data_deletion_page: Option<String>,
    pub is_data_broker: bool,
    pub privacy_policy_url: Option<String>,
}

pub enum ServiceCategory {
    SocialMedia,
    Shopping,
    Financial,
    Healthcare,
    Government,
    DataBroker,
    Newsletter,
    SaaS,
    Gaming,
    Travel,
    Other(String),
}
```

### 4.5 Browser Data Scanner

```rust
// /crates/spectral-discovery/src/scanners/browser.rs

pub struct BrowserScanner {
    detector: PiiDetector,
}

pub enum BrowserTarget {
    Chrome { profile_path: Option<PathBuf> },
    Firefox { profile_path: Option<PathBuf> },
    Edge { profile_path: Option<PathBuf> },
    Brave { profile_path: Option<PathBuf> },
    Safari,                               // macOS only, different storage format
    Custom { profile_path: PathBuf },
}

pub enum BrowserScanScope {
    SavedPasswords,      // identifies services with accounts (passwords NOT extracted)
    AutofillData,        // names, addresses, phone numbers, CC numbers in autofill
    BrowsingHistory,     // visits to data broker sites, account creation pages
    Cookies,             // identifies services with active sessions
    CachedFormData,      // form submissions that may contain PII
}

/// NOTE: We NEVER extract or store actual passwords.
/// For saved passwords, we only record the DOMAIN to identify accounts.
/// The credential data itself is never read, decrypted, or stored.
```

---
