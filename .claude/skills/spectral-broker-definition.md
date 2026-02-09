# spectral:broker-definition

Expert for creating and validating Spectral broker definition TOML files. Use when adding new brokers, updating existing definitions, or validating broker configurations.

## Expertise

You are a **Broker Integration Specialist** with expertise in:
- Web scraping and form automation patterns
- Data broker opt-out workflows
- TOML configuration syntax
- CSS selectors and XPath
- Privacy law requirements by jurisdiction

## Broker Definition Schema

```toml
# /broker-definitions/<category>/<broker-id>.toml

[broker]
id = "broker-id"                    # lowercase, hyphens only
name = "Human Readable Name"
domain = "example.com"              # primary domain, no https://
category = "people-search"          # see valid categories
difficulty = "easy"                 # easy, medium, hard
typical_removal_days = 7            # expected removal timeline
recheck_interval_days = 30          # when to re-verify removal
last_verified = "2025-01-15"        # when definition was tested
notes = "Additional context"

[search]
method = "url-template"             # see search methods
# ... method-specific fields

[removal]
method = "web-form"                 # see removal methods
# ... method-specific fields

[[jurisdictions]]
law = "ccpa"
applies = true
template = "ccpa-standard"

[[jurisdictions]]
law = "gdpr"
applies = false
notes = "US-only broker"
```

## Valid Categories

| Category | Description |
|----------|-------------|
| `people-search` | Spokeo, BeenVerified, WhitePages |
| `data-aggregator` | Acxiom, Oracle Data Cloud |
| `background-check` | Checkr, employment screening |
| `marketing-list` | Direct mail, advertising data |
| `public-records` | Court records, property records |
| `social-media` | Profile scraping sites |
| `financial` | Credit-adjacent, alternative data |
| `other` | Doesn't fit above categories |

## Search Methods

### url-template
```toml
[search]
method = "url-template"
template = "https://www.spokeo.com/{first}-{last}/{state}/{city}"
requires_fields = ["first_name", "last_name", "state", "city"]
```

### form-search
```toml
[search]
method = "form-search"
url = "https://example.com/search"
form_selector = "form#search-form"
submit_button = "button[type=submit]"

[search.fields]
first_name = "input[name=firstName]"
last_name = "input[name=lastName]"
state = "select[name=state]"
```

### api-search
```toml
[search]
method = "api-search"
endpoint = "https://api.example.com/v1/search"
http_method = "POST"
content_type = "application/json"
payload_template = '{"name": "{first_name} {last_name}", "location": "{city}, {state}"}'
```

### llm-guided
```toml
[search]
method = "llm-guided"
start_url = "https://example.com"
instructions = """
1. Look for a search box on the homepage
2. Enter the person's full name
3. Click search and wait for results
4. Identify if any listings match the user's details
"""
```

## Removal Methods

### web-form
```toml
[removal]
method = "web-form"
url = "https://example.com/optout"
form_selector = "form#optout-form"
confirmation = "email-verification"  # none, email-verification, account-required, photo-id, mail

[removal.fields]
listing_url = "input[name=listingUrl]"
email = "input[name=email]"
reason = { selector = "select[name=reason]", value = "privacy" }
```

### email
```toml
[removal]
method = "email"
to = "privacy@example.com"
subject_template = "Data Removal Request - {full_name}"
body_template = "ccpa-email-standard"  # reference to template
requires_fields = ["full_name", "email", "address"]
```

### multi-step
```toml
[removal]
method = "multi-step"

[[removal.steps]]
type = "web-form"
url = "https://example.com/optout/step1"
form_selector = "form#step1"
[removal.steps.fields]
email = "input[name=email]"

[[removal.steps]]
type = "email-confirmation"
wait_for = "Click here to confirm"
timeout_hours = 72

[[removal.steps]]
type = "web-form"
url = "{confirmation_link}"
form_selector = "form#confirm"
```

### manual
```toml
[removal]
method = "manual"
instructions = """
1. Call 1-800-XXX-XXXX
2. Request to speak with privacy department
3. Provide your full name and address
4. Request removal under CCPA
"""
estimated_time_minutes = 15
```

## Validation Rules

The broker definition validator (`scripts/validate-broker-toml.py`) checks:

1. **Required fields present**: id, name, domain, category, search.method, removal.method
2. **Valid category**: Must be in allowed list
3. **Valid domain format**: No protocol, no trailing slash
4. **ID format**: lowercase alphanumeric with hyphens
5. **Removal method valid**: Must be in allowed list
6. **Jurisdiction laws valid**: ccpa, gdpr, vcdpa, etc.

## Example Complete Definition

```toml
# /broker-definitions/people-search/spokeo.toml

[broker]
id = "spokeo"
name = "Spokeo"
domain = "spokeo.com"
category = "people-search"
difficulty = "easy"
typical_removal_days = 3
recheck_interval_days = 30
last_verified = "2025-01-15"
notes = "One of the largest people search sites. Removal requires email verification."

[search]
method = "url-template"
template = "https://www.spokeo.com/{first}-{last}/{state}/{city}"
requires_fields = ["first_name", "last_name", "state", "city"]
result_selector = "div.search-result"
no_results_indicator = "No results found"

[removal]
method = "web-form"
url = "https://www.spokeo.com/optout"
confirmation = "email-verification"
confirmation_timeout_hours = 72

[removal.fields]
listing_url = { selector = "input[name=url]", value = "{found_listing_url}" }
email = { selector = "input[name=email]", value = "{user_email}" }

[removal.captcha]
type = "recaptcha-v2"
handling = "user-solve"  # never automated

[[jurisdictions]]
law = "ccpa"
applies = true
template = "ccpa-standard"

[[jurisdictions]]
law = "gdpr"
applies = false
notes = "US-focused broker"
```

## Invocation Examples

- "Create a broker definition for FastPeopleSearch"
- "Validate this broker TOML file"
- "Update the Spokeo definition with new selectors"
- "What removal method should I use for a broker that requires phone calls?"
