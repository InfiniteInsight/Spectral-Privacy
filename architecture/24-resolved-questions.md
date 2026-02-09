## 24. Resolved Open Questions

The following questions from Section 19 have been resolved. Section 19 is retained for historical reference; this section contains the binding architectural decisions.

### 24.1 Email Sending: Built-in SMTP vs. Generate Drafts

**Decision:** Both, presented during onboarding (Step 5) with three modes:

| Mode | Sending | Inbox Monitoring | Credentials | Best For |
|------|---------|-------------------|-------------|----------|
| **Draft** (default) | User sends via their email client | None — user handles verification | None needed | Privacy-maximizing users |
| **Hybrid** | SMTP (automated sending) | None — user handles verification | SMTP only | Balance of automation + privacy |
| **Full Automation** | SMTP (automated sending) | IMAP (reads verification emails) | SMTP + IMAP | Fully hands-off experience |

Implementation details:
- Draft mode: Spectral generates RFC 5322-compliant `.eml` files and opens them in the user's default email client via `mailto:` URI or direct file open. On macOS, `open -a Mail draft.eml`; on Linux, `xdg-open`; on Windows, `start`.
- SMTP credentials are stored in the vault (encrypted with ChaCha20-Poly1305, key derived from master password via Argon2id).
- Users can change mode at any time via Settings without re-onboarding.
- Automated SMTP mode generates a dedicated opt-out email address alias if the user's provider supports it (e.g., Gmail `+optout` addressing), keeping the user's primary inbox cleaner.

### 24.2 CAPTCHA Handling

**Decision:** Pause and present to user. No automated CAPTCHA solving.

**Where CAPTCHAs are encountered:**

Based on research into major data broker opt-out flows, CAPTCHAs appear in two primary places:

1. **Opt-out form submission:** Most major people-search sites (Spokeo, BeenVerified, Veriforia, Epsilon, and many others) require reCAPTCHA completion when submitting an opt-out request. This is the most common CAPTCHA encounter point.

2. **Search/lookup pages:** Some brokers also place CAPTCHAs on the initial search page used to find the user's profile before opting out.

CAPTCHAs are rarely encountered during verification email link clicks or on email-based opt-out flows.

**How competitors handle it:**

- **Optery:** Explicitly advertises "solves captcha puzzles" as part of its automation. They are a commercial SaaS company that likely uses a combination of CAPTCHA-solving services (e.g., 2Captcha, Anti-Captcha) and AI-based solving. They can absorb the cost and legal risk of this at scale.
- **DeleteMe:** Uses human Privacy Advisors who solve CAPTCHAs manually as part of their quarterly review process. This is feasible because humans are doing the work.

**Why Spectral does NOT auto-solve CAPTCHAs:**

1. **Legal risk:** Automated CAPTCHA circumvention may violate the CFAA (Computer Fraud and Abuse Act) or similar laws, depending on interpretation. An open-source project can't absorb this risk.
2. **Ethical stance:** CAPTCHAs exist to distinguish humans from bots. Spectral is a tool that assists humans — it doesn't pretend to be one.
3. **Third-party dependency:** CAPTCHA-solving services cost money, require API keys, and introduce a third-party data dependency that conflicts with Spectral's privacy-first design.
4. **Arms race:** CAPTCHA systems evolve constantly. Maintaining a solver is an ongoing engineering burden that diverts resources from core functionality.

**Implementation:**

```rust
pub enum CaptchaStrategy {
    /// Pause automation, screenshot the CAPTCHA, notify the user
    PauseAndPresent,
}

pub struct CaptchaEncounter {
    pub broker_id: BrokerId,
    pub page_url: String,
    pub captcha_type: CaptchaType,  // reCAPTCHA v2, v3, hCaptcha, custom
    pub screenshot: PathBuf,
    pub encountered_at: DateTime<Utc>,
    pub automation_state: AutomationState,  // saved state to resume from
}

// UX flow:
// 1. Browser automation hits CAPTCHA
// 2. Save automation state (which fields are filled, which step we're on)
// 3. Take screenshot of the page
// 4. Show desktop notification:
//    "Spokeo requires human verification — click to complete"
// 5. Open the embedded browser view focused on the CAPTCHA element
// 6. User solves CAPTCHA in the browser view
// 7. Spectral detects CAPTCHA completion (page navigation or form state change)
// 8. Resume automation from saved state
```

**Broker definitions note which brokers require CAPTCHAs** (the `requires_captcha: bool` field in `BrokerDefinition`), so the scheduler can batch CAPTCHA-requiring brokers together and present them to the user in a single "solve these 5 CAPTCHAs" session rather than interrupting throughout the day.

### 24.3 Verification Email Handling

**Decision:** User choice, presented as part of email mode selection.

**Three approaches, matching the email mode from onboarding:**

| Email Mode | Verification Handling |
|---|---|
| **Draft** | User handles everything. Spectral shows notification: "Check your inbox for a verification email from Spokeo. Click the confirmation link within 72 hours." |
| **Hybrid** | Same as Draft — user handles verification emails manually since IMAP is not configured. |
| **Full Automation** | Spectral monitors inbox (IMAP) for verification emails from active broker domains and auto-clicks confirmation links. |

**Full Automation verification email handling rules:**

```rust
pub struct VerificationEmailMatcher {
    /// Only match emails from domains where we have an active removal in progress
    pub active_removal_domains: HashSet<String>,

    /// Match rules for verification emails
    pub rules: Vec<VerificationRule>,
}

pub struct VerificationRule {
    /// Sender domain must match (wildcard: *@spokeo.com)
    pub sender_domain: String,

    /// Subject or body must contain verification-related keywords
    pub content_keywords: Vec<String>,
    // "verify", "confirm", "opt-out", "removal", "click here to confirm"

    /// Link URL must match the broker's domain
    pub link_domain_must_match: bool,  // always true

    /// Link URL or anchor text should contain verification keywords
    pub link_keywords: Vec<String>,
    // "verify", "confirm", "opt-out", "complete", "finalize"
}

/// Safety checks before auto-clicking any link:
pub fn should_auto_click(email: &Email, link: &Url, matcher: &VerificationEmailMatcher) -> bool {
    // 1. We must have an active removal for this domain
    let sender_domain = email.sender_domain();
    if !matcher.active_removal_domains.contains(&sender_domain) {
        return false;  // We didn't initiate anything with this company
    }

    // 2. The link domain must match the sender domain
    //    (prevents phishing: email from @spokeo.com with link to evil.com)
    if link.domain() != Some(&sender_domain)
        && !is_known_broker_redirect_domain(link.domain()) {
        return false;
    }

    // 3. The email must contain verification-related keywords
    let has_verification_keywords = matcher.rules.iter().any(|rule| {
        rule.sender_domain == sender_domain
            && rule.content_keywords.iter().any(|kw| {
                email.subject_contains(kw) || email.body_contains(kw)
            })
    });
    if !has_verification_keywords {
        return false;
    }

    // 4. The link itself should contain verification-related keywords
    let link_text = link.as_str().to_lowercase();
    let has_link_keyword = ["verify", "confirm", "opt", "complete", "finalize", "activate"]
        .iter()
        .any(|kw| link_text.contains(kw));

    has_link_keyword
}
```

**Key safety principle:** Spectral only auto-clicks links in emails from domains where it has an active, in-progress removal. If Spectral kicked off an opt-out from Spokeo, it monitors for emails from `*@spokeo.com` and clicks links that point back to `spokeo.com` containing verification keywords. It never clicks links in unsolicited emails or emails from unknown senders.

### 24.4 Legal Compliance / ToS Implications

**Decision:** Resolved — no further input needed.

Summary of architectural decisions:
- Include a clear disclaimer: "Spectral generates removal requests based on your legal rights under applicable privacy laws. These are not legal advice. Consult an attorney for specific legal questions."
- Automated submissions do NOT disclose they are tool-assisted. Requests are submitted on behalf of the user, at the user's direction.
- The disclaimer is shown during onboarding and accessible from Settings > Legal.
- Spectral is a tool, not a law firm. It never claims to provide legal advice.

### 24.5 Telemetry-Free Analytics

**Decision:** Resolved — no further input needed.

Summary:
- No telemetry, no phoning home, ever.
- "Report broken broker" button in the app generates a pre-filled GitHub issue template (stripped of PII — broker ID, error type, timestamp only).
- CI pipeline tests broker definitions against live sites automatically.
- Community issue tracker provides trending data on broker changes.

### 24.6 Project Name

**Decision:** Primary name is **Spectral**. Alternative considered: **Privacy Shroud**.

Analysis of "Spectral":
- Memorable, strong metaphor (invisible, like a specter watching over your data)
- Not taken by any major software product in the privacy space
- Works well as a CLI command (`spectral scan`, `spectral status`)
- Domain options: `spectral.dev`, `getspectral.app`, `spectral-privacy.org`

Analysis of "Privacy Shroud":
- Descriptive — immediately communicates purpose
- No software product conflicts found (only physical products: a door security panel and a cash register privacy screen)
- No USPTO trademark conflicts found in preliminary search for software/SaaS categories
- More approachable for non-technical users
- Slightly harder as a CLI command (`privacyshroud scan` is verbose; could alias as `shroud`)
- Domain options: `privacyshroud.com`, `privacyshroud.org`, `privacyshroud.app`

**Recommendation:** Keep "Spectral" as the project/code name. "Privacy Shroud" could serve as a user-facing brand name or tagline if desired. A formal USPTO trademark search should be conducted before launch for whichever name is chosen. The architecture document continues to use "Spectral" as the working name.

### 24.7 Network Monitoring Permissions / Elevated Privileges

**Decision:** Resolved — graceful degradation.

Summary:
- Spectral never requires elevated privileges
- Each network collector works with whatever access is available
- If a collector can't access a data source (e.g., DNS cache requires elevation), it logs a warning, skips it, and explains to the user what it can't see and why
- The privacy score uses whatever data sources are available
- On macOS, TCC prompts are surfaced naturally through the permission system

### 24.8 Domain Intelligence False Positives

**Decision:** Resolved — local whitelist + community dispute resolution.

Summary:
- Users can whitelist domains locally (user_override layer takes precedence)
- Community disputes handled via GitHub PRs with evidence
- "Report miscategorization" button generates pre-filled PR or issue
- Domain intelligence DB includes a `confidence` field; low-confidence entries are flagged

### 24.9 Auto-Reply Global Daily Cap

**Decision:** Global daily cap IS implemented. Here's why it matters:

**The problem without a global cap:**

Per-thread caps (5 replies max per thread, from Section 7) prevent runaway conversations with a single broker. But consider this scenario:

- The user has 30 active removal threads across different brokers
- All 30 brokers reply on the same day (e.g., after a batch of removal requests)
- Without a global cap, Spectral would auto-send up to 30 replies in a single day
- From the user's email account, this looks like a spam burst
- Email providers (Gmail, Outlook) may flag the account for unusual activity
- The user's email could be temporarily rate-limited or suspended
- Some brokers may flag coordinated-looking automated responses

**The solution:**

```rust
pub struct AutoReplyLimits {
    /// Max auto-replies per individual thread (existing)
    pub per_thread_max: u32,        // default: 5

    /// Max auto-replies across ALL threads per day (new)
    pub global_daily_max: u32,      // default: 10

    /// Max auto-replies across ALL threads per hour (new)
    pub global_hourly_max: u32,     // default: 3

    /// When daily cap is hit, remaining replies are queued for next day
    pub queue_overflow: bool,       // default: true
}

// Permission preset defaults:
// Paranoid:       global_daily_max = 0  (no auto-replies ever)
// Local Privacy:  global_daily_max = 5
// Balanced:       global_daily_max = 10
// Custom:         user-configurable, max 25
```

When the daily cap is reached, remaining replies are queued and sent the next day. The user is notified: "Daily auto-reply limit reached (10/10). 4 replies queued for tomorrow."

### 24.10 Legal Escalation Disclaimer & User Guidance

**Decision:** Disclaimer required. User can choose self-service or app-assisted escalation.

**Implementation:**

```
┌──────────────────────────────────────────────────────────────────┐
│  Escalation Options for: Spokeo (45 days overdue)                │
│                                                                   │
│  Spokeo has not responded to your deletion request within the     │
│  timeframe required by CCPA/CPRA (45 days + 45-day extension).   │
│                                                                   │
│  ⚠ DISCLAIMER: Spectral is not a law firm. The following         │
│  options are informational and do not constitute legal advice.    │
│  Consider consulting an attorney for complex situations.         │
│                                                                   │
│  What would you like to do?                                       │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐    │
│  │ 📧 Re-send deletion request (with legal deadline cited)  │    │
│  │    Spectral will send a follow-up email citing the CCPA  │    │
│  │    deadline and requesting immediate compliance.          │    │
│  │    [Send follow-up]                                       │    │
│  └──────────────────────────────────────────────────────────┘    │
│  ┌──────────────────────────────────────────────────────────┐    │
│  │ 📋 File regulatory complaint (app-assisted)              │    │
│  │    Spectral will generate a complaint for the California  │    │
│  │    Privacy Protection Agency (CPPA) and submit it on      │    │
│  │    your behalf.                                           │    │
│  │    ⚠ This is a formal government complaint.              │    │
│  │    [Generate & review before sending]                     │    │
│  └──────────────────────────────────────────────────────────┘    │
│  ┌──────────────────────────────────────────────────────────┐    │
│  │ 📖 Do it yourself (guided instructions)                  │    │
│  │    Spectral will show you step-by-step instructions       │    │
│  │    for filing a complaint with CPPA yourself.             │    │
│  │    Includes: complaint URL, what to include, tips.        │    │
│  │    [Show me how]                                          │    │
│  └──────────────────────────────────────────────────────────┘    │
│  ┌──────────────────────────────────────────────────────────┐    │
│  │ ⏸ Wait longer                                            │    │
│  │    Some brokers are slow but eventually comply. Spectral  │    │
│  │    will continue monitoring and remind you in 14 days.    │    │
│  │    [Remind me later]                                      │    │
│  └──────────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────────┘
```

**"Do it yourself" guided instructions include:**

1. Direct link to the regulatory body's complaint form
2. What information to include (broker name, dates of requests, copies of emails)
3. Which regulation to cite and the specific section violated
4. Expected timeline for regulatory response
5. What to expect after filing (investigation process, possible outcomes)
6. A pre-filled template the user can copy/paste into the complaint form

**Safety guardrails for app-assisted complaints:**
- Regulatory complaints (Tier 3 escalation) ALWAYS require explicit user confirmation
- The generated complaint is shown to the user for review before submission
- A "complexity circuit breaker" flags situations where the user should consult a lawyer (e.g., broker claims exemption, counter-arguments about legitimate business purpose, cross-border jurisdiction disputes)
- The complaint explicitly states it is filed by the individual, not by Spectral
