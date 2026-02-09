## 6. Removal Verification & Follow-Up Engine (`spectral-verify`)

### 6.1 Verification Pipeline

Submitting a removal request is only half the battle. Spectral must verify that brokers actually comply, track response timelines against legal requirements, and escalate when they don't.

```
Removal Request Submitted
        │
        ▼
  ┌─────────────┐     ┌──────────────────┐
  │  Wait Timer  │────►│  Re-scan Broker   │
  │  (per broker │     │  for User's PII   │
  │   SLA)       │     └────────┬─────────┘
  └──────────────┘              │
                     ┌──────────┴──────────┐
                     │                     │
                  PII Gone              PII Still There
                     │                     │
                     ▼                     ▼
              Mark Confirmed        ┌──────────────┐
                                    │  Escalation   │
                                    │  Pipeline     │
                                    └──────┬───────┘
                                           │
                              ┌────────────┼───────────┐
                              ▼            ▼           ▼
                         Re-submit    Email Follow    Flag for
                         Opt-Out      Up (w/ legal    Manual
                                      citation)      Escalation
```

### 6.2 Core Types

```rust
// /crates/spectral-verify/src/lib.rs

pub struct VerificationEngine {
    broker_engine: Arc<BrokerEngine>,
    browser: Arc<BrowserEngine>,
    scheduler: Arc<Scheduler>,
    mailer: Arc<MailEngine>,
    vault: Arc<Vault>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationSchedule {
    pub broker_result_id: Uuid,
    pub broker_id: String,
    /// When the removal was first requested
    pub removal_requested_at: DateTime<Utc>,
    /// Broker's stated or legal removal timeframe
    pub expected_completion: DateTime<Utc>,
    /// Verification check schedule
    pub check_schedule: Vec<ScheduledCheck>,
    /// Current status
    pub status: VerificationStatus,
    /// History of all checks
    pub check_history: Vec<VerificationCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledCheck {
    pub check_at: DateTime<Utc>,
    pub check_type: CheckType,
    pub completed: bool,
}

pub enum CheckType {
    /// Re-scan the broker site for the user's listing
    WebRescan,
    /// Check email for broker response
    EmailCheck,
    /// Verify via broker's API if available
    ApiCheck,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationStatus {
    /// Waiting for broker's stated timeframe to pass
    WaitingForSla {
        expected_by: DateTime<Utc>,
        days_remaining: i64,
    },
    /// SLA period passed, running verification scans
    Verifying,
    /// Confirmed removed — PII no longer found on broker site
    Confirmed {
        confirmed_at: DateTime<Utc>,
        method: VerificationMethod,
    },
    /// Still found after SLA — escalation in progress
    Overdue {
        days_overdue: i64,
        escalation_level: EscalationLevel,
    },
    /// Removal was confirmed but PII reappeared
    Reappeared {
        reappeared_at: DateTime<Utc>,
        original_removal_at: DateTime<Utc>,
    },
    /// Broker responded — needs parsing/action
    BrokerResponded {
        response_summary: String,
        needs_user_action: bool,
    },
    /// Failed permanently — broker refused or is unreachable
    Failed {
        reason: String,
        suggested_action: String,
    },
}

pub enum EscalationLevel {
    /// First: re-submit the opt-out request
    Level1Resubmit,
    /// Second: send a follow-up email citing legal requirements
    Level2LegalEmail,
    /// Third: alert user for manual escalation (regulatory complaint, etc.)
    Level3ManualEscalation,
}

pub enum VerificationMethod {
    WebScanNegative,      // scanned broker site, listing no longer appears
    ApiConfirmation,      // broker's API confirmed deletion
    EmailConfirmation,    // broker sent confirmation email
    ManualConfirmation,   // user manually confirmed
}

impl VerificationEngine {
    /// Schedule verification checks for a new removal request
    pub async fn schedule_verification(
        &self,
        broker_result: &BrokerResult,
        broker: &BrokerDefinition,
    ) -> Result<VerificationSchedule> {
        let sla_days = broker.typical_removal_time.num_days();
        let requested_at = Utc::now();
        let expected_by = requested_at + broker.typical_removal_time;

        let checks = vec![
            // Check at 50% of SLA (early detection of fast removals)
            ScheduledCheck {
                check_at: requested_at + (broker.typical_removal_time / 2),
                check_type: CheckType::WebRescan,
                completed: false,
            },
            // Check at SLA deadline
            ScheduledCheck {
                check_at: expected_by,
                check_type: CheckType::WebRescan,
                completed: false,
            },
            // Check at SLA + 3 days (grace period)
            ScheduledCheck {
                check_at: expected_by + Duration::days(3),
                check_type: CheckType::WebRescan,
                completed: false,
            },
            // If not removed by SLA + 7, escalate
            ScheduledCheck {
                check_at: expected_by + Duration::days(7),
                check_type: CheckType::EmailCheck,
                completed: false,
            },
        ];

        // ... store and return schedule
        todo!()
    }
}
```

### 6.3 Legal Timeline Tracking

Different jurisdictions have different legal requirements for data deletion timelines. Spectral tracks these to know when a broker is actually violating the law versus just being slow.

```rust
// /crates/spectral-verify/src/legal.rs

pub struct LegalTimeline {
    pub regulation: PrivacyRegulation,
    pub max_response_days: u32,
    pub max_deletion_days: u32,
    pub allows_extension: bool,
    pub extension_max_days: Option<u32>,
    pub complaint_authority: Option<String>,
    pub complaint_url: Option<String>,
}

pub enum PrivacyRegulation {
    /// California Consumer Privacy Act / California Privacy Rights Act
    CcpaCpra {
        /// 45 days to respond, one 45-day extension allowed
        response_deadline_days: u32,     // 45
        extension_days: u32,             // 45
    },
    /// EU General Data Protection Regulation
    Gdpr {
        /// 30 days, extendable by 60 days for complex requests
        response_deadline_days: u32,     // 30
        extension_days: u32,             // 60
    },
    /// Virginia Consumer Data Protection Act
    Vcdpa {
        response_deadline_days: u32,     // 45
        extension_days: u32,             // 45
    },
    /// Colorado Privacy Act
    Cpa {
        response_deadline_days: u32,     // 45
        extension_days: u32,             // 45
    },
    /// Canada PIPEDA
    Pipeda {
        response_deadline_days: u32,     // 30
    },
    /// Generic / Unknown jurisdiction
    Generic {
        assumed_deadline_days: u32,      // 45 (reasonable assumption)
    },
}

impl LegalTimeline {
    /// Determine applicable regulation based on user location and broker location
    pub fn determine(
        user_state: Option<&str>,
        user_country: &str,
        broker_jurisdiction: Option<&str>,
    ) -> Self {
        // CCPA applies if user is in California OR broker does business in California
        // GDPR applies if user is in EU OR broker offers services to EU residents
        // Most protective regulation wins when multiple apply
        todo!()
    }

    /// Generate a legal citation string for follow-up emails
    pub fn citation_text(&self) -> String {
        match &self.regulation {
            PrivacyRegulation::CcpaCpra { .. } => {
                "Under the California Consumer Privacy Act (CCPA) / California Privacy \
                 Rights Act (CPRA), Cal. Civ. Code § 1798.105, you are required to \
                 delete my personal information within 45 business days of receiving \
                 a verifiable consumer request.".to_string()
            },
            PrivacyRegulation::Gdpr { .. } => {
                "Under the General Data Protection Regulation (GDPR), Article 17 \
                 (Right to Erasure), you are required to erase personal data without \
                 undue delay, and in any event within one month of receipt of the \
                 request.".to_string()
            },
            // ... other regulations
            _ => todo!()
        }
    }
}
```

---
