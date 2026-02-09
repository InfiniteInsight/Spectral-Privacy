## 5. Network Telemetry Engine (`spectral-netmon`)

### 5.1 Architecture Overview

Spectral passively monitors the user's network activity to build a picture of which data brokers, ad platforms, and tracking services are receiving data from the user's machine. This creates a baseline, tracks improvement over time, and can surface connections the user didn't know existed.

This is **not** a firewall or packet inspector — it reads existing OS-level connection metadata (DNS cache, active connections, established sessions) and correlates it against a maintained database of known data brokers, ad networks, and tracking domains.

```
┌───────────────────────────────────────────────────────────┐
│                  Network Telemetry Engine                  │
│                                                           │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │  Collector   │  │  Correlator  │  │   Baseline      │  │
│  │  Scheduler   │  │  & Enricher  │  │   Tracker       │  │
│  └──────┬──────┘  └──────┬───────┘  └───────┬─────────┘  │
│         │                │                   │            │
│  ┌──────┴──────────────────────────────────────────────┐  │
│  │              Data Source Adapters                    │  │
│  │                                                     │  │
│  │  ┌──────────┐ ┌───────────┐ ┌────────────────────┐  │  │
│  │  │   DNS    │ │  Netstat  │ │  Hosts / Firewall  │  │  │
│  │  │  Cache   │ │  / ss     │ │  Log Ingest        │  │  │
│  │  └──────────┘ └───────────┘ └────────────────────┘  │  │
│  └─────────────────────────────────────────────────────┘  │
│                                                           │
│  ┌─────────────────────────────────────────────────────┐  │
│  │              Domain Intelligence DB                 │  │
│  │  Known brokers, ad networks, trackers, CDNs,        │  │
│  │  analytics platforms — community maintained          │  │
│  └─────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────┘
```

### 5.2 Data Source Adapters

```rust
// /crates/spectral-netmon/src/lib.rs

#[async_trait]
pub trait NetworkDataSource: Send + Sync {
    fn source_id(&self) -> &str;
    fn platform_support(&self) -> &[Platform];
    async fn collect(&self) -> Result<Vec<NetworkObservation>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkObservation {
    pub timestamp: DateTime<Utc>,
    pub source: String,                  // "dns_cache", "netstat", "firewall_log"
    pub observation_type: ObservationType,
    pub domain: Option<String>,
    pub ip_address: IpAddr,
    pub port: Option<u16>,
    pub protocol: Option<Protocol>,
    pub process_name: Option<String>,    // which local process made the connection
    pub process_pid: Option<u32>,
    pub state: Option<ConnectionState>,  // established, time_wait, etc.
    pub direction: ConnectionDirection,
}

pub enum ObservationType {
    DnsResolution,        // domain was resolved (from DNS cache)
    ActiveConnection,     // currently open TCP/UDP connection
    RecentConnection,     // recently closed connection (TIME_WAIT, etc.)
    FirewallEvent,        // logged by host firewall
}

pub enum ConnectionDirection {
    Outbound,
    Inbound,
    Unknown,
}
```

### 5.3 Platform-Specific Collectors

```rust
// /crates/spectral-netmon/src/collectors/dns.rs

pub struct DnsCacheCollector;

impl DnsCacheCollector {
    /// Windows: ipconfig /displaydns
    /// macOS: sudo dscacheutil -cachedump (or mDNSResponder log parsing)
    /// Linux: systemd-resolve --statistics + /etc/hosts, or
    ///        parse /var/log/syslog for dnsmasq/systemd-resolved entries
    ///
    /// NOTE: Linux DNS cache availability varies significantly by distribution.
    /// systemd-resolved caches by default on Ubuntu/Fedora.
    /// Other distros may use dnsmasq, unbound, or no local cache at all.
    /// The collector should detect what's available and adapt.
    async fn collect_platform(&self) -> Result<Vec<NetworkObservation>> {
        #[cfg(target_os = "windows")]
        return self.collect_windows().await;

        #[cfg(target_os = "macos")]
        return self.collect_macos().await;

        #[cfg(target_os = "linux")]
        return self.collect_linux().await;
    }
}

// /crates/spectral-netmon/src/collectors/connections.rs

pub struct ConnectionCollector;

impl ConnectionCollector {
    /// Windows: netstat -ano  (or Get-NetTCPConnection in PowerShell)
    /// macOS: netstat -anv  or  lsof -i -n -P
    /// Linux: ss -tunap  (preferred over netstat, more info, faster)
    ///
    /// We parse the output to get:
    /// - Remote IP + port
    /// - Local port (for process correlation)
    /// - Process name/PID
    /// - Connection state
    async fn collect_platform(&self) -> Result<Vec<NetworkObservation>> {
        #[cfg(target_os = "windows")]
        return self.collect_windows().await;

        #[cfg(target_os = "macos")]
        return self.collect_macos().await;

        #[cfg(target_os = "linux")]
        return self.collect_linux().await;
    }
}

// /crates/spectral-netmon/src/collectors/firewall.rs

pub struct FirewallLogCollector;

impl FirewallLogCollector {
    /// Optional — reads firewall logs if available and permitted
    /// Windows: Windows Firewall log (%systemroot%\system32\LogFiles\Firewall\pfirewall.log)
    /// macOS: /var/log/appfirewall.log (Application Firewall)
    /// Linux: iptables/nftables logs in /var/log/kern.log or journalctl
    ///
    /// This collector is best-effort — many systems don't have firewall logging enabled.
    /// We should detect availability and inform the user if they want to enable it.
    async fn collect_platform(&self) -> Result<Vec<NetworkObservation>> {
        todo!()
    }
}
```

### 5.4 Domain Intelligence Database

A community-maintained database of known data brokers, ad networks, trackers, and analytics platforms. This is what we correlate observations against.

```rust
// /crates/spectral-netmon/src/intelligence.rs

pub struct DomainIntelligenceDb {
    /// In-memory trie for fast domain matching (including subdomain matching)
    domain_trie: DomainTrie,
    /// Source data version
    version: semver::Version,
    last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEntry {
    pub domain: String,                   // "spokeo.com", "*.doubleclick.net"
    pub category: DomainCategory,
    pub entity: String,                   // parent company: "Google", "Oracle", "Acxiom"
    pub subcategory: Option<String>,      // more specific classification
    pub risk_level: RiskLevel,
    pub description: String,
    pub is_data_broker: bool,
    pub known_data_types: Vec<String>,    // what kind of data they collect/sell
    pub opt_out_url: Option<String>,      // link to opt-out if applicable
    pub related_broker_id: Option<String>,// link to spectral broker definition
    pub sources: Vec<String>,            // where this classification comes from
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomainCategory {
    DataBroker,
    AdNetwork,
    Tracker,
    Analytics,
    Fingerprinting,
    SocialMediaTracker,
    EmailTracker,          // tracking pixels, open tracking
    CdnWithTracking,       // CDNs that also serve tracking (some Google/Cloudflare endpoints)
    RetargetingPlatform,
    DataManagementPlatform,
    ConsentManagement,     // ironic, but some CMPs track too
    Telemetry,             // OS/app telemetry endpoints
    Benign,                // known safe — CDNs, OS updates, etc. (used for noise filtering)
}

impl DomainIntelligenceDb {
    /// Look up a domain (including subdomain matching)
    /// "tracker.spokeo.com" matches "*.spokeo.com" and "spokeo.com"
    pub fn lookup(&self, domain: &str) -> Option<&DomainEntry> {
        self.domain_trie.longest_match(domain)
    }

    /// Classify an IP address by reverse DNS + known IP ranges
    pub async fn classify_ip(&self, ip: IpAddr) -> Option<DomainEntry> {
        // 1. Reverse DNS lookup
        // 2. Check known IP ranges (e.g., Google ad-serving ranges)
        // 3. Fall back to ASN lookup for organization name
        todo!()
    }
}
```

**Domain intelligence sources** (to seed and maintain the database):
- EasyList / EasyPrivacy (well-established ad/tracker blocklists)
- Disconnect.me tracking protection lists
- Steven Black's hosts file aggregation
- OISD blocklist
- Spectral's own community-contributed broker domain list
- Public data broker registries (California, Vermont, etc. maintain legal registries)

**Example domain definition files:**

```toml
# /domains/data-brokers/spokeo.toml
[domain]
domain = "spokeo.com"
also_match = ["*.spokeo.com"]
category = "DataBroker"
entity = "Spokeo, Inc."
risk_level = "High"
is_data_broker = true
description = "People search engine aggregating public records, social media, and other sources"
known_data_types = ["name", "address", "phone", "email", "relatives", "social_profiles"]
opt_out_url = "https://www.spokeo.com/optout"
related_broker_id = "spokeo"
sources = ["california_data_broker_registry", "manual_verification"]
```

```toml
# /domains/ad-networks/doubleclick.toml
[domain]
domain = "doubleclick.net"
also_match = ["*.doubleclick.net", "*.googlesyndication.com", "*.googleadservices.com"]
category = "AdNetwork"
entity = "Google LLC"
subcategory = "display_advertising"
risk_level = "Medium"
is_data_broker = false
description = "Google's display advertising network, serves targeted ads across the web"
known_data_types = ["browsing_behavior", "ad_interests", "device_fingerprint"]
opt_out_url = "https://adssettings.google.com"
sources = ["easylist", "disconnect"]
```

### 5.5 Collection Scheduling & Baseline Building

```rust
// /crates/spectral-netmon/src/scheduler.rs

pub struct NetmonScheduler {
    config: NetmonConfig,
    collectors: Vec<Box<dyn NetworkDataSource>>,
    intelligence: Arc<DomainIntelligenceDb>,
    vault: Arc<Vault>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetmonConfig {
    /// How often to collect network observations
    pub collection_interval: Duration,     // default: every 15 minutes
    /// How long to retain raw observations (rolled up into summaries after)
    pub raw_retention: Duration,           // default: 7 days
    /// How long to retain daily summaries
    pub summary_retention: Duration,       // default: 365 days
    /// Whether to resolve IPs to domains for unmatched connections
    pub reverse_dns_enabled: bool,         // default: true
    /// Whether to attempt process name resolution
    pub process_resolution: bool,          // default: true (requires elevated permissions on some OS)
    /// Domains to ignore (user's own infrastructure, known-safe, etc.)
    pub ignore_domains: Vec<String>,
    /// Only alert on these categories (reduce noise)
    pub alert_categories: Vec<DomainCategory>,
}

/// Stored observation after correlation with intelligence DB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedObservation {
    pub raw: NetworkObservation,
    pub classification: Option<DomainEntry>,
    pub is_new_contact: bool,             // first time seeing this domain
    pub process_context: Option<ProcessContext>,
}

pub struct ProcessContext {
    pub name: String,
    pub pid: u32,
    pub executable_path: Option<PathBuf>,
    pub is_browser: bool,
    pub is_system: bool,
}
```

### 5.6 Privacy Score Calculation

```rust
// /crates/spectral-netmon/src/scoring.rs

/// Privacy score: 0 (terrible) to 100 (excellent)
/// This is a composite metric that gives users a single number to track progress.
pub struct PrivacyScoreCalculator;

impl PrivacyScoreCalculator {
    pub fn calculate(summary: &DailySummary, context: &ScoringContext) -> PrivacyScore {
        let mut score = 100.0_f64;

        // Deductions for data broker contacts (heaviest penalty)
        // Each unique broker domain costs 3 points, capped at -40
        let broker_penalty = (summary.unique_broker_domains as f64 * 3.0).min(40.0);
        score -= broker_penalty;

        // Deductions for ad network contacts
        // Each unique ad domain costs 0.5 points, capped at -20
        let ad_penalty = (summary.unique_ad_domains as f64 * 0.5).min(20.0);
        score -= ad_penalty;

        // Deductions for tracker contacts
        // Each unique tracker domain costs 1 point, capped at -20
        let tracker_penalty = (summary.unique_tracker_domains as f64 * 1.0).min(20.0);
        score -= tracker_penalty;

        // Bonus: successful removals improve score
        let removal_bonus = (context.confirmed_removals as f64 * 2.0).min(15.0);
        score += removal_bonus;

        // Bonus: pending removals show progress
        let pending_bonus = (context.pending_removals as f64 * 0.5).min(5.0);
        score += pending_bonus;

        PrivacyScore {
            overall: score.clamp(0.0, 100.0),
            breakdown: ScoreBreakdown {
                broker_penalty,
                ad_penalty,
                tracker_penalty,
                removal_bonus,
                pending_bonus,
            },
            grade: PrivacyGrade::from_score(score),
        }
    }
}

pub enum PrivacyGrade {
    A,      // 90-100: Excellent — minimal broker exposure, few trackers
    B,      // 75-89:  Good — some exposure, actively being managed
    C,      // 60-74:  Fair — moderate exposure, room for improvement
    D,      // 40-59:  Poor — significant exposure to brokers and trackers
    F,      // 0-39:   Critical — widespread exposure, immediate action needed
}
```

---
