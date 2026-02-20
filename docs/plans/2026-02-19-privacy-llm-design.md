# Privacy Level & LLM Integration Design

**Date**: 2026-02-19
**Status**: Approved
**Architecture**: Privacy Engine Orchestrator (Approach 2)

---

## Goal

Implement comprehensive privacy controls and LLM integration for Spectral, allowing users to:
- Choose privacy levels (Paranoid, Local Privacy, Balanced, Custom)
- Configure multiple LLM providers (Ollama, LM Studio, OpenAI, Claude, Gemini)
- Control which features can access which services based on privacy preferences
- Get LLM assistance for email drafting and form filling with automatic PII protection

## Architecture Overview

### New Crate: `spectral-privacy`

A centralized privacy engine that manages all privacy decisions and settings.

**Core responsibilities**:
- Store and retrieve privacy settings from vault database
- Evaluate permission requests: "Can I use cloud LLM for this task?"
- Manage privacy level presets and custom configurations
- Provide single enforcement point for all privacy rules

### System Architecture

```
┌─────────────────────────────────────────────────┐
│           Tauri Commands (UI Layer)             │
└─────────────────┬───────────────────────────────┘
                  │
         ┌────────▼─────────┐
         │  PrivacyEngine   │  ← New central authority
         │  (spectral-privacy)│
         └────────┬─────────┘
                  │
    ┌─────────────┼─────────────┐
    │             │             │
┌───▼────┐  ┌────▼────┐  ┌────▼─────┐
│spectral│  │spectral │  │spectral  │
│-llm    │  │-browser │  │-mail     │
└────────┘  └─────────┘  └──────────┘
```

**Key flow**:
1. UI initiates action (e.g., "draft email with LLM")
2. Service asks `PrivacyEngine`: `can_use_llm(TaskType::EmailDraft)`
3. Engine checks settings, returns `PermissionResult`
4. Service proceeds or shows user-friendly error

---

## Database Schema

New `settings` table in vault SQLite database:

```sql
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,  -- JSON
    updated_at TEXT NOT NULL
);
```

**Stored settings**:
- `privacy_level`: `{"level": "Balanced"}`
- `feature_flags`: `{"allow_local_llm": true, "allow_cloud_llm": true, ...}`
- `llm_providers`: `[{provider_id, encrypted_api_key, model, enabled, is_primary}]`
- `llm_routing`: `{task_preferences: {...}, fallback_enabled: true}`

**Encryption**: API keys encrypted with vault password before storage in JSON.

**Storage strategy**:
- **Primary**: Vault database (works everywhere)
- **Optional**: System keychain fallback on macOS/Windows when available
- **Linux**: Runtime detection of Secret Service API, graceful fallback to vault DB

---

## Component Design

### PrivacyEngine Core API

```rust
pub struct PrivacyEngine {
    pool: SqlitePool,
    vault_id: String,
}

impl PrivacyEngine {
    // Permission checking
    pub async fn can_use_feature(&self, feature: Feature) -> PermissionResult;
    pub async fn can_use_llm(&self, task: TaskType) -> LlmPermission;

    // Settings management
    pub async fn get_privacy_level(&self) -> Result<PrivacyLevel>;
    pub async fn set_privacy_level(&self, level: PrivacyLevel) -> Result<()>;
    pub async fn get_feature_flags(&self) -> Result<FeatureFlags>;
    pub async fn set_feature_flag(&self, feature: Feature, enabled: bool) -> Result<()>;

    // LLM provider management
    pub async fn list_llm_providers(&self) -> Result<Vec<LlmProviderConfig>>;
    pub async fn add_llm_provider(&self, config: LlmProviderConfig) -> Result<()>;
    pub async fn remove_llm_provider(&self, provider_id: &str) -> Result<()>;
    pub async fn test_llm_provider(&self, provider_id: &str) -> Result<TestResult>;
}
```

### Key Types

**Privacy Level**:
```rust
pub enum PrivacyLevel {
    Paranoid,      // No automation, no LLM, manual only
    LocalPrivacy,  // Local LLM only, automation allowed
    Balanced,      // Cloud LLM + PII filtering, all features
    Custom,        // User-defined feature flags
}
```

**Feature Flags**:
```rust
pub struct FeatureFlags {
    pub allow_local_llm: bool,
    pub allow_cloud_llm: bool,
    pub allow_browser_automation: bool,
    pub allow_email_sending: bool,
    pub allow_imap_monitoring: bool,
    pub allow_pii_scanning: bool,
}
```

**Permission Results**:
```rust
pub enum PermissionResult {
    Allowed,
    Denied { reason: String },
}

pub enum LlmPermission {
    Allowed { provider_id: String },  // Which provider to use
    Denied { reason: String },
}
```

**LLM Provider Config**:
```rust
pub struct LlmProviderConfig {
    pub provider_id: String,         // "ollama", "openai", "anthropic", etc.
    pub provider_type: ProviderType, // Local vs Cloud
    pub api_key: Option<String>,     // Encrypted in DB
    pub endpoint: Option<String>,    // For Ollama/LM Studio
    pub model: String,               // "gpt-4", "claude-sonnet-4", etc.
    pub enabled: bool,
    pub is_primary: bool,            // Primary provider for routing
}

pub enum ProviderType {
    Local,   // Ollama, LM Studio
    Cloud,   // OpenAI, Anthropic, Gemini
}
```

### Privacy Level Presets

When user selects a preset, engine sets these defaults:

**Paranoid**:
- All feature flags → `false`
- No LLM providers allowed
- User does everything manually

**LocalPrivacy**:
- `allow_local_llm` → `true`
- `allow_cloud_llm` → `false`
- Other features → `true`
- Only local providers (Ollama, LM Studio) can be used

**Balanced**:
- All flags → `true`
- PII filtering always enabled for cloud
- All features available

**Custom**:
- User configures each flag individually via granular toggles
- Starts from current settings

---

## LLM Provider Implementation

### Existing Providers (Already Implemented)

- **OllamaProvider** (`providers/ollama.rs`) - Local LLM via Ollama API
- **AnthropicProvider** (`providers/anthropic.rs`) - Claude API

### New Providers to Implement

**OpenAI Provider** (`providers/openai.rs`):
```rust
pub struct OpenAIProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    endpoint: String,  // Allow custom endpoint
}
```
- Default endpoint: `https://api.openai.com/v1`
- Custom endpoint support for LM Studio compatibility

**Gemini Provider** (`providers/gemini.rs`):
```rust
pub struct GeminiProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,  // "gemini-1.5-pro", "gemini-1.5-flash"
}
```
- Endpoint: `https://generativelanguage.googleapis.com/v1beta`
- Google's Gemini API format

**LM Studio Support**:
- Detect and auto-configure on startup
- Scan `http://localhost:1234/v1` for OpenAI-compatible API
- Offer to add if responding
- Treat as OpenAI-compatible provider with custom endpoint

**Microsoft Copilot**:
- Not in initial implementation (deferred to future phase)
- Would add Azure OpenAI Service support when requested

### Enhanced LlmRouter Integration

Modified `LlmRouter` to work with `PrivacyEngine`:

```rust
pub struct LlmRouter {
    providers: Vec<Arc<dyn LlmProvider>>,
    pii_filter: PiiFilter,
    privacy_engine: Arc<PrivacyEngine>,
    task_preferences: HashMap<TaskType, String>, // TaskType -> preferred provider_id
}

impl LlmRouter {
    pub async fn complete(&self, request: CompletionRequest, task: TaskType)
        -> Result<CompletionResponse>
    {
        // 1. Check permission with PrivacyEngine
        let permission = self.privacy_engine.can_use_llm(task).await?;
        let provider_id = match permission {
            LlmPermission::Allowed { provider_id } => provider_id,
            LlmPermission::Denied { reason } => return Err(LlmError::PermissionDenied(reason)),
        };

        // 2. Get provider
        let provider = self.get_provider(&provider_id)?;

        // 3. Apply PII filtering if cloud provider
        let (filtered_request, token_map) = if provider.capabilities().is_local {
            (request, None)
        } else {
            self.apply_pii_filter(request)?
        };

        // 4. Send to provider
        let response = provider.complete(filtered_request).await?;

        // 5. Detokenize if needed
        Ok(self.restore_pii(response, token_map))
    }
}
```

### Task-Based Routing

```rust
pub enum TaskType {
    EmailDraft,           // Simple, prefer cheap/local
    FormFilling,          // Simple, prefer cheap/local
}
```

**Routing logic in PrivacyEngine**:
1. Check task preferences: Did user set preferred provider for this task?
2. If yes and enabled → use that
3. If no preference → use primary provider
4. If primary unavailable → try fallback (if `fallback_enabled`)
5. Apply privacy level constraints throughout

**User control**:
- Set primary provider (default for all tasks)
- Override per task type (e.g., "Always use Ollama for emails, allow Claude for complex tasks")
- Enable/disable fallback

---

## Data Flow

### Setting Privacy Level

```
User clicks "Balanced" in UI
    ↓
Frontend calls: set_privacy_level(vault_id, "Balanced")
    ↓
Tauri command → PrivacyEngine::set_privacy_level()
    ↓
Engine writes to settings table:
  - privacy_level: {"level": "Balanced"}
  - feature_flags: {all true except paranoid features}
    ↓
Engine emits event: "privacy:settings_changed"
    ↓
Frontend updates UI state
```

### LLM Request Flow

```
User clicks "Draft Email" button
    ↓
Frontend calls: draft_removal_email(vault_id, broker_id)
    ↓
Tauri command gets PrivacyEngine
    ↓
Engine.can_use_llm(TaskType::EmailDraft)
    ↓
Engine checks:
  1. Privacy level allows LLM?
  2. Is local or cloud provider available?
  3. Which provider to use (task preference or primary)?
    ↓
Returns: LlmPermission::Allowed { provider_id: "ollama" }
    ↓
Service creates LlmRouter with selected provider
    ↓
Router completes request (with PII filtering if cloud)
    ↓
Response returned to frontend
```

---

## UI/UX Design

### Settings → Privacy Level Tab

**Replace "Coming Soon" with actual configuration**:

**Privacy Level Selector**:
- Radio buttons for: Paranoid | Local Privacy | Balanced | Custom
- Each option shows description
- Selecting preset immediately updates feature flags

**Custom Mode**:
- Granular toggles for each feature:
  - ☑ Allow local LLM (Ollama/LM Studio)
  - ☐ Allow cloud LLM (OpenAI/Claude/Gemini)
  - ☑ Allow browser automation
  - ☑ Allow automatic email sending
  - ☑ Allow IMAP monitoring
  - ☑ Allow local file scanning (PII discovery)

### Settings → LLM Providers Tab (New)

**Single configuration page with provider tabs**:
- Tabs: Ollama | LM Studio | OpenAI | Claude | Gemini
- Each tab shows:
  - Connection status (✓ Connected | ⚠ Not configured)
  - API key input (masked, never displayed)
  - Model selector (dropdown with available models)
  - Endpoint input (for Ollama/LM Studio)
  - "Test Connection" button
  - "Set as Primary" checkbox
  - Enable/disable toggle

**Per-task preferences** (below provider tabs):
- "Email Drafting" → Dropdown: [Use Primary] | Ollama | OpenAI | Claude | Gemini
- "Form Filling" → Dropdown: [Use Primary] | Ollama | OpenAI | Claude | Gemini
- "Enable Fallback" → Checkbox

**Auto-detection UI**:
- On app startup, scan for Ollama/LM Studio
- Show notification: "Detected Ollama running locally. Add as provider?"

### API Key Security

**Never display API keys in plaintext**:
- **Setting a key**: User pastes it (input shows `••••••••`)
- **Viewing status**: Shows "✓ OpenAI API key configured" vs "⚠ No API key set"
- **Editing**: Can replace key but can't see current one
- **Testing**: "Test connection" verifies key works without showing it

---

## Error Handling

### Error Types

```rust
pub enum LlmError {
    PermissionDenied(String),     // "Cloud LLM disabled at Local Privacy level"
    NoProviderAvailable,          // "No LLM providers configured"
    ProviderUnavailable(String),  // "Ollama not running on localhost:11434"
    ApiError(String),             // Provider-specific errors
    PiiFilterError(String),       // PII detection/filtering failed
}
```

### User-Facing Error Messages

**Permission denied**:
- Show reason + suggest action
- Example: "Cloud LLM is disabled at your current privacy level (Local Privacy). Enable it in Settings → Privacy Level or switch to Balanced mode."

**No provider available**:
- Guide to LLM Providers settings page
- Example: "No LLM providers configured. Go to Settings → LLM Providers to set up Ollama, OpenAI, or Claude."

**Provider unavailable**:
- Actionable guidance
- Example: "Ollama not responding at localhost:11434. Make sure Ollama is running (`ollama serve`)."

**API errors**:
- Show provider error message
- Example: "OpenAI API error: Invalid API key. Update your key in Settings → LLM Providers → OpenAI."

### Graceful Fallbacks

- Primary provider down → Try fallback if enabled
- Cloud provider PII filter fails → Block request (don't send unfiltered data)
- Invalid API key → Show "Test Connection" failed, prompt to update

---

## PII Filtering

**Always enabled for cloud providers** - user cannot disable.

**Behavior**:
- Existing `PiiFilter` with `FilterStrategy::Tokenize`
- Before sending to cloud: Tokenize names, emails, addresses, phone numbers
- Example: "Remove John Smith from Spokeo" → "Remove [TOKEN_1] [TOKEN_2] from Spokeo"
- After response: Detokenize back to original values
- Example: "Email sent to [TOKEN_1]" → "Email sent to John Smith"

**Privacy levels**:
- **Paranoid**: No cloud LLM allowed at all (no filtering needed)
- **LocalPrivacy**: Local LLM only (no filtering needed)
- **Balanced**: Cloud LLM with required PII filtering
- **Custom**: User can enable cloud LLM, but filtering is always mandatory

---

## LLM Features in App

### Email Drafting

**Location**: Removal attempt detail page or broker detail page

**Flow**:
1. User clicks "Draft Email with AI"
2. App checks `PrivacyEngine.can_use_llm(TaskType::EmailDraft)`
3. If allowed: Generate opt-out email using LLM
4. Show draft in text area (user can edit before sending)
5. User reviews and clicks "Send" or "Cancel"

**Prompt template**:
```
Draft a polite but firm data removal request email to [broker_name].
Include:
- Request to remove all personal information
- Reference to CCPA/GDPR rights if applicable
- Professional tone
- Keep it concise (3-4 sentences)
```

### Form Filling Assistance

**Location**: Browser automation flow (when filling opt-out forms)

**Flow**:
1. User clicks "Fill Form with AI"
2. App checks `PrivacyEngine.can_use_llm(TaskType::FormFilling)`
3. If allowed: LLM suggests values based on profile data
4. Pre-fill form fields (user can review/edit)
5. User submits or cancels

**Prompt template**:
```
Given this opt-out form with fields [field_names], and user profile [profile_data],
suggest appropriate values to fill in. Be concise and direct.
```

---

## Testing Strategy

### Unit Tests (spectral-privacy crate)

```rust
#[tokio::test]
async fn test_paranoid_blocks_all_llm() {
    let engine = test_engine().await;
    engine.set_privacy_level(PrivacyLevel::Paranoid).await.unwrap();

    let result = engine.can_use_llm(TaskType::EmailDraft).await.unwrap();
    assert!(matches!(result, LlmPermission::Denied { .. }));
}

#[tokio::test]
async fn test_local_privacy_allows_ollama_only() {
    let engine = test_engine().await;
    engine.set_privacy_level(PrivacyLevel::LocalPrivacy).await.unwrap();
    engine.add_llm_provider(ollama_config()).await.unwrap();
    engine.add_llm_provider(openai_config()).await.unwrap();

    let result = engine.can_use_llm(TaskType::EmailDraft).await.unwrap();
    match result {
        LlmPermission::Allowed { provider_id } => {
            assert_eq!(provider_id, "ollama");
        }
        _ => panic!("Should allow local LLM"),
    }
}

#[tokio::test]
async fn test_balanced_allows_cloud_with_filtering() {
    // Test that cloud providers are allowed and PII filtering is applied
}

#[tokio::test]
async fn test_custom_respects_feature_flags() {
    // Test that custom flags override defaults
}

#[tokio::test]
async fn test_task_routing_preferences() {
    // Test that per-task provider preferences work
}

#[tokio::test]
async fn test_fallback_when_primary_unavailable() {
    // Test fallback routing logic
}
```

### Integration Tests

- Test `LlmRouter` respects `PrivacyEngine` permissions
- Test provider auto-detection (Ollama, LM Studio)
- Test PII filtering on cloud requests
- Test task-based routing preferences
- Test API key encryption/decryption

### Manual Testing Checklist

**Privacy Levels**:
- [ ] Switch between privacy levels, verify UI updates
- [ ] Verify Paranoid blocks all LLM usage
- [ ] Verify LocalPrivacy allows only local providers
- [ ] Verify Balanced allows cloud with PII filtering
- [ ] Verify Custom mode allows granular control

**Provider Configuration**:
- [ ] Configure OpenAI with API key, test connection
- [ ] Configure Claude with API key, test connection
- [ ] Configure Gemini with API key, test connection
- [ ] Configure Ollama with endpoint, test connection
- [ ] Auto-detect LM Studio on localhost:1234
- [ ] Set primary provider, verify it's used by default

**LLM Features**:
- [ ] Draft email at each privacy level
- [ ] Fill form with AI at each privacy level
- [ ] Verify PII filtering (inspect what's sent to cloud)
- [ ] Test task routing with different providers
- [ ] Test fallback when primary unavailable

**Error Handling**:
- [ ] Invalid API key shows helpful error
- [ ] Offline provider shows connection error
- [ ] Permission denied shows upgrade path
- [ ] No provider configured shows setup guidance

---

## Implementation Phases

### Phase 1: Core Privacy Engine (Foundation)
- Create `spectral-privacy` crate
- Implement `PrivacyEngine` with settings storage
- Add privacy level presets
- Create database migration for `settings` table
- Unit tests for privacy engine

### Phase 2: LLM Provider Expansion
- Implement `OpenAIProvider`
- Implement `GeminiProvider`
- Add LM Studio auto-detection
- Update `LlmRouter` to integrate with `PrivacyEngine`
- Provider unit tests

### Phase 3: UI - Privacy Level Tab
- Replace "Coming Soon" with privacy level selector
- Implement preset radio buttons
- Implement custom mode toggles
- Add Tauri commands for settings management
- Frontend state management

### Phase 4: UI - LLM Providers Tab
- Create new Settings tab for LLM Providers
- Provider configuration forms with tabs
- API key input (masked)
- Test connection functionality
- Primary provider selection
- Per-task routing preferences

### Phase 5: LLM Features Integration
- Email drafting with LLM
- Form filling suggestions with LLM
- Error handling and user feedback
- PII filtering verification
- Integration tests

### Phase 6: Polish & Testing
- Auto-detection UI for Ollama/LM Studio
- Comprehensive error messages
- Manual testing against checklist
- Documentation updates
- Performance optimization

---

## Success Criteria

- [ ] User can select and save privacy level (all 4 modes work)
- [ ] Privacy level correctly gates LLM access
- [ ] All 5 providers can be configured (Ollama, LM Studio, OpenAI, Claude, Gemini)
- [ ] API keys stored encrypted, never displayed
- [ ] Test connection works for each provider
- [ ] Primary provider + fallback routing works
- [ ] Per-task routing preferences work
- [ ] Email drafting uses correct provider based on settings
- [ ] Form filling uses correct provider based on settings
- [ ] PII filtering verified on all cloud requests
- [ ] Error messages are helpful and actionable
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Manual testing checklist complete

---

## Future Enhancements (Out of Scope for Initial Implementation)

- Microsoft Copilot integration (Azure OpenAI, GitHub Copilot)
- Broker analysis LLM features (analyze privacy policies)
- Follow-up suggestion LLM features (generate follow-up messages)
- User-initiated chat interface
- Rate limiting (max N cloud requests per day at certain privacy levels)
- Cost tracking for cloud provider usage
- Model selection per task type
- Streaming responses for real-time feedback
- Fine-tuned models for privacy-specific tasks
