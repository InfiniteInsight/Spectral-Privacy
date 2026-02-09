# Spectral

Open-source, local-first privacy tool for automated data broker removal and personal data management. Licensed AGPLv3.

---

## YOU ARE THE SPECTRAL PROJECT MANAGER

**Claude operates as the Spectral PM by default.** This means you:

1. **Coordinate specialists** - Activate the right skill for each task:
   - Security work → `spectral:infosec`, then `spectral:pentester`
   - Broker work → `spectral:broker-research` → `spectral:broker-definition`
   - UI work → `spectral:accessibility` + `spectral:patterns`
   - Email/LLM → `spectral:email-safety`
   - Legal questions → `spectral:legal`
   - All code → `spectral:patterns` (always)

2. **Enforce quality gates** before marking work complete:
   - [ ] Relevant architecture section read
   - [ ] Patterns.md compliance verified
   - [ ] Tests written and passing
   - [ ] Security review if applicable
   - [ ] Pre-commit hooks pass

3. **Report clearly** to Evan (product owner):
   - What was done
   - Decisions made
   - Questions or blockers
   - Next steps

4. **Escalate appropriately**:
   - Architectural changes → Ask Evan
   - Security tradeoffs → Ask Evan
   - Scope questions → Ask Evan
   - Privacy implications → Ask Evan

5. **Work autonomously** on:
   - Implementation within established patterns
   - Bug fixes that don't change behavior
   - Test coverage improvements
   - Documentation updates

**When spawning subagents**, include: "Follow patterns.md. Report findings clearly."

---

## Tech Stack

- **Backend:** Rust (2021 edition), Tauri v2 desktop framework
- **Frontend:** Svelte 5, SvelteKit (static adapter, SSR disabled), TypeScript, Tailwind CSS, shadcn-svelte
- **Database:** SQLite via SQLx (will migrate to SQLCipher for encryption)
- **Build:** Cargo workspace, npm, Vite
- **CI:** GitHub Actions (Linux, Windows, macOS)

## Project Structure

```
src/                          # SvelteKit frontend (routes, components, stores)
src-tauri/                    # Tauri app shell (thin — registers commands, manages windows)
  src/main.rs                 # Entry point
  src/lib.rs                  # Command registrations
  Cargo.toml                  # Workspace root
  tauri.conf.json             # Tauri configuration
crates/                       # Rust library crates (core logic)
  spectral-auth/              # App authentication (PIN, biometrics, session management)
  spectral-vault/             # PII vault — encryption (ChaCha20-Poly1305), Argon2id KDF
  spectral-db/                # Database layer (SQLite/SQLCipher, migrations)
  spectral-broker-engine/     # Broker definitions, scanning, opt-out automation
  spectral-browser/           # Browser automation (chromiumoxide)
  spectral-mail/              # Email generation, SMTP/IMAP, OAuth2, safety pipeline
  spectral-network/           # Network telemetry, DNS/connection monitoring
  spectral-jurisdiction/      # Legal/regulatory engine, privacy law database
  spectral-verification/      # Removal verification, deadline tracking
  spectral-commercial/        # Commercial relationship engine (email pattern analysis)
  spectral-permissions/       # Granular permission system (Paranoid→Balanced presets)
  spectral-llm/               # LLM router (optional AI features, prompt injection defense)
broker-definitions/           # TOML files defining data broker sites (CC-BY-SA)
company-definitions/          # TOML files for commercial companies (deletion URLs, privacy contacts)
```

Not all crates exist yet — they are scaffolded incrementally as development progresses.

## Commands

```bash
cargo tauri dev                              # Full app: Vite + Rust + window (primary dev command)
npm run dev                                  # Frontend only at localhost:5173 (no Tauri APIs)
cargo test --manifest-path src-tauri/Cargo.toml --all  # All Rust tests
cargo test -p spectral-vault                 # Single crate tests
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
npm run lint                                 # Frontend ESLint
npm run check                                # Svelte/TypeScript type checking
cargo tauri build                            # Release binary for current OS
```

## Architecture Principles

- **Local-only:** All PII stays on the user's device. No telemetry. No cloud sync. Ever.
- **LLM-optional:** Every feature must work without an LLM. AI enhances but is never required.
- **Encryption-first:** PII vault uses ChaCha20-Poly1305 with Argon2id key derivation from master password.
- **Authentication-required:** Users must authenticate to access the app (PIN/biometrics) and unlock the vault (master password).
- **Jurisdiction-aware:** Legal templates, deadlines, and escalation paths adapt to user + broker location.
- **Graceful degradation:** Features degrade gracefully when permissions are restricted or system access is limited.
- **Prompt injection defense:** Any text from third parties (emails, broker responses) is sanitized before LLM exposure.

## Authentication Model

Spectral uses layered authentication:

| Layer | Purpose | Methods |
|-------|---------|---------|
| **App Launch** | Prevent unauthorized access | PIN (4-6 digits), Biometrics (Touch ID, Windows Hello, fprintd), Password |
| **Vault Unlock** | Decrypt PII data | Master Password → Argon2id (256MB) → ChaCha20-Poly1305 |
| **Credentials** | External services | Encrypted in vault (LLM API keys, SMTP/IMAP, OAuth2 tokens) |

**Platform biometrics:**
- **Windows:** Windows Hello (face, fingerprint, PIN)
- **macOS:** Touch ID via LocalAuthentication framework
- **Linux:** fprintd (fingerprint), polkit (system auth dialog), PAM

**Session management:**
- Auto-lock after 15 min inactivity (configurable)
- 5 failed PIN attempts → 5 min lockout
- Vault keys zeroized from memory on lock

See `patterns.md` Section 11 for implementation details.

## Rust Conventions

- Use `thiserror` for library crate errors, `anyhow` only in the Tauri app shell
- Prefer `tracing` over `println!` or `log` for all logging
- All public functions need doc comments
- Use `#[cfg(test)]` modules in each file, plus integration tests in `tests/`
- Shared dependency versions are defined in workspace `[workspace.dependencies]` in `src-tauri/Cargo.toml`
- Run `cargo clippy` with `-D warnings` — treat all warnings as errors

## Frontend Conventions

- SvelteKit with `adapter-static` and `ssr = false` (required for Tauri)
- Use `$lib/` alias for imports from `src/lib/`
- Tauri commands are called via `@tauri-apps/api` — wrap them in `$lib/api/` modules
- Component library: shadcn-svelte (Bits UI + Tailwind). Add components as needed.
- Stores in `$lib/stores/` for shared state

## Key Domain Concepts

- **Broker:** A data broker website (Spokeo, BeenVerified, etc.) that aggregates and sells personal info
- **Removal request:** A formal deletion/opt-out request sent to a broker citing applicable privacy law
- **Commercial relationship:** A non-broker company the user has done business with (detected via email patterns)
- **Jurisdiction:** The user's location determines which privacy laws apply (CCPA, GDPR, VCDPA, etc.)
- **Permission preset:** User's privacy comfort level — Paranoid, LocalPrivacy, Balanced, or Custom
- **Email mode:** How Spectral sends removal emails — Draft (generates .eml), Hybrid (SMTP only), Full Automation (SMTP + IMAP)

## Documentation

### Architecture (architecture/)

Split into sections for easier navigation. See `architecture/README.md` for index.

| File | Topic |
|------|-------|
| `03-core-modules.md` | Vault, LLM router, broker engine, browser automation, plugins |
| `07-mail-communication.md` | Email engine, safety guardrails, prompt injection defense |
| `08-permissions.md` | Granular permission system |
| `09-security.md` | Threat model, PII handling rules |
| `11-llm-integration.md` | LLM task routing, local vs cloud, feature toggles |
| `14-database-schema.md` | SQLite/SQLCipher schema |
| `20-onboarding.md` | User onboarding wizard (6-step flow) |
| `21-geolocation-jurisdiction.md` | Privacy law database, jurisdiction detection |
| `22-proactive-scanning.md` | Broker scanning model |
| `23-commercial-relationships.md` | Non-broker deletion engine |
| `24-resolved-questions.md` | All resolved design decisions |

### Development Patterns (patterns.md)

Coding patterns and conventions:

| Section | Topic |
|---------|-------|
| 1 | Error handling (thiserror vs anyhow) |
| 2 | Async & concurrency (cancellation, channels) |
| 3 | State management (Tauri state, Svelte stores) |
| 4 | Testing patterns |
| 5 | Logging & tracing |
| 6 | Configuration |
| 7 | API design (Tauri commands) |
| 8 | Frontend components (shadcn-svelte) |
| 9 | Security patterns (PII handling, sanitization) |
| 10 | Database patterns (SQLx, migrations) |
| 11 | Authentication (all layers, biometrics, OAuth2) |

## Development Environment

- **Primary:** WSL2 Ubuntu 24.04, VS Code Remote-WSL
- **Windows builds:** Native Windows toolchain (VS Build Tools, Rust MSVC target)
- **macOS builds:** GitHub Actions only (no local macOS)
- **Project location:** Always on Linux filesystem (`~/projects/spectral`), never `/mnt/c/`

## Working With This Codebase

- Evan is the product owner/PM. Claude is the developer.
- When unsure about architectural decisions, check the architecture document before proceeding.
- TOML broker/company definitions should follow the schema in `broker-definitions/schema.toml`.
- Never add telemetry, analytics, or any network calls that send user data externally.
- CAPTCHAs are solved by the human user, never automated.
- Auto-reply emails have global daily caps (default 10/day) to prevent email provider flagging.

## Skill-Based Development

### Primary Orchestrator: `spectral:pm`

For complex tasks, use the **Project Manager skill** as the coordinator:

```
"As spectral:pm, implement the vault unlock feature"
"As spectral:pm, add support for a new broker"
"As spectral:pm, review the authentication system"
```

The PM will:
- Plan work and identify affected components
- Activate specialist skills at the right time
- Ensure quality gates before completion
- Report status and decisions

### Available Specialist Skills

| Skill | Use For |
|-------|---------|
| `spectral:patterns` | Code pattern compliance |
| `spectral:infosec` | Security review |
| `spectral:pentester` | Offensive security testing |
| `spectral:broker-research` | Data broker research |
| `spectral:broker-definition` | Broker TOML files |
| `spectral:legal` | Privacy law, jurisdiction |
| `spectral:email-safety` | Email/LLM security |
| `spectral:accessibility` | WCAG, screen readers |

See `.claude/skills/` for full skill definitions.

---

## MANDATORY: Pattern Compliance

**Before writing any code, Claude and all subagents MUST:**

1. **Read `patterns.md`** for the relevant section before implementing:
   - Error handling → Section 1
   - Async code → Section 2
   - State management → Section 3
   - Tests → Section 4
   - Logging → Section 5
   - Config → Section 6
   - Tauri commands → Section 7
   - Svelte components → Section 8
   - Security/PII → Section 9
   - Database → Section 10
   - Authentication → Section 11

2. **Follow these non-negotiable rules:**
   - Use `thiserror` in library crates, `anyhow` only in src-tauri/src/
   - Use `tracing` for logging, never `println!` or `log`
   - Use `Zeroizing<T>` for any sensitive data (passwords, keys, PII)
   - Use `sqlx::query!` macro, never string-formatted SQL
   - All async operations must support `CancellationToken`
   - All PII must be encrypted before storage
   - Never log PII - use IDs or hashed summaries
   - Wrap Tauri commands in `$lib/api/` modules on frontend

3. **Before completing a task, verify:**
   - [ ] Code follows patterns.md conventions
   - [ ] No `.unwrap()` in production code (use `?` or `.expect("reason")`)
   - [ ] Errors have context at module boundaries
   - [ ] Tests use `#[cfg(test)]` modules or `tests/` directory
   - [ ] No PII in logs or error messages

**Subagent instructions:** When spawning Task agents, include in the prompt:
> "Follow patterns in patterns.md. Use thiserror for errors, tracing for logs, Zeroizing for secrets."
