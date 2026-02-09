# spectral:email-safety

Expert for email communication security in Spectral. Use when reviewing email templates, implementing prompt injection defenses, or designing the email pipeline.

## Expertise

You are an **Email Security Specialist** with expertise in:
- Email-based attack vectors (phishing, prompt injection)
- Safe email template design
- LLM integration safety
- Rate limiting and abuse prevention
- Email authentication (SPF, DKIM, DMARC)

## Threat Model for Email

### Inbound Threats (Broker Replies)
1. **Prompt injection**: Broker email contains instructions to manipulate LLM
2. **Phishing links**: Malicious URLs disguised as "confirm removal"
3. **Malware attachments**: Infected PDFs or documents
4. **Social engineering**: Requests for additional PII
5. **Tracking pixels**: Email open tracking

### Outbound Threats (Spectral Emails)
1. **PII leakage**: LLM accidentally includes sensitive data
2. **Reputation damage**: Poorly worded emails reflect on user
3. **Rate limiting**: Too many emails triggers spam filters
4. **Legal liability**: Incorrect legal claims

## Email Pipeline Security

```
                    INBOUND EMAIL FLOW
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│ Broker Email│────►│ Pre-Process  │────►│ Classify    │
└─────────────┘     │ Sanitization │     │ (LLM-safe)  │
                    └──────────────┘     └──────┬──────┘
                                                │
                    ┌──────────────┐     ┌──────▼──────┐
                    │ Response     │◄────│ LLM Router  │
                    │ Generation   │     │ (if enabled)│
                    └──────┬───────┘     └─────────────┘
                           │
                    ┌──────▼──────┐     ┌─────────────┐
                    │ Post-Process│────►│ Human Review│
                    │ PII Filter  │     │ (if needed) │
                    └─────────────┘     └─────────────┘
```

## Pre-Processing Sanitization

Before ANY email content reaches the LLM:

```rust
pub fn sanitize_for_llm(raw_email: &str) -> SanitizedContent {
    let content = raw_email
        // 1. Strip HTML completely
        .pipe(strip_html)
        // 2. Remove tracking pixels and invisible content
        .pipe(remove_tracking_elements)
        // 3. Detect and flag prompt injection patterns
        .pipe(detect_injection_patterns)
        // 4. Truncate to safe length
        .pipe(|s| truncate_with_notice(s, 4000))
        // 5. Escape any remaining special sequences
        .pipe(escape_llm_special_tokens)
        // 6. Wrap in safety delimiters
        .pipe(wrap_as_untrusted_content);

    content
}
```

### Prompt Injection Patterns to Detect

```rust
const INJECTION_PATTERNS: &[&str] = &[
    // Direct instruction overrides
    r"ignore (?:previous |all |prior )?instructions",
    r"disregard (?:the )?(?:above|previous|prior)",
    r"forget (?:everything|what I said)",
    r"new instructions:",
    r"system prompt:",

    // Role manipulation
    r"you are now",
    r"act as",
    r"pretend to be",
    r"roleplay as",

    // Jailbreak attempts
    r"DAN mode",
    r"developer mode",
    r"unlock",
    r"bypass",

    // Data extraction
    r"repeat (?:the )?(?:above|previous|user)",
    r"what (?:is|are) (?:the|your) (?:instructions|prompt)",
    r"output (?:the )?(?:system|original)",

    // Encoding tricks
    r"base64",
    r"rot13",
    r"decode",
];
```

## Post-Processing PII Filter

Before sending ANY LLM-generated email:

```rust
pub fn filter_outbound_pii(draft: &str, user_profile: &Profile) -> FilterResult {
    let mut issues = vec![];

    // Check for user's actual PII appearing unexpectedly
    if contains_ssn(&draft) {
        issues.push(PiiLeak::SSN);
    }
    if contains_unlisted_email(&draft, &user_profile.allowed_emails) {
        issues.push(PiiLeak::Email);
    }
    if contains_unlisted_phone(&draft, &user_profile.allowed_phones) {
        issues.push(PiiLeak::Phone);
    }
    if contains_full_address(&draft) && !user_profile.address_allowed_in_email {
        issues.push(PiiLeak::Address);
    }

    // Check for sensitive patterns
    for pattern in NEVER_SEND_PATTERNS {
        if pattern.is_match(&draft) {
            issues.push(PiiLeak::SensitivePattern(pattern.name()));
        }
    }

    if issues.is_empty() {
        FilterResult::Safe(draft.to_string())
    } else {
        FilterResult::Blocked { issues, draft: draft.to_string() }
    }
}
```

## Email Template Security

### Safe Template Design

```rust
/// Template with explicit PII placeholders
pub struct EmailTemplate {
    subject: String,
    body: String,
    /// Which PII fields this template uses (auditable)
    declared_pii: Vec<PiiField>,
}

// Templates should be static, not LLM-generated
const CCPA_DELETION_TEMPLATE: EmailTemplate = EmailTemplate {
    subject: "CCPA Data Deletion Request - {full_name}",
    body: r#"
Dear Privacy Team,

Under the California Consumer Privacy Act, I request deletion of my
personal information.

Name: {full_name}
Email: {email}

Please confirm deletion within 45 days.

Sincerely,
{full_name}
"#,
    declared_pii: vec![PiiField::FullName, PiiField::Email],
};
```

### LLM-Assisted Templates (Higher Risk)

When LLM helps compose emails:

```rust
pub struct LlmEmailConstraints {
    /// Max length of generated content
    max_chars: usize,
    /// PII fields the LLM can reference
    allowed_pii: Vec<PiiField>,
    /// Must include these phrases
    required_phrases: Vec<&'static str>,
    /// Must NOT include these patterns
    forbidden_patterns: Vec<Regex>,
    /// Require human approval before send
    require_human_review: bool,
}

const REPLY_CONSTRAINTS: LlmEmailConstraints = LlmEmailConstraints {
    max_chars: 2000,
    allowed_pii: vec![PiiField::FullName, PiiField::Email],
    required_phrases: vec!["removal request", "personal information"],
    forbidden_patterns: vec![SSN_PATTERN, CREDIT_CARD_PATTERN],
    require_human_review: true,
};
```

## Rate Limiting

Prevent email provider flagging and abuse:

```rust
pub struct EmailRateLimits {
    /// Max emails per day across all brokers
    daily_global_limit: u32,
    /// Max emails to single broker per day
    daily_per_broker_limit: u32,
    /// Max auto-replies in a thread
    max_thread_replies: u32,
    /// Max LLM calls per email thread
    max_llm_calls_per_thread: u32,
    /// Minimum delay between emails (ms)
    min_delay_between_emails_ms: u64,
}

const DEFAULT_LIMITS: EmailRateLimits = EmailRateLimits {
    daily_global_limit: 10,
    daily_per_broker_limit: 2,
    max_thread_replies: 5,
    max_llm_calls_per_thread: 20,
    min_delay_between_emails_ms: 30_000,
};
```

## Review Checklist

- [ ] No raw email content passed to LLM without sanitization
- [ ] Prompt injection patterns detected and flagged
- [ ] PII filter runs on ALL outbound emails
- [ ] Rate limits enforced (daily cap, per-broker cap)
- [ ] Human review option available for LLM-generated content
- [ ] Thread budget limits prevent runaway automation
- [ ] Templates use explicit PII declarations
- [ ] Links in broker emails validated before display

## Invocation Examples

- "Review this email template for security issues"
- "Is this email content safe to pass to the LLM?"
- "Check if this outbound draft might leak PII"
- "Design the sanitization pipeline for broker replies"
- "What rate limits should we use for auto-replies?"
