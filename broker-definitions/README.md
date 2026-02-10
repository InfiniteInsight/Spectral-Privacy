# Broker Definitions

This directory contains TOML definition files for data brokers. Each file describes how to search for listings and request removal from a specific data broker.

## Directory Structure

```
broker-definitions/
├── people-search/       # People search engines
├── background-check/    # Background check services
├── public-records/      # Public records aggregators
├── phone-lookup/        # Phone number lookup services
└── ...                  # Additional categories
```

## TOML Format Specification

Each broker definition file must follow this structure:

### Basic Metadata

```toml
[broker]
id = "broker-id"                    # Lowercase, alphanumeric with hyphens
name = "Broker Name"                # Human-readable name
url = "https://example.com"         # Broker website URL
category = "PeopleSearch"           # Category (see below)
difficulty = "Easy"                 # Removal difficulty: Easy, Medium, Hard
typical_removal_days = 7            # Expected days until removal (1-365)
recheck_interval_days = 30          # Days between re-checks (1-365)
last_verified = "2025-05-01"        # Date this definition was last verified (YYYY-MM-DD)
```

### Categories

- `PeopleSearch` - People search engines (Spokeo, BeenVerified, etc.)
- `BackgroundCheck` - Background check services
- `PublicRecords` - Public records aggregators
- `PhoneLookup` - Phone number lookup services
- `PropertyRecords` - Address/property records
- `SocialMedia` - Social media aggregators
- `Marketing` - Marketing data brokers
- `Financial` - Financial/credit data
- `Healthcare` - Healthcare data
- `Other` - Other/uncategorized

### Search Methods

#### URL Template

Direct URL with variable substitution:

```toml
[search]
method = "UrlTemplate"
template = "https://example.com/{first}-{last}/{state}/{city}"
requires_fields = ["first_name", "last_name", "state", "city"]
```

**Available Variables:**
- `{first}` - First name
- `{last}` - Last name
- `{full_name}` - Full name
- `{email}` - Email address
- `{phone}` - Phone number
- `{address}` - Street address
- `{city}` - City
- `{state}` - State
- `{zip}` - ZIP code

#### Web Form

Form that needs to be filled:

```toml
[search]
method = "WebForm"
url = "https://example.com/search"
requires_fields = ["first_name", "last_name", "state"]

[search.fields]
first_name = "{first}"
last_name = "{last}"
state = "{state}"
```

#### Manual

Requires manual search (no automation):

```toml
[search]
method = "Manual"
url = "https://example.com/search"
instructions = "Navigate to the site and use the search bar. Select your state, then enter your name."
```

### Removal Methods

#### Web Form

Form-based opt-out:

```toml
[removal]
method = "WebForm"
url = "https://example.com/optout"
confirmation = "EmailVerification"  # EmailVerification, Automatic, or Manual
notes = "Additional instructions or warnings"

[removal.fields]
listing_url = "{found_listing_url}"
email = "{user_email}"
first_name = "{first_name}"
last_name = "{last_name}"
```

**Available Variables:**
- `{found_listing_url}` - URL of the found listing
- `{user_email}` - User's email address
- `{user_phone}` - User's phone number
- `{first_name}` - User's first name
- `{last_name}` - User's last name
- `{full_name}` - User's full name
- `{address}` - User's street address
- `{city}` - User's city
- `{state}` - User's state
- `{zip}` - User's ZIP code

#### Email

Email-based removal request:

```toml
[removal]
method = "Email"
email = "privacy@example.com"
subject = "Removal Request - {full_name}"
body = """
I am requesting removal of my data.

Listing URL: {found_listing_url}
Name: {full_name}
Email: {user_email}

Thank you.
"""
response_days = 7                   # Expected response time (1-90)
notes = "Additional instructions"
```

#### Phone

Phone-based removal:

```toml
[removal]
method = "Phone"
phone = "+1-800-123-4567"
instructions = "Call during business hours (9am-5pm EST). Ask for privacy/opt-out department. Provide listing URL and verify identity."
```

#### Manual

Manual process with instructions:

```toml
[removal]
method = "Manual"
instructions = "Must create account, log in, find listing, and click 'Remove' button. May take multiple attempts."
```

## Field Reference

### PII Fields

These are the standardized field names used in `requires_fields`:

- `first_name` - First name
- `last_name` - Last name
- `full_name` - Full legal name
- `middle_name` - Middle name
- `email` - Email address
- `phone` - Phone number
- `address` - Street address
- `city` - City
- `state` - State/province
- `zip_code` - ZIP/postal code
- `country` - Country
- `date_of_birth` - Date of birth
- `age` - Age
- `ssn` - Social Security Number (last 4 digits only)

## Validation Rules

1. **Broker ID**: Must be lowercase alphanumeric with hyphens, 3-50 characters
2. **Required Fields**: `id`, `name`, `url`, `category`, `difficulty`, `typical_removal_days`, `recheck_interval_days`, `last_verified`
3. **Date Format**: `YYYY-MM-DD` for `last_verified`
4. **Days Range**:
   - `typical_removal_days`: 1-365
   - `recheck_interval_days`: 1-365
   - `response_days` (email): 1-90
5. **URLs**: Must be valid HTTPS URLs
6. **Search Method**: At least one field in `requires_fields`
7. **Removal Method**: All required fields must be present

## Contributing

When adding or updating broker definitions:

1. **Test thoroughly** - Verify the search URL works and opt-out process is accurate
2. **Update `last_verified`** - Set to current date when testing
3. **Document quirks** - Add notes about any special requirements or gotchas
4. **Be specific** - Include exact field names, URLs, and instructions
5. **Follow the schema** - Validate against the TOML format above

## Examples

See the files in `people-search/` for complete examples:
- `spokeo.toml` - URL template search with web form removal
- `beenverified.toml` - Web form search and removal
- `whitepages.toml` - URL template with phone verification
- `fastpeoplesearch.toml` - Simple automatic removal
- `truepeoplesearch.toml` - Email-based removal

## License

These broker definitions are community-maintained and provided as-is. Each definition is verified on the date specified in `last_verified`, but sites may change their processes at any time.
