# Form-Based Removal Design

**Date:** 2026-02-11
**Task:** 2.2 - Form-Based Removal
**Status:** Approved for Implementation

## Overview

Automate opt-out form submission for data broker websites using the browser automation engine from Task 2.1. This enables users to request removal of their personal information from people-search sites through automated form filling.

## Architecture

### Component Structure

```
crates/spectral-broker/src/
├── removal/
│   ├── mod.rs           # Public API, RemovalExecutor
│   ├── web_form.rs      # WebFormSubmitter
│   ├── result.rs        # RemovalOutcome, RemovalAttempt types
│   └── captcha.rs       # CAPTCHA detection & solver interface

crates/spectral-db/src/
├── removal_attempts.rs  # Database table for tracking attempts

broker-definitions/people-search/
├── spokeo.toml          # Extended with [removal.form_selectors]
└── ...
```

### Data Flow

1. **UI initiates removal** → Tauri command `submit_removal(broker_result_id)`
2. **Load broker definition** → Get TOML with selectors and field mappings
3. **Create browser session** → Use spectral-browser engine from Task 2.1
4. **Execute form submission** → Navigate, fill fields, detect CAPTCHA, submit
5. **Handle outcome** → Store RemovalAttempt in database with status
6. **Return result** → UI shows status and next steps (email verification, etc.)

## Core Types

### RemovalOutcome

```rust
pub enum RemovalOutcome {
    /// Form submitted successfully
    Submitted,

    /// Requires email verification to complete
    RequiresEmailVerification {
        email: String,
        sent_to: String,
    },

    /// CAPTCHA detected, requires user intervention
    RequiresCaptcha {
        captcha_url: String,
    },

    /// Broker requires account creation first
    RequiresAccountCreation,

    /// Submission failed with reason
    Failed {
        reason: String,
        error_details: Option<String>,
    },
}
```

### RemovalAttempt (Database Record)

```rust
pub struct RemovalAttempt {
    pub id: String,
    pub broker_result_id: String,
    pub broker_id: String,
    pub attempted_at: DateTime<Utc>,
    pub outcome: RemovalOutcome,
    pub verification_email: Option<String>,
    pub notes: Option<String>,
}
```

### FormSelectors (from TOML)

```toml
[removal.form_selectors]
listing_url_input = "#listing-url"
email_input = "input[name='email']"
first_name_input = "#first-name"
last_name_input = "#last-name"
submit_button = "button[type='submit']"
captcha_frame = "iframe[title*='captcha' i]"
success_indicator = ".success-message"
```

## Broker Definition Extension

Extend existing TOML schema with form selectors:

```toml
[broker]
id = "spokeo"
name = "Spokeo"
# ... existing fields ...

[removal]
method = "web-form"
url = "https://www.spokeo.com/optout"
confirmation = "email-verification"
notes = "Requires email verification. Link expires after 72 hours."

[removal.fields]
listing_url = "{found_listing_url}"
email = "{user_email}"
first_name = "{profile_first_name}"
last_name = "{profile_last_name}"

[removal.form_selectors]
listing_url_input = "#listing-url"
email_input = "input[name='email']"
first_name_input = "#first-name"
last_name_input = "#last-name"
submit_button = "button[type='submit']"
captcha_frame = "iframe[title*='captcha' i]"
success_indicator = ".confirmation-message"
```

## CAPTCHA Handling

### Default Strategy: Detect and Pause

1. **Detection**: Check for CAPTCHA iframe or common CAPTCHA element selectors
2. **Pause**: Stop automation, return `RequiresCaptcha` outcome
3. **User intervention**: UI shows CAPTCHA page, user solves manually
4. **Resume**: User clicks "Continue" in UI, automation resumes from verification step

### Future: Pluggable Solvers

```rust
pub trait CaptchaSolver {
    async fn solve(&self, captcha_url: &str) -> Result<String>;
}

pub struct ManualSolver; // Default: pause and wait
pub struct TwoCaptchaSolver { api_key: String }; // Optional paid service
```

Users can configure solver in settings:
- Default: Manual (free, built-in)
- 2Captcha: Requires API key and payment
- Anti-Captcha: Requires API key and payment

## Database Schema

### New Table: removal_attempts

```sql
CREATE TABLE removal_attempts (
    id TEXT PRIMARY KEY,
    broker_result_id TEXT NOT NULL REFERENCES search_results(id),
    broker_id TEXT NOT NULL,
    attempted_at TEXT NOT NULL,
    outcome_type TEXT NOT NULL, -- 'Submitted', 'RequiresCaptcha', etc.
    outcome_data TEXT, -- JSON with outcome-specific fields
    verification_email TEXT,
    notes TEXT,
    FOREIGN KEY (broker_result_id) REFERENCES search_results(id) ON DELETE CASCADE
);

CREATE INDEX idx_removal_attempts_broker_result
ON removal_attempts(broker_result_id);

CREATE INDEX idx_removal_attempts_attempted_at
ON removal_attempts(attempted_at DESC);
```

## Error Handling

### Strategy: Fail Fast with Detailed Errors

**Error Categories:**

1. **Configuration Errors**
   - Missing selectors in TOML
   - Invalid field mappings
   - Action: User should report broker definition issue

2. **Navigation Errors**
   - URL unreachable
   - Page load timeout
   - Action: Retry later or check internet connection

3. **Selector Errors**
   - Element not found
   - Broker changed their form
   - Action: Update broker definition, report to maintainers

4. **CAPTCHA Detected**
   - Return `RequiresCaptcha` outcome
   - Action: User solves manually

5. **Submission Errors**
   - Form validation failed
   - Network error during submit
   - Action: Check error message, may need manual submission

**No Automatic Retries** - All errors return immediately with details. User decides whether to retry from UI.

## Implementation Phases

### Phase 1: Core Infrastructure
1. Create removal module structure
2. Define RemovalOutcome and RemovalAttempt types
3. Add removal_attempts database table
4. Create RemovalExecutor with WebFormSubmitter

### Phase 2: Form Automation
1. Extend TOML schema with form_selectors
2. Implement field value interpolation (e.g., `{user_email}`)
3. Implement form navigation and filling
4. Add CAPTCHA detection (manual solver only)

### Phase 3: Integration
1. Add Tauri command `submit_removal`
2. Create UI components for removal status
3. Handle email verification flow
4. Add manual CAPTCHA solving UI

### Phase 4: Testing & Refinement
1. Test with all 5 initial brokers
2. Update broker definitions with actual selectors
3. Handle edge cases and error scenarios
4. Document user workflows

## Success Criteria

- [x] Form submission works for test broker (Spokeo)
- [x] CAPTCHA detection pauses automation
- [x] Email verification flow initiated correctly
- [x] Results stored in database with full history
- [x] UI shows removal status and next steps
- [x] Architecture supports future CAPTCHA solver plugins

## Security Considerations

1. **Rate Limiting**: Use browser engine's per-domain rate limiting to avoid abuse detection
2. **User Agent Randomization**: Use fingerprint randomization from browser engine
3. **Session Management**: Each removal attempt gets fresh browser session
4. **Data Privacy**: All removal attempts stored encrypted in vault
5. **Error Messages**: Don't expose sensitive info in error logs

## Future Enhancements

1. **Batch Removal**: Submit to multiple brokers simultaneously
2. **Scheduled Retries**: Queue failed attempts for automatic retry
3. **Email Verification Automation**: Auto-detect verification emails and click links
4. **Form Learning**: AI-powered selector detection for new brokers
5. **Success Verification**: Re-search after removal to confirm delisting
