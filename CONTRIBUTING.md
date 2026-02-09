# Contributing to Spectral

This document describes the development patterns, tooling, and workflows for contributing to Spectral.

## Development Environment Setup

1. Run the setup script (Linux/WSL2):
   ```bash
   chmod +x setup.sh
   ./setup.sh
   ```

2. Install pre-commit hooks:
   ```bash
   pip install pre-commit
   pre-commit install --install-hooks
   pre-commit install --hook-type commit-msg
   ```

3. Install additional cargo tools:
   ```bash
   cargo install cargo-deny cargo-cyclonedx cargo-tarpaulin
   ```

## Git Hooks

Pre-commit hooks run automatically before each commit. They enforce:

| Hook | What it checks |
|------|----------------|
| `cargo fmt` | Rust code formatting |
| `cargo clippy` | Rust lints (warnings = errors) |
| `cargo deny` | Dependency vulnerabilities and licenses |
| `npm lint` | Frontend ESLint rules |
| `npm check` | Svelte/TypeScript type checking |
| `prettier` | Frontend code formatting |
| `semgrep` | Security rules (OWASP, secrets, custom) |
| `detect-secrets` | Prevents committing secrets |
| `conventional-pre-commit` | Commit message format |

### Running Hooks Manually

```bash
# Run all hooks on all files
pre-commit run --all-files

# Run a specific hook
pre-commit run cargo-fmt --all-files
pre-commit run semgrep --all-files

# Skip hooks (use sparingly, with justification)
git commit --no-verify -m "fix: emergency hotfix"
```

## Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/). Format:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

### Types

| Type | Use for |
|------|---------|
| `feat` | New features |
| `fix` | Bug fixes |
| `docs` | Documentation changes |
| `style` | Formatting, no code change |
| `refactor` | Code change that neither fixes nor adds |
| `perf` | Performance improvements |
| `test` | Adding/updating tests |
| `build` | Build system, dependencies |
| `ci` | CI/CD changes |
| `chore` | Maintenance tasks |
| `security` | Security fixes |

### Examples

```bash
feat(vault): add ChaCha20-Poly1305 encryption

fix(broker-engine): handle timeout on spokeo requests

docs: update README with installation instructions

security(mail): sanitize email body before LLM processing
```

## Code Quality Tools

### SonarQube

Analyzes code quality, coverage, and duplication.

```bash
# Set token (get from http://192.168.1.220:9000)
export SONAR_TOKEN='sqa_...'

# Run analysis
./scripts/sonar-scan.sh

# PR analysis
./scripts/sonar-scan.sh --pr 123
```

View results at: http://192.168.1.220:9000/dashboard?id=spectral

### Dependency Track

Tracks dependencies and vulnerabilities via SBOM.

```bash
# Set API key (get from http://192.168.1.220:8081)
export DTRACK_API_KEY='...'

# Generate and upload SBOMs
./scripts/dependency-track.sh

# Generate only (no upload)
./scripts/dependency-track.sh --generate
```

View results at: http://192.168.1.220:8081/projects

### Semgrep

Security scanning with custom rules for Spectral.

```bash
# Run locally
semgrep --config=.semgrep/ --config=p/security-audit .

# View Spectral-specific rules
cat .semgrep/spectral-rules.yaml
```

### cargo-deny

Checks dependencies for vulnerabilities, licenses, and banned crates.

```bash
cargo deny check
```

Configuration: `deny.toml`

## Testing

### Rust Tests

```bash
# All tests
cargo test --manifest-path src-tauri/Cargo.toml --all

# Single crate
cargo test -p spectral-vault

# With coverage
cargo tarpaulin --manifest-path src-tauri/Cargo.toml --out Html
```

### Frontend Tests

```bash
npm run test        # Unit tests
npm run test:e2e    # E2E tests (Playwright)
```

## Security Requirements

Spectral has strict security requirements. All code must:

1. **Never send data externally** - No telemetry, analytics, or cloud sync
2. **Use proper encryption** - ChaCha20-Poly1305 for data, Argon2id for passwords
3. **Sanitize LLM inputs** - All external text must be sanitized before LLM exposure
4. **Use parameterized queries** - Never format SQL strings
5. **Avoid unsafe code** - Document any `unsafe` blocks thoroughly

The Semgrep rules in `.semgrep/spectral-rules.yaml` enforce many of these.

## Directory Structure

```
spectral/
├── src/                    # SvelteKit frontend
├── src-tauri/              # Tauri app shell
├── crates/                 # Rust library crates
├── broker-definitions/     # Data broker TOML configs
├── company-definitions/    # Commercial company configs
├── scripts/                # Development scripts
├── .semgrep/               # Custom Semgrep rules
├── .pre-commit-config.yaml # Git hooks
├── deny.toml               # cargo-deny config
└── sonar-project.properties # SonarQube config
```

## Pull Request Process

1. Create a feature branch from `main`
2. Make changes with conventional commits
3. Ensure all hooks pass: `pre-commit run --all-files`
4. Run tests: `cargo test` and `npm run test`
5. Push and create PR
6. SonarQube and Dependency Track will analyze automatically
7. Address review feedback
8. Squash merge to main

## Questions?

- Architecture decisions: Check `spectral-unified-architecture.md`
- Quick reference: Check `claude.md`
- Tooling issues: Check this document or ask in the PR
