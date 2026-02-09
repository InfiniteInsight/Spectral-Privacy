## 10. Reporting & Progress Dashboard

### 10.1 Report Types

```rust
// /crates/spectral-core/src/reporting.rs

pub enum ReportType {
    /// Overall privacy posture snapshot
    PrivacySummary {
        as_of: DateTime<Utc>,
    },
    /// Network monitoring trends over time
    NetworkTrend {
        period: ReportPeriod,
    },
    /// Broker removal status and timeline
    RemovalProgress {
        period: ReportPeriod,
    },
    /// Local PII discovery findings
    PiiDiscovery {
        scan_id: Uuid,
    },
    /// Comprehensive report combining all above
    Comprehensive {
        period: ReportPeriod,
    },
}

pub enum ReportPeriod {
    Last7Days,
    Last30Days,
    Last90Days,
    AllTime,
    Custom {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
}

pub enum ReportFormat {
    /// Interactive in the UI dashboard
    Dashboard,
    /// Exportable markdown document
    Markdown,
    /// PDF report (via markdown → PDF)
    Pdf,
    /// Machine-readable JSON
    Json,
}

/// Privacy Summary Report Data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySummaryReport {
    pub generated_at: DateTime<Utc>,
    pub period: ReportPeriod,

    // ── Privacy Score ──────────────────────
    pub current_score: PrivacyScore,
    pub score_trend: Vec<ScoreDataPoint>,    // daily scores over period
    pub score_change: f64,                   // delta from period start

    // ── Broker Status ──────────────────────
    pub total_brokers_known: u32,
    pub brokers_scanned: u32,
    pub brokers_with_pii_found: u32,
    pub removals_submitted: u32,
    pub removals_confirmed: u32,
    pub removals_pending: u32,
    pub removals_overdue: u32,
    pub removals_failed: u32,
    pub reappearances: u32,
    pub broker_details: Vec<BrokerStatusDetail>,

    // ── Network Monitoring ─────────────────
    pub avg_daily_broker_contacts: f64,
    pub avg_daily_tracker_contacts: f64,
    pub broker_contact_trend: Vec<TrendDataPoint>,
    pub tracker_contact_trend: Vec<TrendDataPoint>,
    pub new_domains_discovered: Vec<NewDomainEntry>,
    pub top_contacting_processes: Vec<ProcessContactSummary>,

    // ── Local PII ──────────────────────────
    pub local_pii_findings: u32,
    pub critical_findings: u32,
    pub findings_by_type: HashMap<FindingType, u32>,
    pub findings_remediated: u32,

    // ── Communication ──────────────────────
    pub active_email_threads: u32,
    pub threads_awaiting_user: u32,
    pub threads_awaiting_broker: u32,
    pub avg_broker_response_days: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerStatusDetail {
    pub broker_id: String,
    pub broker_name: String,
    pub category: BrokerCategory,
    pub status: BrokerRemovalStatus,
    pub first_found: Option<DateTime<Utc>>,
    pub removal_requested: Option<DateTime<Utc>>,
    pub removal_confirmed: Option<DateTime<Utc>>,
    pub days_since_request: Option<i64>,
    pub legal_deadline: Option<DateTime<Utc>>,
    pub is_overdue: bool,
    pub verification_history: Vec<VerificationCheck>,
    /// Whether this broker has been seen in network telemetry
    pub seen_in_network: bool,
    pub last_network_contact: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreDataPoint {
    pub date: NaiveDate,
    pub score: f64,
    pub grade: PrivacyGrade,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendDataPoint {
    pub date: NaiveDate,
    pub value: f64,
    pub label: Option<String>,  // e.g., "Removed from Spokeo" annotation
}
```

### 10.2 Dashboard Widgets

The frontend dashboard is organized into cards/widgets for at-a-glance status:

```
┌─────────────────────────────────────────────────────────────────┐
│  Spectral Dashboard                                    [Scan ▼] │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────────┐  ┌────────────────────────────────────┐  │
│  │  Privacy Score    │  │  Score Trend (30 days)             │  │
│  │                   │  │                                    │  │
│  │      ┌───┐        │  │  85 ─  ╭─╮                        │  │
│  │      │ B │        │  │  80 ─╭╯  ╰──╮    ╭──╮            │  │
│  │      │ 78│        │  │  75 ╯       ╰──╮╯   ╰──╮  ╭──   │  │
│  │      └───┘        │  │  70 ─           ╰       ╰─╯      │  │
│  │  ▲ +12 from start │  │                                    │  │
│  └──────────────────┘  └────────────────────────────────────┘  │
│                                                                 │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  Broker Removal Status                                     │ │
│  │                                                            │ │
│  │  ■ Confirmed (12)  ■ Pending (4)  ■ Overdue (1)  □ Not   │ │
│  │                                      found (30)            │ │
│  │                                                            │ │
│  │  ⚠ BeenVerified: 8 days overdue (CCPA deadline passed)    │ │
│  │  ◷ Spokeo: 3 days remaining                               │ │
│  │  ◷ Radaris: 12 days remaining                             │ │
│  │  ◷ Intelius: submitted today                              │ │
│  │  ✓ WhitePages: confirmed removed (2 days ago)             │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                 │
│  ┌────────────────────────┐  ┌────────────────────────────┐    │
│  │  Network Activity      │  │  Communications            │    │
│  │  (last 24h)            │  │                             │    │
│  │                        │  │  2 threads awaiting broker  │    │
│  │  Broker contacts: 3    │  │  1 thread needs your reply  │    │
│  │  Ad networks: 47       │  │                             │    │
│  │  Trackers: 23          │  │  [View Threads →]           │    │
│  │                        │  │                             │    │
│  │  ▼ -8% vs baseline     │  └────────────────────────────┘    │
│  │                        │                                     │
│  │  New: pixel.broker.io  │  ┌────────────────────────────┐    │
│  │  [View Details →]      │  │  Local PII Findings         │    │
│  │                        │  │                             │    │
│  └────────────────────────┘  │  🔴 2 critical              │    │
│                               │  🟡 5 medium                │    │
│                               │  🔵 8 informational         │    │
│                               │  [View Findings →]          │    │
│                               └────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

### 10.3 Cross-Correlation Intelligence

The real power comes from correlating data across all modules:

```rust
// /crates/spectral-core/src/correlation.rs

pub struct CrossCorrelationEngine {
    vault: Arc<Vault>,
    netmon: Arc<NetmonEngine>,
    broker_engine: Arc<BrokerEngine>,
    discovery: Arc<DiscoveryOrchestrator>,
}

impl CrossCorrelationEngine {
    /// Example correlations that surface actionable insights:
    pub async fn generate_insights(&self) -> Vec<Insight> {
        let mut insights = Vec::new();

        // 1. "You requested removal from Spokeo 15 days ago, but we're still
        //     seeing DNS queries to spokeo.com from your browser."
        // → Possible: removal not yet processed, or a different browser/device
        //   is still hitting the site

        // 2. "We found your email address in 3 local documents (tax_2023.pdf,
        //     resume_v4.docx, signup_confirmation.eml) AND you're listed on
        //     BeenVerified. The email in these documents matches the one
        //     BeenVerified has."
        // → Suggests how the broker may have obtained the data

        // 3. "Network monitoring shows connections to datatrade.io, which is
        //     a data broker not yet in our scan list. Would you like to add it?"
        // → Discover new brokers from network telemetry

        // 4. "After removing yourself from Spokeo, network contacts to
        //     spokeo.com dropped from 12/day to 0. Removal appears effective."
        // → Network-level confirmation of removal

        // 5. "BeenVerified removal was confirmed, but your data reappeared
        //     after 60 days. Re-submitting removal request."
        // → Reappearance detection triggering automatic re-removal

        insights
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub id: Uuid,
    pub severity: InsightSeverity,
    pub category: InsightCategory,
    pub title: String,
    pub description: String,
    pub evidence: Vec<InsightEvidence>,
    pub suggested_actions: Vec<SuggestedAction>,
    pub generated_at: DateTime<Utc>,
    pub acknowledged: bool,
}

pub enum InsightCategory {
    RemovalVerification,
    NewBrokerDiscovered,
    DataReappearance,
    NetworkAnomaly,
    PiiExposureCorrelation,
    ProgressMilestone,
    ComplianceViolation,
}
```

---
