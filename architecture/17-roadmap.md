## 17. Development Roadmap

### Phase 1: Foundation (v0.1) — ~8 weeks
- [ ] Cargo workspace scaffolding with all crates
- [ ] Encrypted vault with master password and Argon2id KDF
- [ ] Basic Tauri shell with unlock screen and profile setup
- [ ] LLM abstraction with Anthropic + Ollama support
- [ ] LLM-optional capability registry and kill switch
- [ ] 5 broker definitions (Spokeo, BeenVerified, WhitePages, FastPeopleSearch, TruePeopleSearch)
- [ ] Manual scan trigger with results display
- [ ] Basic chat interface for status queries (+ command palette fallback)
- [ ] Granular permission system with first-run wizard

### Phase 2: Automation (v0.2) — ~6 weeks
- [ ] Browser automation engine with headless Chromium
- [ ] Automated opt-out form submission for Tier 1 brokers
- [ ] Email-based removal flow (generate and send via user's SMTP or copy-to-clipboard)
- [ ] Scan scheduling and background re-checks
- [ ] Dashboard with status tracking, privacy score, and timeline
- [ ] Additional LLM providers (OpenAI, LM Studio, llama.cpp)
- [ ] Removal verification engine with legal timeline tracking
- [ ] Third-party communication engine with safety guardrails

### Phase 3: Intelligence (v0.3) — ~6 weeks
- [ ] LLM-guided browser sessions for complex brokers
- [ ] PII tokenization pipeline for cloud LLM safety
- [ ] Smart match confidence scoring (is this listing actually me?)
- [ ] Automated broker definition updates when procedures change
- [ ] Plugin system (Extism WASM runtime)
- [ ] Local PII discovery engine (filesystem, email, browser scanners)
- [ ] Network telemetry engine with privacy scoring
- [ ] Cross-correlation intelligence and insights
- [ ] Notification integrations (first-party: desktop notifications, email digest)

### Phase 4: Community (v0.4) — ~4 weeks
- [ ] Plugin marketplace / registry
- [ ] Broker definition contribution workflow
- [ ] Automated CI testing of broker definitions
- [ ] Multi-profile support (family plans)
- [ ] Export/reporting features (Markdown, PDF, JSON)
- [ ] Domain intelligence community contribution workflow
- [ ] Comprehensive documentation and contributor guides

---
