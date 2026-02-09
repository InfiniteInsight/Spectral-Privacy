## 9. Security Architecture

### 9.1 Threat Model

| Threat | Mitigation |
|--------|-----------|
| **PII exfiltration via malicious plugin** | WASM sandbox, explicit permission grants, no filesystem access by default |
| **PII leakage to cloud LLM** | PII filter pipeline with tokenization; local-preferred routing |
| **Database theft from disk** | SQLCipher AES-256 + field-level ChaCha20-Poly1305 encryption |
| **Memory scraping** | Zeroize sensitive data on drop; minimize PII residence time in memory |
| **Broker site bot detection** | Fingerprint rotation, human-like timing, rate limiting |
| **Man-in-the-middle on LLM API calls** | TLS certificate pinning for known providers |
| **Supply chain attack** | WASM plugin checksums, dependency auditing via `cargo-audit`, Sigstore signing for releases |
| **Rogue broker definitions** | Broker definitions are data-only TOML; validated against schema; community review |
| **Prompt injection via broker email** | Three-layer defense: pre-processing, locked system prompt, post-processing validation (see Section 7.4) |
| **Auto-reply abuse / infinite loops** | Hard budget caps per thread: max 5 auto-replies, 20 LLM calls, 5000 tokens (see Section 7.5) |
| **PII leakage in broker replies** | Post-processing PII detection on all LLM-generated replies before sending |

### 9.2 PII Handling Rules

```
RULE 1: PII is encrypted at rest, always.
RULE 2: PII is decrypted only in-memory, only when needed, and zeroized immediately after.
RULE 3: PII sent to cloud LLMs must pass through the PII filter (tokenize or redact).
RULE 4: PII sent to local LLMs can bypass the filter (user's hardware, user's risk).
RULE 5: PII is never written to application logs. Audit logs reference record IDs, not values.
RULE 6: Screenshots containing PII are encrypted before storage.
RULE 7: Plugins must declare which PII fields they access; users approve at install time.
RULE 8: Third-party email content is NEVER passed to LLMs without pre-processing sanitization.
RULE 9: LLM-generated outbound replies are ALWAYS checked for PII leakage before sending.
RULE 10: Auto-reply budgets are hard-capped and non-overridable by LLM or external content.
```

### 9.3 Authentication & Key Management

```
Master Password
       │
       ▼
   Argon2id (m=256MB, t=4, p=4)
       │
       ▼
   Master Key (256-bit)
       │
       ├──► SQLCipher DB encryption key
       │
       └──► HKDF derivation
               │
               ├──► PII field encryption key
               ├──► Screenshot encryption key
               └──► API credential encryption key
```

---
