## 2. High-Level Architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│                            Tauri Shell                               │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │                Frontend (TypeScript/React)                     │  │
│  │  ┌────────────┐ ┌───────────┐ ┌──────────┐ ┌──────────────┐  │  │
│  │  │ Chat /     │ │ Dashboard │ │ Discovery│ │  Settings    │  │  │
│  │  │ Cmd Palette│ │ & Status  │ │ & Alerts │ │  & Profile   │  │  │
│  │  └─────┬──────┘ └─────┬─────┘ └────┬─────┘ └──────┬───────┘  │  │
│  └────────┼──────────────┼────────────┼───────────────┼──────────┘  │
│           │       Tauri IPC (Commands/Events)         │            │
│  ┌────────┴──────────────┴────────────┴───────────────┴──────────┐  │
│  │                      Rust Core Engine                          │  │
│  │                                                                │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐ │  │
│  │  │  LLM Router  │  │   Broker     │  │  Permission Manager  │ │  │
│  │  │  & Adapter   │  │   Engine     │  │  & Audit Logger      │ │  │
│  │  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘ │  │
│  │         │                 │                      │            │  │
│  │  ┌──────┴───────┐  ┌─────┴────────┐  ┌──────────┴──────────┐ │  │
│  │  │  Browser     │  │  Encrypted   │  │  Capability         │ │  │
│  │  │  Automation  │  │  Vault       │  │  Registry           │ │  │
│  │  └──────────────┘  └──────────────┘  └─────────────────────┘ │  │
│  │                                                                │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌─────────────────────┐ │  │
│  │  │  Discovery   │  │  Network     │  │  Verification &     │ │  │
│  │  │  Engine      │  │  Telemetry   │  │  Mail Engine        │ │  │
│  │  └──────────────┘  └──────────────┘  └─────────────────────┘ │  │
│  │                                                                │  │
│  │  ┌──────────────┐  ┌──────────────┐                          │  │
│  │  │  Scheduler   │  │  Plugin      │                          │  │
│  │  │  & Queue     │  │  Runtime     │                          │  │
│  │  └──────────────┘  └──────────────┘                          │  │
│  └────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────┘
```

### Technology Stack

| Layer | Technology | Rationale |
|-------|-----------|-----------|
| **GUI Framework** | Tauri v2 | Rust-native, small binary (~5MB vs Electron's ~150MB), cross-platform, strong security model with IPC permissions |
| **Frontend** | React + TypeScript + Tailwind | Large ecosystem, easy to find contributors, good component libraries |
| **Core Engine** | Rust | Memory safety, performance, strong type system, excellent async (tokio) |
| **Browser Automation** | chromiumoxide (Rust) or Playwright via sidecar | Headless browser for scanning and form automation |
| **Encrypted Storage** | SQLCipher (SQLite + AES-256) | Battle-tested encrypted database, single-file, cross-platform |
| **LLM Integration** | Custom abstraction layer | Unified trait for OpenAI, Anthropic, Ollama, llama.cpp, LM Studio |
| **Plugin System** | Extism (WASM) | Language-agnostic plugins sandboxed in WASM, safe execution |
| **Task Scheduling** | tokio-cron-scheduler | Periodic re-scans, retry logic, background processing |

---
