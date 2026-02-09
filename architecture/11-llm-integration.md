## 11. LLM Integration Strategy

### 11.1 Task Classification & Routing

Not all tasks require the same LLM capability. The router classifies tasks and selects the best available provider:

| Task | PII Exposure | Minimum Capability | Preferred Provider |
|------|-------------|-------------------|-------------------|
| Parse opt-out instructions from broker page | None | Basic completion | Any (local preferred) |
| Generate opt-out email from template | Tokenized PII | Basic completion | Local preferred |
| Interpret scan results (is this me?) | Hashed summary | Moderate reasoning | Local preferred |
| Navigate complex broker UI (LLM-guided) | Limited (current page content) | Vision + tool use | Capable model required |
| Conversational status queries | None | Basic completion | Any |
| Explain broker privacy practices | None | Basic completion | Any |
| Compose appeal for rejected removal | Tokenized PII | Strong writing | Best available |
| Classify broker email reply | Sanitized excerpt | Basic completion | Local preferred |
| Draft broker follow-up reply | Tokenized PII | Basic completion | Local preferred |
| Generate natural language summaries | None | Basic completion | Any |
| Local PII document classification | Document excerpt (local) | Moderate reasoning | Force local |

### 11.2 Local LLM Recommendations

For the README / docs, provide guidance on minimum specs:

| Task Type | Minimum Local Model | VRAM Required |
|-----------|-------------------|---------------|
| Chat & status queries | Llama 3.1 8B (Q4_K_M) | ~6 GB |
| Email generation | Mistral 7B or Llama 3.1 8B | ~6 GB |
| Complex reasoning | Llama 3.1 70B (Q4_K_M) | ~40 GB |
| Vision tasks | LLaVA 13B or Llama 3.2 Vision | ~10 GB |

### 11.3 Feature Behavior: LLM On vs. Off

Every feature has a defined fallback when LLM is disabled:

| Feature | With LLM | Without LLM |
|---------|----------|-------------|
| **Chat interface** | Natural language conversation with tool orchestration | Replaced by a structured command palette / wizard UI |
| **Scan broker** | LLM interprets ambiguous results, confirms matches | Deterministic matching (exact/fuzzy name + location), manual confirmation |
| **Submit removal** | LLM generates personalized opt-out emails, navigates complex forms | Template-based emails with field substitution, scripted form fills only |
| **Match confidence** | LLM analyzes listing context to score match likelihood | Simple fuzzy string matching score (Levenshtein + field overlap) |
| **Broker def repair** | LLM navigates changed site, proposes updated selectors | Flags broken definitions, links to community issue tracker |
| **Status summaries** | Natural language progress narratives | Structured table/card view with status badges |
| **Local PII discovery** | LLM classifies documents, understands context, finds implicit PII | Regex + pattern matching for structured PII (SSN, email, phone, address formats) |
| **Email scanning** | LLM understands email context, identifies accounts/services | Header/sender analysis, regex extraction, known-service domain matching |
| **Broker email replies** | LLM classifies and drafts contextual responses | Static template selection based on keyword matching |
| **Privacy score narrative** | LLM generates plain-English trend explanation | Score displayed numerically with grade badge |

### 11.4 Configuration

```toml
# ~/.config/spectral/config.toml

[llm]
enabled = false                          # Master kill switch — false by default

[llm.provider]
# Only read if llm.enabled = true
type = "ollama"                          # "anthropic", "openai", "ollama", "llamacpp", "lmstudio", "vllm"
model = "llama3.1:8b"
endpoint = "http://localhost:11434"
# api_key stored in vault, not in config file

[llm.routing]
preference = "local_only"                # "local_only", "prefer_local", "best_available"
pii_filter = "tokenize"                  # "tokenize", "redact", "block"

# Per-feature toggles — every feature with LLM usage is independently controllable
[llm.features]
chat_interface = true
natural_language_commands = true
llm_guided_browser = false               # off by default — advanced, higher risk
smart_match_confidence = true
auto_broker_repair = false               # off by default — modifies definitions
opt_out_email_generation = true
natural_language_summaries = true
threat_narrative = false

[llm.features.file_system_pii_scan]
enabled = false
routing = "force_local"                  # always force local for filesystem scanning

[llm.features.email_pii_scan]
enabled = false
routing = "force_local"                  # always force local for email scanning
```

**Key design decisions:**
- LLM is **disabled by default** — users must explicitly opt in
- Each feature has its own toggle
- Discovery features (filesystem, email) default to off AND force local routing
- Config file never contains API keys — those go in the encrypted vault

---
