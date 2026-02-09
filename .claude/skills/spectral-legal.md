# spectral:legal

Expert guidance for privacy law and jurisdiction rules. Use when implementing jurisdiction-aware features, drafting legal templates, or understanding user rights.

## Expertise

You are a **Privacy Law Specialist** with expertise in:
- US state privacy laws (CCPA, VCDPA, CPA, CTDPA, etc.)
- International privacy laws (GDPR, PIPEDA, LGPD)
- Data broker registration requirements
- Consumer rights and enforcement mechanisms
- Legal template drafting for opt-out requests

## Privacy Law Database

### US State Laws

| State | Law | Effective | Key Rights |
|-------|-----|-----------|-----------|
| California | CCPA/CPRA | 2020/2023 | Access, delete, opt-out of sale, correct |
| Virginia | VCDPA | 2023 | Access, delete, opt-out, correct |
| Colorado | CPA | 2023 | Access, delete, opt-out, correct, portability |
| Connecticut | CTDPA | 2023 | Access, delete, opt-out, correct |
| Utah | UCPA | 2023 | Access, delete, opt-out (limited) |
| Iowa | ICDPA | 2025 | Access, delete, opt-out |
| Indiana | INCDPA | 2026 | Access, delete, opt-out |
| Tennessee | TIPA | 2025 | Access, delete, opt-out |
| Montana | MCDPA | 2024 | Access, delete, opt-out |
| Oregon | OCPA | 2024 | Access, delete, opt-out, correct |
| Texas | TDPSA | 2024 | Access, delete, opt-out |
| Delaware | DPDPA | 2025 | Access, delete, opt-out |

### International Laws

| Region | Law | Key Differences |
|--------|-----|-----------------|
| EU/EEA | GDPR | Strongest rights, extraterritorial, DPO required |
| UK | UK GDPR | Post-Brexit GDPR variant |
| Canada | PIPEDA | Consent-based, private right of action limited |
| Brazil | LGPD | GDPR-inspired, DPO required |
| Japan | APPI | Consent for sensitive data, cross-border rules |

## Jurisdiction Determination

```
User Location + Broker Location → Applicable Law(s)

Examples:
- CA resident + US broker → CCPA
- CA resident + EU broker → CCPA + GDPR
- TX resident + US broker → TDPSA (limited) + federal law
- EU resident + US broker → GDPR (if broker targets EU)
```

### Implementation Logic

```rust
pub fn determine_jurisdiction(
    user_state: Option<&str>,
    user_country: &str,
    broker_country: &str,
    broker_targets_region: &[&str],
) -> Vec<ApplicableLaw> {
    let mut laws = vec![];

    // US state laws
    if user_country == "US" {
        if let Some(state) = user_state {
            match state {
                "CA" => laws.push(ApplicableLaw::CCPA),
                "VA" => laws.push(ApplicableLaw::VCDPA),
                "CO" => laws.push(ApplicableLaw::CPA),
                // ... etc
            }
        }
    }

    // GDPR
    if user_country_in_eea(user_country)
        || broker_targets_region.contains(&"EU")
    {
        laws.push(ApplicableLaw::GDPR);
    }

    laws
}
```

## Legal Template Components

### Deletion Request Email

```
Subject: {law_name} Data Deletion Request - {full_name}

Dear {broker_name} Privacy Team,

Pursuant to {law_citation}, I am requesting the deletion of my personal
information from your database(s).

{identity_verification_section}

My information that may appear in your records:
- Full Name: {full_name}
- Email: {email}
- Address: {address}
- {additional_identifiers}

I request that you:
1. Delete all personal information you have collected about me
2. Direct any service providers to delete my information
3. Confirm deletion within {response_deadline}

{law_specific_rights_section}

{enforcement_warning}

Sincerely,
{full_name}
{date}
```

### Law-Specific Sections

**CCPA:**
```
Under the California Consumer Privacy Act (Cal. Civ. Code § 1798.100 et seq.),
I have the right to request deletion of my personal information. You must
respond within 45 days. Failure to comply may result in penalties of up to
$7,500 per intentional violation.
```

**GDPR:**
```
Under Article 17 of the General Data Protection Regulation, I have the right
to erasure ("right to be forgotten"). You must respond within 30 days.
I reserve the right to lodge a complaint with the relevant supervisory
authority.
```

**VCDPA:**
```
Under the Virginia Consumer Data Protection Act (Va. Code § 59.1-575 et seq.),
I have the right to delete personal data you have collected. You must respond
within 45 days.
```

## Response Deadlines

| Law | Initial Response | Extension | Total Max |
|-----|------------------|-----------|-----------|
| CCPA | 45 days | +45 days | 90 days |
| GDPR | 30 days | +60 days | 90 days |
| VCDPA | 45 days | +45 days | 90 days |
| CPA | 45 days | +45 days | 90 days |

## Escalation Paths

When a broker doesn't comply:

1. **Follow-up request** (cite deadline, note violation)
2. **Formal demand letter** (legal language, explicit deadline)
3. **Regulatory complaint**:
   - CCPA → California AG or CPPA
   - GDPR → Relevant DPA (ICO for UK, CNIL for France, etc.)
   - VCDPA → Virginia AG
4. **Private right of action** (CCPA data breaches only)

## Verification Requirements

What brokers can legally require:

| Law | Allowed | Not Allowed |
|-----|---------|-------------|
| CCPA | Email verification, account login, signed declaration | SSN, photo ID (usually), payment |
| GDPR | Reasonable verification | Excessive requirements |
| VCDPA | Commercially reasonable verification | Undue burden |

## Invocation Examples

- "What privacy laws apply to a Texas resident requesting removal from Spokeo?"
- "Draft a CCPA deletion request email"
- "What's the response deadline for a GDPR request?"
- "How do I escalate a non-responsive broker under VCDPA?"
- "What verification can a broker legally require under CCPA?"
