## 23. Commercial Relationship Engine (Non-Data-Broker Deletion)

This is a key differentiator from services like DeleteMe and Optery, which **only** handle data broker / people-search sites. Spectral can also help users request data deletion from any company they've done business with — retailers, subscription services, SaaS platforms, airlines, hotels, etc.

### 23.1 Problem Statement

Data brokers are only part of the picture. Every online purchase, subscription, or account creation leaves PII with companies the user has directly interacted with. These companies have:

- Full name, email, phone, shipping address (from orders)
- Payment method metadata (last 4 digits, billing address)
- Purchase history, browsing behavior, preferences
- Account credentials, profile data, saved items

Under GDPR, CCPA, and similar laws, users have the right to request deletion from **any** company that holds their data — not just data brokers.

### 23.2 Discovery via Email Scanning

When the user grants email scanning permission, Spectral's discovery engine identifies commercial relationships by scanning for transactional email patterns:

```rust
// /crates/spectral-discovery/src/email/commercial.rs

/// Patterns that indicate a commercial relationship with a company
pub struct CommercialEmailPatterns {
    pub order_confirmation: Vec<Pattern>,
    pub shipping_confirmation: Vec<Pattern>,
    pub delivery_notification: Vec<Pattern>,
    pub invoice: Vec<Pattern>,
    pub receipt: Vec<Pattern>,
    pub subscription_confirmation: Vec<Pattern>,
    pub account_creation: Vec<Pattern>,
    pub password_reset: Vec<Pattern>,
    pub marketing_newsletter: Vec<Pattern>,
    pub loyalty_program: Vec<Pattern>,
}

/// Default detection patterns
impl Default for CommercialEmailPatterns {
    fn default() -> Self {
        Self {
            order_confirmation: vec![
                Pattern::subject_contains(&[
                    "order confirmation",
                    "order received",
                    "order #",
                    "your order",
                    "purchase confirmation",
                    "thank you for your order",
                    "we've received your order",
                    "order placed",
                ]),
                Pattern::body_contains(&[
                    "order number",
                    "order total",
                    "items ordered",
                    "order summary",
                    "estimated delivery",
                ]),
            ],
            shipping_confirmation: vec![
                Pattern::subject_contains(&[
                    "shipped",
                    "shipping confirmation",
                    "your package",
                    "tracking number",
                    "on its way",
                    "out for delivery",
                    "has shipped",
                ]),
            ],
            delivery_notification: vec![
                Pattern::subject_contains(&[
                    "delivered",
                    "delivery confirmation",
                    "your package was delivered",
                    "package arrived",
                ]),
            ],
            invoice: vec![
                Pattern::subject_contains(&[
                    "invoice",
                    "billing statement",
                    "payment received",
                    "payment confirmation",
                    "your bill",
                ]),
            ],
            receipt: vec![
                Pattern::subject_contains(&[
                    "receipt",
                    "purchase receipt",
                    "transaction receipt",
                    "payment receipt",
                    "e-receipt",
                ]),
                Pattern::attachment_name(&[
                    "receipt.pdf",
                    "invoice.pdf",
                ]),
            ],
            subscription_confirmation: vec![
                Pattern::subject_contains(&[
                    "subscription confirmed",
                    "welcome to",
                    "subscription active",
                    "membership confirmed",
                    "you're subscribed",
                    "trial started",
                ]),
            ],
            account_creation: vec![
                Pattern::subject_contains(&[
                    "welcome to",
                    "account created",
                    "verify your email",
                    "confirm your email",
                    "activate your account",
                    "registration complete",
                ]),
            ],
            password_reset: vec![
                Pattern::subject_contains(&[
                    "password reset",
                    "reset your password",
                    "forgot password",
                    "change your password",
                ]),
            ],
            marketing_newsletter: vec![
                Pattern::has_unsubscribe_header(),
                Pattern::has_list_unsubscribe_header(),
            ],
            loyalty_program: vec![
                Pattern::subject_contains(&[
                    "rewards",
                    "loyalty",
                    "points balance",
                    "member status",
                    "earned points",
                ]),
            ],
        }
    }
}
```

### 23.3 Commercial Relationship Model

```rust
// /crates/spectral-discovery/src/commercial.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommercialRelationship {
    pub id: Uuid,
    pub company_name: String,
    pub company_domain: String,
    pub discovered_via: DiscoverySource,
    pub discovered_at: DateTime<Utc>,

    /// What kind of relationship this appears to be
    pub relationship_type: RelationshipType,

    /// Evidence collected (email subjects, dates — NOT email bodies)
    pub evidence: Vec<RelationshipEvidence>,

    /// Estimated PII the company likely holds based on relationship type
    pub estimated_pii_held: Vec<EstimatedPiiCategory>,

    /// Known privacy/deletion page for this company (if in our database)
    pub known_deletion_url: Option<String>,
    pub known_privacy_email: Option<String>,

    /// User's action status
    pub status: RelationshipStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    /// Active customer — recent orders/activity
    ActiveCustomer,
    /// Inactive customer — no activity in 12+ months
    InactiveCustomer,
    /// Active subscription
    ActiveSubscription,
    /// Cancelled/expired subscription
    InactiveSubscription,
    /// Account exists but no purchases (free tier, browsing account)
    AccountOnly,
    /// Newsletter/marketing recipient only
    MarketingOnly,
    /// Loyalty/rewards program member
    LoyaltyProgram,
    /// One-time interaction (single purchase, donation, etc.)
    OneTime,
    /// Unknown — detected a relationship but can't classify further
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipEvidence {
    pub evidence_type: EvidenceType,
    pub date: DateTime<Utc>,
    pub sender_email: String,
    pub subject_snippet: String,  // first 80 chars of subject, no body content
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvidenceType {
    OrderConfirmation,
    ShippingNotification,
    DeliveryConfirmation,
    Invoice,
    Receipt,
    SubscriptionNotice,
    AccountCreation,
    PasswordReset,
    MarketingEmail,
    LoyaltyUpdate,
}

/// What PII the company likely holds, inferred from relationship type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EstimatedPiiCategory {
    FullName,
    EmailAddress,
    PhoneNumber,
    ShippingAddress,
    BillingAddress,
    PaymentMethodPartial,   // last 4 digits, expiry
    PurchaseHistory,
    BrowsingHistory,
    AccountCredentials,
    Preferences,
    IpAddressLogs,
    DeviceFingerprint,
}

impl RelationshipType {
    /// What PII this type of relationship typically involves
    pub fn typical_pii(&self) -> Vec<EstimatedPiiCategory> {
        match self {
            Self::ActiveCustomer | Self::InactiveCustomer => vec![
                EstimatedPiiCategory::FullName,
                EstimatedPiiCategory::EmailAddress,
                EstimatedPiiCategory::PhoneNumber,
                EstimatedPiiCategory::ShippingAddress,
                EstimatedPiiCategory::BillingAddress,
                EstimatedPiiCategory::PaymentMethodPartial,
                EstimatedPiiCategory::PurchaseHistory,
                EstimatedPiiCategory::AccountCredentials,
                EstimatedPiiCategory::IpAddressLogs,
            ],
            Self::ActiveSubscription | Self::InactiveSubscription => vec![
                EstimatedPiiCategory::FullName,
                EstimatedPiiCategory::EmailAddress,
                EstimatedPiiCategory::BillingAddress,
                EstimatedPiiCategory::PaymentMethodPartial,
                EstimatedPiiCategory::AccountCredentials,
                EstimatedPiiCategory::Preferences,
            ],
            Self::MarketingOnly => vec![
                EstimatedPiiCategory::EmailAddress,
                EstimatedPiiCategory::Preferences,
            ],
            Self::AccountOnly => vec![
                EstimatedPiiCategory::FullName,
                EstimatedPiiCategory::EmailAddress,
                EstimatedPiiCategory::AccountCredentials,
                EstimatedPiiCategory::IpAddressLogs,
            ],
            _ => vec![
                EstimatedPiiCategory::EmailAddress,
                EstimatedPiiCategory::FullName,
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipStatus {
    /// Discovered, not yet shown to user
    Discovered,
    /// Shown to user, awaiting their decision
    PendingUserReview,
    /// User wants to request deletion
    DeletionRequested,
    /// Deletion request sent to company
    DeletionSent { sent_at: DateTime<Utc> },
    /// Company confirmed deletion
    DeletionConfirmed { confirmed_at: DateTime<Utc> },
    /// User wants to keep this relationship (don't delete)
    UserKept,
    /// User dismissed (don't show again)
    Dismissed,
}
```

### 23.4 Commercial Relationship Dashboard View

```
┌──────────────────────────────────────────────────────────────────┐
│  Commercial Relationships (discovered via email scan)            │
│                                                                   │
│  Spectral found 47 companies that likely have your data.         │
│  Review each and decide what to do.                              │
│                                                                   │
│  Filter: [All ▼] [Active ▼] [Inactive ▼] [Marketing ▼]         │
│  Sort:   [Most recent ▼]                                        │
│                                                                   │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ 🛒 Amazon.com                          Active Customer     │  │
│  │    Last activity: Jan 2026 (order confirmation)            │  │
│  │    Estimated PII: name, email, phone, 3 addresses,        │  │
│  │                   payment info, purchase history           │  │
│  │    [Request Deletion] [Keep] [Dismiss]                     │  │
│  │    ℹ Deletion will require closing your Amazon account     │  │
│  └────────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ 📦 Wayfair                             Inactive Customer   │  │
│  │    Last activity: Mar 2024 (shipping notification)         │  │
│  │    Estimated PII: name, email, address, payment info,      │  │
│  │                   purchase history                         │  │
│  │    [Request Deletion] [Keep] [Dismiss]                     │  │
│  │    🔗 Known deletion page: wayfair.com/privacy             │  │
│  └────────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ 📧 Pottery Barn                        Marketing Only      │  │
│  │    Last activity: Dec 2025 (marketing email)               │  │
│  │    Estimated PII: email address, preferences               │  │
│  │    [Unsubscribe + Delete] [Keep] [Dismiss]                 │  │
│  └────────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ 💳 Netflix                             Inactive Sub         │  │
│  │    Last activity: Aug 2024 (billing statement)             │  │
│  │    Estimated PII: name, email, billing address,            │  │
│  │                   payment info, viewing history            │  │
│  │    [Request Deletion] [Keep] [Dismiss]                     │  │
│  │    🔗 Known: netflix.com/account/delete                    │  │
│  └────────────────────────────────────────────────────────────┘  │
│                                                                   │
│  [Select All Inactive → Request Deletion]                        │
│  [Select All Marketing → Unsubscribe + Delete]                   │
└──────────────────────────────────────────────────────────────────┘
```

### 23.5 Deletion Request Generation for Commercial Entities

For companies in the commercial relationship database, Spectral generates deletion requests using templates adapted to the relationship type and applicable jurisdiction:

```rust
pub fn generate_commercial_deletion_request(
    relationship: &CommercialRelationship,
    profile: &UserProfile,
    jurisdiction: &UserJurisdiction,
) -> DeletionRequest {
    let regulation = jurisdiction.strongest_regulation.clone();
    let citation = regulation.citation_text();

    let template = match relationship.relationship_type {
        RelationshipType::ActiveCustomer |
        RelationshipType::InactiveCustomer => {
            // Warn user: deleting from active retailer means losing
            // order history, saved addresses, etc.
            Template::CustomerDeletion {
                company: relationship.company_name.clone(),
                legal_citation: citation,
                estimated_data: relationship.estimated_pii_held.clone(),
            }
        },
        RelationshipType::MarketingOnly => {
            // Simpler: unsubscribe + delete email from their list
            Template::MarketingDeletion {
                company: relationship.company_name.clone(),
                legal_citation: citation,
            }
        },
        RelationshipType::InactiveSubscription => {
            // Account exists but sub is cancelled — full deletion
            Template::AccountDeletion {
                company: relationship.company_name.clone(),
                legal_citation: citation,
                estimated_data: relationship.estimated_pii_held.clone(),
            }
        },
        _ => Template::GenericDeletion {
            company: relationship.company_name.clone(),
            legal_citation: citation,
        },
    };

    // Route to known deletion mechanism if available
    let delivery = if let Some(url) = &relationship.known_deletion_url {
        DeliveryMethod::DirectLink(url.clone())
    } else if let Some(email) = &relationship.known_privacy_email {
        DeliveryMethod::Email(email.clone())
    } else {
        // Attempt to find privacy@ or dpo@ for the domain
        DeliveryMethod::InferredEmail(format!(
            "privacy@{}", relationship.company_domain
        ))
    };

    DeletionRequest { template, delivery, relationship_id: relationship.id }
}
```

### 23.6 Known Company Privacy Database

Spectral ships with a community-maintained database of known privacy/deletion endpoints for popular companies:

```toml
# /company-definitions/amazon.toml
[company]
name = "Amazon"
domain = "amazon.com"
alternate_domains = ["amazon.co.uk", "amazon.de", "amazon.fr", "amazon.ca"]

[company.deletion]
method = "web_portal"
url = "https://www.amazon.com/privacy/data-deletion"
privacy_email = "privacy@amazon.com"
notes = "Account deletion required for full data removal. Partial deletion available via privacy portal."

[company.detection]
sender_domains = ["amazon.com", "amazon.co.uk"]
sender_addresses = ["shipment-tracking@amazon.com", "order-update@amazon.com",
                     "auto-confirm@amazon.com", "no-reply@amazon.com"]
```

### 23.7 Relationship to Existing Engines

The Commercial Relationship Engine feeds into existing systems:

- **Verification Engine (Section 6):** Track whether the company responded within legal deadlines
- **Mail Engine (Section 7):** Send/draft deletion request emails using the same safety pipeline
- **Dashboard (Section 10):** Commercial relationships shown as a separate category alongside data brokers
- **Jurisdiction System (Section 21):** Legal citations adapted to user's location
- **Permission System (Section 8):** Requires `EmailImapRead` permission for email scanning discovery

---
