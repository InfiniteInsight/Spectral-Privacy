# Spectral

Privacy-focused email automation assistant with configurable LLM integration.

## Table of Contents

- [Overview](#overview)
- [Privacy Configuration](#privacy-configuration)
- [Getting Started](#getting-started)
- [Development](#development)

## Overview

Spectral is a desktop application that automates email workflows while giving you complete control over your data privacy. With configurable privacy levels and LLM provider options, you can choose between local-only processing or cloud-powered assistance.

## Privacy Configuration

Spectral provides multiple privacy levels and LLM provider options to balance convenience with data protection.

### Privacy Levels

Choose a preset privacy level that matches your requirements:

#### Paranoid
- **All features disabled** - No automation, no LLM, manual operation only
- Best for: Highly sensitive environments, maximum control
- Trade-off: No AI assistance available

#### Local Privacy
- **Local LLM only** - Uses Ollama or LM Studio running on your machine
- All automation features enabled (browser, email, scanning)
- Zero data sent to external services
- Best for: Privacy-conscious users with local LLM setup
- Trade-off: Slower responses, requires local compute resources

#### Balanced (Default)
- **Cloud + Local LLM** - Best of both worlds
- PII filtering enabled - Sensitive data tokenized before cloud processing
- All features enabled
- Best for: Most users wanting convenience with privacy protection
- Trade-off: Some data sent to cloud (with PII filtered)

#### Custom
- **Full control** - Configure individual feature flags
- Granular permissions for each capability:
  - Local LLM usage
  - Cloud LLM usage
  - Browser automation
  - Email sending
  - IMAP monitoring
  - PII scanning
- Best for: Advanced users with specific requirements

### LLM Provider Setup

Spectral supports multiple LLM providers. Configure your preferences in Settings > LLM Providers.

#### Local Providers (No API Key Required)

**Ollama**
```bash
# Install Ollama from https://ollama.com
# Pull a model
ollama pull llama3.2

# Ollama runs at http://localhost:11434 by default
```

**LM Studio**
```bash
# Download from https://lmstudio.ai
# Load a model and start the server
# LM Studio runs at http://localhost:1234 by default
```

#### Cloud Providers (API Key Required)

**OpenAI**
1. Get API key from https://platform.openai.com/api-keys
2. Navigate to Settings > LLM Providers
3. Select "OpenAI" and enter your API key
4. Uses GPT-4 for optimal results

**Anthropic Claude**
1. Get API key from https://console.anthropic.com/
2. Navigate to Settings > LLM Providers
3. Select "Claude" and enter your API key
4. Uses Claude 3.5 Sonnet for optimal results

**Google Gemini**
1. Get API key from https://makersuite.google.com/app/apikey
2. Navigate to Settings > LLM Providers
3. Select "Gemini" and enter your API key
4. Uses Gemini Pro for optimal results

### Task-Based Routing

Configure different providers for different tasks:

1. **Primary Provider** - Default for all tasks
2. **Task-Specific Overrides** - Use different providers per task type:
   - Email Drafting
   - Form Filling

Example configuration:
- Primary: Ollama (local, fast, private)
- Email Drafting: Claude (cloud, high quality)
- Form Filling: Ollama (local, sufficient quality)

### PII Filtering

When using cloud providers with Balanced or Custom mode (cloud enabled + PII scanning enabled):

1. **Detection** - Identifies emails, phone numbers, SSNs, credit cards, etc.
2. **Tokenization** - Replaces PII with reversible tokens (e.g., `EMAIL_TOKEN_001`)
3. **Cloud Processing** - Sends tokenized content to cloud LLM
4. **Detokenization** - Restores original PII in the response

This ensures cloud LLMs never see your sensitive data while still providing high-quality assistance.

**Example:**

Input:
```
Draft email to john.doe@company.com about project deadline
```

Sent to cloud (tokenized):
```
Draft email to EMAIL_TOKEN_001 about project deadline
```

Cloud response:
```
Subject: Project Deadline Discussion

Dear EMAIL_TOKEN_001,

I hope this email finds you well...
```

Final output (detokenized):
```
Subject: Project Deadline Discussion

Dear john.doe@company.com,

I hope this email finds you well...
```

## Getting Started

### Prerequisites

- Rust 1.75+ (for building)
- Node.js 20+ (for frontend)
- SQLCipher (for encrypted database)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/spectral.git
cd spectral

# Install dependencies
npm install

# Build and run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

### First Run

1. Application creates encrypted vault on first launch
2. Navigate to Settings > Privacy & Security
3. Choose your privacy level (Balanced recommended)
4. Configure LLM provider(s) in Settings > LLM Providers
5. (Optional) Add API keys for cloud providers
6. Start using email drafting and automation features

## Development

### Project Structure

```
spectral/
├── crates/
│   ├── spectral-privacy/    # Privacy engine and LLM router
│   ├── spectral-llm/         # LLM provider implementations
│   ├── spectral-browser/     # Browser automation
│   └── spectral-db/          # Encrypted database layer
├── src/                      # Svelte frontend
├── src-tauri/                # Tauri backend
└── docs/
    └── testing/              # Manual testing checklists
```

### Running Tests

```bash
# Run all tests
cargo test --all

# Run privacy engine tests
cargo test -p spectral-privacy

# Run integration tests
cargo test -p spectral-privacy --test integration_test
```

### Manual Testing

See [docs/testing/privacy-llm-manual-tests.md](docs/testing/privacy-llm-manual-tests.md) for comprehensive manual testing checklist.

## License

[Your License Here]

## Contributing

[Your Contributing Guidelines Here]
