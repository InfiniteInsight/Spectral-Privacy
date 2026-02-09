## 15. Dependencies

```toml
# Cargo.toml workspace dependencies (consolidated)
[workspace.dependencies]

# ── Framework & Runtime ─────────────────────────────────────
tauri = { version = "2", features = ["protocol-asset"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# ── Encrypted Storage ───────────────────────────────────────
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio"] }
# Note: SQLCipher integration via sqlx requires building SQLite with
# the SQLITE_HAS_CODEC flag or using the sqlcipher feature
argon2 = "0.5"
chacha20poly1305 = "0.10"
zeroize = { version = "1", features = ["derive"] }

# ── Core Utilities ──────────────────────────────────────────
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
toml = "0.8"
regex = "1"
semver = "1"
thiserror = "2"

# ── Browser Automation ──────────────────────────────────────
chromiumoxide = "0.7"

# ── Plugin System ───────────────────────────────────────────
extism = "1"

# ── Scheduling ──────────────────────────────────────────────
tokio-cron-scheduler = "0.13"

# ── Logging & Tracing ──────────────────────────────────────
tracing = "0.1"
tracing-subscriber = "0.3"

# ── Email ───────────────────────────────────────────────────
imap = "3"                              # IMAP client
mailparse = "0.15"                      # Email parsing
mail-builder = "0.3"                    # For generating opt-out emails
lettre = "0.11"                         # SMTP email sending

# ── Document Parsing ───────────────────────────────────────
zip = "2"                               # For .docx/.xlsx (they're ZIP archives)
pdf-extract = "0.8"                     # PDF text extraction
calamine = "0.26"                       # Excel/spreadsheet reading

# ── PII Detection ──────────────────────────────────────────
aho-corasick = "1"                      # Fast multi-pattern string matching
unicode-segmentation = "1"              # Proper text segmentation

# ── Browser Data ────────────────────────────────────────────
rusqlite = "0.32"                       # Read Chrome/Firefox SQLite databases (read-only)

# ── Filesystem ──────────────────────────────────────────────
walkdir = "2"                           # Recursive directory traversal
ignore = "0.4"                          # .gitignore-style path filtering
notify = "7"                            # Optional: filesystem watcher for real-time scanning

# ── Network Monitoring ─────────────────────────────────────
dns-lookup = "2"                        # DNS resolution
sysinfo = "0.32"                        # Process information
ipnetwork = "0.20"                      # IP range matching

# ── Domain Intelligence ────────────────────────────────────
publicsuffix = "2"                      # Public suffix list for domain matching

# ── Reporting (optional) ───────────────────────────────────
plotters = "0.3"                        # Chart generation for PDF reports
```

---
