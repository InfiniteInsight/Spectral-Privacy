## 21. Geolocation & Jurisdiction System

The user's location is not just metadata — it fundamentally shapes the entire removal strategy, from which brokers to prioritize, to what legal language to use, to what timelines apply.

### 21.1 Jurisdiction Model

```rust
// /crates/spectral-core/src/jurisdiction.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserJurisdiction {
    pub country: CountryCode,
    pub region: Option<String>,           // state, province, etc.
    pub applicable_regulations: Vec<PrivacyRegulation>,
    pub strongest_regulation: PrivacyRegulation,
    pub broker_regions: Vec<BrokerRegion>,  // which broker regions matter
}

/// Privacy regulations that may apply based on user + broker location
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PrivacyRegulation {
    // ── United States (state-level) ────────────────────────
    Ccpa,               // California – CA Consumer Privacy Act / CPRA
    Vcdpa,              // Virginia – Consumer Data Protection Act
    Cpa,                // Colorado – Privacy Act
    Ctdpa,              // Connecticut – Data Privacy Act
    Ucpa,               // Utah – Consumer Privacy Act
    Tdpsa,              // Texas – Data Privacy and Security Act
    Modpa,              // Maryland – Online Data Privacy Act
    Mcdpa,              // Minnesota – Consumer Data Privacy Act
    Icdpa,              // Iowa – Consumer Data Protection Act
    Ndpa,               // Nebraska – Data Privacy Act
    Njdpa,              // New Jersey – Data Privacy Act
    Dpdpa,              // Delaware – Personal Data Privacy Act
    Ocdpa,              // Oregon – Consumer Data Privacy Act
    Mtcdpa,             // Montana – Consumer Data Privacy Act
    Incdpa,             // Indiana – Consumer Data Protection Act
    Tipa,               // Tennessee – Information Protection Act
    // (additional state laws added as enacted)

    // ── US Federal ─────────────────────────────────────────
    CanSpam,            // email opt-out rights
    Fcra,               // Fair Credit Reporting Act (background checks)
    Coppa,              // Children's Online Privacy (if applicable)

    // ── International ──────────────────────────────────────
    Gdpr,               // EU General Data Protection Regulation
    UkGdpr,             // UK GDPR (post-Brexit)
    Pipeda,             // Canada – Personal Information Protection
    Lgpd,               // Brazil – Lei Geral de Proteção de Dados
    Appi,               // Japan – Act on Protection of Personal Information
    PdpBill,            // India – Digital Personal Data Protection
    Popia,              // South Africa – Protection of Personal Information Act
    Cpa2020,            // Australia – Privacy Act (Consumer Data Right)
    Pipl,               // China – Personal Information Protection Law

    /// Unknown / no specific privacy law identified
    NoSpecificLaw,
}

impl PrivacyRegulation {
    /// Response deadline the broker must meet
    pub fn response_deadline_days(&self) -> Option<u32> {
        match self {
            Self::Ccpa => Some(45),
            Self::Gdpr | Self::UkGdpr => Some(30),
            Self::Vcdpa | Self::Cpa => Some(45),
            Self::Pipeda => Some(30),
            Self::Lgpd => Some(15),
            Self::Appi => Some(30),
            // ... etc
            Self::NoSpecificLaw => None,
        }
    }

    /// Can the deadline be extended?
    pub fn extension_days(&self) -> Option<u32> {
        match self {
            Self::Ccpa => Some(45),        // 45-day extension allowed
            Self::Gdpr | Self::UkGdpr => Some(60), // up to 2-month extension
            Self::Vcdpa | Self::Cpa => Some(45),
            _ => None,
        }
    }

    /// Regulatory body to complain to if broker doesn't comply
    pub fn regulatory_body(&self) -> Option<RegulatoryBody> {
        match self {
            Self::Ccpa => Some(RegulatoryBody {
                name: "California Privacy Protection Agency (CPPA)".into(),
                complaint_url: Some("https://cppa.ca.gov/complaints/".into()),
                description: "File a complaint for CCPA/CPRA violations.".into(),
            }),
            Self::Gdpr => Some(RegulatoryBody {
                name: "Your national Data Protection Authority (DPA)".into(),
                complaint_url: None,  // varies by EU member state
                description: "Contact your country's DPA. Spectral can help \
                    identify the correct authority.".into(),
            }),
            // ... etc
            Self::NoSpecificLaw => None,
        }
    }

    /// Legal citation text to include in removal request emails
    pub fn citation_text(&self) -> String {
        match self {
            Self::Ccpa => "Pursuant to the California Consumer Privacy Act \
                (Cal. Civ. Code §1798.100 et seq.) and the California Privacy \
                Rights Act, I request the deletion of all personal information \
                your organization has collected about me.".into(),
            Self::Gdpr => "In accordance with Article 17 of the General Data \
                Protection Regulation (EU) 2016/679, I request the erasure of \
                all personal data you hold relating to me.".into(),
            Self::Modpa => "Pursuant to the Maryland Online Data Privacy Act \
                (effective October 1, 2025), I request the deletion of all \
                personal data your organization has collected about me.".into(),
            // ... etc
            Self::NoSpecificLaw => "I request the deletion of all personal \
                information your organization has collected about me.".into(),
        }
    }
}
```

### 21.2 How Jurisdiction Flows Through the System

```
UserJurisdiction (set in onboarding)
       │
       ├──► Broker Engine (Section 3.4)
       │    • Filter broker list by region relevance
       │    • Prioritize brokers registered in user's jurisdiction
       │    • Tag broker results with applicable regulation
       │
       ├──► Mail Engine (Section 7)
       │    • Select legal citation text for removal emails
       │    • Set response deadline expectations
       │    • Use jurisdiction-appropriate escalation language
       │
       ├──► Verification Engine (Section 6)
       │    • Set legal deadlines per regulation
       │    • Track extension requests
       │    • Flag overdue responses with correct legal context
       │
       ├──► Escalation Pipeline (Section 6.3)
       │    • Tier 3 escalation references correct regulatory body
       │    • Generate jurisdiction-specific complaint templates
       │    • If no specific law: template emphasizes ethical/reputational pressure
       │
       ├──► Dashboard (Section 10)
       │    • Show which laws protect the user
       │    • Display legal deadline countdown per broker
       │    • Flag when a broker is in a jurisdiction with weak/no protections
       │
       └──► Proactive Scan (Section 22)
            • Auto-scan list filtered to regionally relevant brokers
            • US user → prioritize US people-search sites
            • EU user → prioritize GDPR-applicable brokers + EU data processors
```

### 21.3 Dual-Jurisdiction Resolution

The applicable law often depends on both the user's location AND the broker's jurisdiction. Spectral resolves this by applying the strongest available regulation:

```rust
pub fn resolve_applicable_regulation(
    user: &UserJurisdiction,
    broker: &BrokerDefinition,
) -> PrivacyRegulation {
    // If the user is in the EU, GDPR applies regardless of broker location
    // (GDPR has extraterritorial reach for EU data subjects)
    if user.country.is_eu_eea() {
        return PrivacyRegulation::Gdpr;
    }

    // If the user is in the UK, UK GDPR applies
    if user.country == CountryCode::GB {
        return PrivacyRegulation::UkGdpr;
    }

    // For US users: check if the user's state has a privacy law
    if user.country == CountryCode::US {
        if let Some(state_law) = user.state_privacy_law() {
            return state_law;
        }

        // Even without a state law, CCPA may apply if the broker
        // is registered in California or does business there
        if broker.registered_state == Some("CA".into())
            || broker.california_registered_data_broker
        {
            return PrivacyRegulation::Ccpa;
        }
    }

    // If the broker is in a jurisdiction with strong laws, those may apply
    if let Some(broker_reg) = broker.home_regulation() {
        return broker_reg;
    }

    // No specific law applies — use general deletion request language
    PrivacyRegulation::NoSpecificLaw
}
```

### 21.4 Jurisdictions Without Strong Privacy Laws

For users in states/countries with weak or no specific privacy law, Spectral is transparent rather than bluffing:

- Removal request templates use polite but firm language without citing specific statutes
- Dashboard shows "Limited legal protections — removal requests are voluntary for this broker"
- The app still submits requests — many brokers honor opt-outs regardless of legal obligation
- Escalation options focus on reputational pressure (BBB complaints, social media exposure) rather than regulatory complaints
- If the broker IS registered in a state with a law (many brokers are CA-registered), Spectral can leverage that

---
