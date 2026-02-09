## 13. Project Structure

```
spectral/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── spectral-vault/           # Encrypted storage & PII management
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── spectral-llm/             # LLM abstraction, routing, PII filtering
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── spectral-broker/          # Broker definitions, scanning, removal logic
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── spectral-browser/         # Headless browser automation
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── spectral-plugins/         # WASM plugin runtime
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── spectral-chat/            # Conversational engine & tool orchestration
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── spectral-scheduler/       # Background task scheduling & retry
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── spectral-core/            # Shared types, error handling, config, correlation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── capabilities.rs   # CapabilityRegistry (LLM-optional gates)
│   │       ├── correlation.rs    # Cross-module intelligence engine
│   │       └── reporting.rs      # Report types & data structures
│   ├── spectral-discovery/       # Local PII discovery engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── orchestrator.rs
│   │       ├── detector.rs       # PII detection pipeline (regex + NER + optional LLM)
│   │       ├── scanners/
│   │       │   ├── mod.rs
│   │       │   ├── filesystem.rs
│   │       │   ├── email.rs
│   │       │   └── browser.rs
│   │       └── parsers/
│   │           ├── mod.rs
│   │           ├── plaintext.rs
│   │           ├── office.rs
│   │           ├── pdf.rs
│   │           └── image.rs      # Optional OCR
│   ├── spectral-permissions/     # Granular permission system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── manager.rs
│   │       ├── presets.rs
│   │       ├── audit.rs
│   │       └── prompts.rs
│   ├── spectral-netmon/          # Network telemetry engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── scheduler.rs
│   │       ├── intelligence.rs   # Domain classification DB
│   │       ├── scoring.rs        # Privacy score calculation
│   │       ├── baseline.rs       # Baseline tracking & comparison
│   │       ├── collectors/
│   │       │   ├── mod.rs
│   │       │   ├── dns.rs        # DNS cache reader
│   │       │   ├── connections.rs # netstat/ss reader
│   │       │   └── firewall.rs   # Firewall log parser
│   │       └── correlation.rs    # Cross-module intelligence
│   ├── spectral-verify/          # Removal verification engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── scheduler.rs      # Verification check scheduling
│   │       ├── legal.rs          # Legal timeline tracking
│   │       └── escalation.rs     # Escalation pipeline
│   └── spectral-mail/            # Third-party communication engine
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── safety.rs         # LLM safety guardrails (anti-injection)
│           ├── budget.rs         # Reply budget & rate limiting
│           ├── templates.rs      # Static reply templates
│           ├── classifier.rs     # Response classification
│           └── thread.rs         # Thread state machine
├── src-tauri/                    # Tauri application shell
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── src/
│   │   ├── main.rs
│   │   └── commands/             # Tauri IPC command handlers
│   │       ├── vault.rs
│   │       ├── scan.rs
│   │       ├── chat.rs
│   │       ├── permissions.rs
│   │       ├── netmon.rs
│   │       └── discovery.rs
│   └── icons/
├── src/                          # Frontend (React + TypeScript)
│   ├── App.tsx
│   ├── components/
│   │   ├── Chat/                 # Conversational interface
│   │   ├── Dashboard/            # Status overview, charts, privacy score
│   │   ├── Profile/              # PII management
│   │   ├── BrokerList/           # Browse/search brokers
│   │   ├── Discovery/            # Local PII scan results UI
│   │   ├── Permissions/          # Permission management UI
│   │   │   ├── PermissionWizard.tsx
│   │   │   ├── PermissionManager.tsx
│   │   │   └── AuditLog.tsx
│   │   ├── CommandPalette/       # Non-LLM structured interface
│   │   ├── Settings/             # LLM config, proxy, scheduling
│   │   └── common/               # Shared UI components
│   ├── hooks/
│   │   ├── useCapabilities.ts    # LLM/feature capability queries
│   │   └── ...
│   ├── stores/                   # Zustand state management
│   └── lib/
│       └── tauri.ts              # IPC bindings
├── brokers/                      # Community-maintained broker definitions
│   ├── people-search/
│   ├── data-aggregators/
│   ├── marketing/
│   └── public-records/
├── domains/                      # Domain intelligence definitions
│   ├── data-brokers/
│   ├── ad-networks/
│   ├── trackers/
│   ├── analytics/
│   └── sources.toml              # External list URLs for auto-import
├── plugins/                      # Official plugins
├── docs/
│   ├── ARCHITECTURE.md
│   ├── CONTRIBUTING.md
│   ├── SECURITY.md
│   └── PLUGIN_DEVELOPMENT.md
├── .github/
│   ├── workflows/
│   └── ISSUE_TEMPLATE/
└── LICENSE                       # AGPLv3
```

---
