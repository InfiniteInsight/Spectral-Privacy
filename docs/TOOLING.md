# Spectral Development Tooling

This document describes all development, security, and quality tools used in Spectral.

## Quick Reference

| Category | Tool | Command |
|----------|------|---------|
| **Static Analysis** | Semgrep | `semgrep --config=.semgrep/ .` |
| **Code Quality** | SonarQube | `./scripts/sonar-scan.sh` |
| **Dependency Vulnerabilities** | Dependency Track | `./scripts/dependency-track.sh` |
| **Rust Vulnerabilities** | cargo-audit | `cargo audit` |
| **License Compliance** | cargo-deny | `cargo deny check` |
| **Unsafe Code Audit** | cargo-geiger | `cargo geiger` |
| **Fuzzing** | cargo-fuzz | `cargo +nightly fuzz run <target>` |
| **Mutation Testing** | cargo-mutants | `cargo mutants` |
| **Coverage** | cargo-tarpaulin | `cargo tarpaulin --out Html` |
| **Benchmarks** | criterion | `cargo bench` |
| **Profiling** | flamegraph | `cargo flamegraph` |
| **Binary Hardening** | checksec | `checksec --file=target/release/spectral` |
| **Release Signing** | cosign | `cosign sign-blob <file>` |

---

## Static Analysis

### Semgrep

Pattern-based static analysis for security and code patterns.

```bash
# Run all rules
semgrep --config=.semgrep/ .

# Run specific ruleset
semgrep --config=.semgrep/spectral-rules.yaml .
semgrep --config=.semgrep/pattern-enforcement.yaml .

# Run community rules
semgrep --config=p/rust .
semgrep --config=p/typescript .
```

**Configuration:** `.semgrep/spectral-rules.yaml`, `.semgrep/pattern-enforcement.yaml`

### SonarQube

Comprehensive code quality and security analysis.

```bash
# Full analysis
export SONAR_TOKEN='sqa_...'
./scripts/sonar-scan.sh

# PR analysis
./scripts/sonar-scan.sh --pr 123
```

**Dashboard:** http://192.168.1.220:9000/dashboard?id=spectral
**Configuration:** `sonar-project.properties`

---

## Dependency Security

### Dependency Track

SBOM-based vulnerability tracking.

```bash
# Generate and upload SBOM
export DTRACK_API_KEY='your-api-key-here'  # pragma: allowlist secret
./scripts/dependency-track.sh

# Generate only (no upload)
./scripts/dependency-track.sh --generate
```

**Dashboard:** http://192.168.1.220:8081/projects
**Output:** `sbom/rust-bom.json`, `sbom/npm-bom.json`

### cargo-audit

RustSec advisory database checking.

```bash
# Check for vulnerabilities
cargo audit

# Generate JSON report
cargo audit --json > audit-report.json

# Fix vulnerabilities (updates Cargo.lock)
cargo audit fix
```

### cargo-deny

Comprehensive dependency checking (vulnerabilities, licenses, bans).

```bash
# Run all checks
cargo deny check

# Run specific check
cargo deny check advisories
cargo deny check licenses
cargo deny check bans
cargo deny check sources
```

**Configuration:** `deny.toml`

---

## Rust Safety

### cargo-geiger

Audit unsafe code usage in dependencies.

```bash
# Full report
cargo geiger

# Only show unsafe in your code (not deps)
cargo geiger --only-local

# Machine-readable output
cargo geiger --output-format Json > geiger-report.json
```

**Metric meanings:**
- 🔒 = No unsafe code
- ❓ = Unable to determine
- ☢️ = Contains unsafe code

### cargo-careful

Run with extra runtime checks (catches UB that miri might miss).

```bash
# Run tests with extra checks
cargo +nightly careful test

# Run binary with checks
cargo +nightly careful run
```

### miri (Rust undefined behavior detector)

```bash
# Install miri
rustup +nightly component add miri

# Run tests under miri
cargo +nightly miri test

# Run specific test
cargo +nightly miri test test_name
```

**Note:** Miri is slow and doesn't support all operations (FFI, inline asm).

---

## Testing

### Fuzzing (cargo-fuzz)

Find crashes and bugs through randomized input.

```bash
# Initialize fuzzing for a crate
cd crates/spectral-vault
cargo +nightly fuzz init

# Create a fuzz target
cargo +nightly fuzz add decrypt_fuzz

# Run fuzzer
cargo +nightly fuzz run decrypt_fuzz

# Run for limited time
cargo +nightly fuzz run decrypt_fuzz -- -max_total_time=300
```

**Fuzz targets:** `crates/*/fuzz/fuzz_targets/`

### Mutation Testing (cargo-mutants)

Test quality by introducing bugs and checking if tests catch them.

```bash
# Run mutation testing
cargo mutants

# Run on specific crate
cargo mutants -p spectral-vault

# Parallel execution
cargo mutants --jobs 4

# Output report
cargo mutants --output mutants-report
```

**Metrics:**
- Killed = Test caught the mutation (good)
- Survived = Test missed the mutation (bad - improve tests)
- Timeout = Mutation caused infinite loop

### Coverage (cargo-tarpaulin)

Code coverage reporting.

```bash
# Generate HTML report
cargo tarpaulin --out Html --output-dir coverage/

# Generate for SonarQube
cargo tarpaulin --out Xml --output-dir coverage/

# Exclude test code
cargo tarpaulin --ignore-tests

# Include all features
cargo tarpaulin --all-features
```

**Output:** `coverage/tarpaulin-report.html`

### Property-Based Testing (proptest)

Add to `Cargo.toml`:
```toml
[dev-dependencies]
proptest = "1"
```

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn encrypt_decrypt_roundtrip(plaintext in any::<Vec<u8>>()) {
        let key = [0u8; 32];
        let encrypted = encrypt(&key, &plaintext).unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();
        prop_assert_eq!(plaintext, decrypted);
    }
}
```

---

## Performance

### Benchmarking (criterion)

Add to `Cargo.toml`:
```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "vault_benchmarks"
harness = false
```

```rust
// benches/vault_benchmarks.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_encrypt(c: &mut Criterion) {
    let key = [0u8; 32];
    let data = vec![0u8; 1024];

    c.bench_function("encrypt_1kb", |b| {
        b.iter(|| encrypt(&key, &data))
    });
}

criterion_group!(benches, bench_encrypt);
criterion_main!(benches);
```

```bash
# Run benchmarks
cargo bench

# Run specific benchmark
cargo bench -- encrypt
```

**Output:** `target/criterion/report/index.html`

### Profiling (flamegraph)

CPU profiling with flame graphs.

```bash
# Profile release binary
cargo flamegraph --bin spectral

# Profile specific function
cargo flamegraph --bin spectral -- --specific-flag

# Profile benchmarks
cargo flamegraph --bench vault_benchmarks
```

**Output:** `flamegraph.svg`

**Note (WSL2):** May need to run with `sudo` or configure `perf_event_paranoid`:
```bash
echo 1 | sudo tee /proc/sys/kernel/perf_event_paranoid
```

### Heap Profiling (dhat)

Add to `Cargo.toml`:
```toml
[dev-dependencies]
dhat = "0.3"

[features]
dhat-heap = []
```

```rust
#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    // ... your code
}
```

```bash
cargo run --features dhat-heap
# Open dhat-heap.json in https://nnethercote.github.io/dh_view/dh_view.html
```

---

## Binary Security

### checksec

Check binary hardening (RELRO, Stack Canary, NX, PIE, etc.).

```bash
# Install
sudo apt install checksec

# Check binary
checksec --file=target/release/spectral

# JSON output
checksec --file=target/release/spectral --output=json
```

**Expected output for secure binary:**
```
RELRO           STACK CANARY      NX            PIE
Full RELRO      Canary found      NX enabled    PIE enabled
```

### cargo-auditable

Embed dependency info in binary for later auditing.

```bash
# Build with embedded SBOM
cargo auditable build --release

# Audit a built binary
cargo audit bin target/release/spectral
```

---

## Supply Chain Security

### Sigstore (cosign)

Sign releases for supply chain security.

```bash
# Sign a file (uses OIDC - opens browser for auth)
cosign sign-blob spectral-v1.0.0-linux-x86_64.tar.gz \
  --output-signature spectral-v1.0.0-linux-x86_64.tar.gz.sig \
  --output-certificate spectral-v1.0.0-linux-x86_64.tar.gz.crt

# Verify signature
cosign verify-blob spectral-v1.0.0-linux-x86_64.tar.gz \
  --signature spectral-v1.0.0-linux-x86_64.tar.gz.sig \
  --certificate spectral-v1.0.0-linux-x86_64.tar.gz.crt \
  --certificate-identity=evan@example.com \
  --certificate-oidc-issuer=https://github.com/login/oauth

# Sign container image (if using Docker)
cosign sign ghcr.io/user/spectral:v1.0.0
```

**Key benefits:**
- No key management (uses OIDC identity)
- Transparency log (rekor) for audit trail
- Free public infrastructure

---

## Tool Installation Summary

All tools are installed via `setup.sh`. To install manually:

```bash
# Rust tools
cargo install cargo-audit cargo-deny cargo-geiger cargo-careful cargo-fuzz \
  cargo-mutants cargo-tarpaulin flamegraph cargo-auditable

# Rust nightly (needed for some tools)
rustup install nightly
rustup component add miri --toolchain nightly

# System tools
pip install semgrep pre-commit detect-secrets

# Sigstore
curl -sLO https://github.com/sigstore/cosign/releases/download/v2.2.3/cosign-linux-amd64
sudo install -m 0755 cosign-linux-amd64 /usr/local/bin/cosign
```

---

## CI Integration

See `.github/workflows/ci.yml` (to be created) for how these tools are integrated into the CI pipeline.

| Stage | Tools |
|-------|-------|
| **Lint** | clippy, eslint, prettier |
| **Test** | cargo test, vitest |
| **Security** | semgrep, cargo-audit, cargo-deny |
| **Quality** | SonarQube |
| **SBOM** | cargo-cyclonedx → Dependency Track |
| **Release** | cargo-auditable, cosign |
