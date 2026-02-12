# spectral-browser

Browser automation engine for JavaScript-heavy sites.

## Requirements

- Chrome or Chromium installed on the system
- For tests: `cargo test --lib` (unit tests only, no browser needed)
- For integration tests: `cargo test -- --include-ignored` (requires Chrome)

## Features

- Headless browser automation
- Anti-fingerprinting measures
- Per-domain rate limiting
- Screenshot capture
- Form interaction primitives
