# Spectral Code Review Checklist

Use this checklist when reviewing code (manually or via code-review agent).

## Error Handling (patterns.md §1)

- [ ] Library crates use `thiserror`, not `anyhow`
- [ ] No `.unwrap()` in production code (tests are OK)
- [ ] Errors have context at module boundaries (`.map_err()`, `.context()`)
- [ ] `CommandError` used for Tauri commands with user-friendly messages

## Async & Concurrency (patterns.md §2)

- [ ] Long-running operations accept `CancellationToken`
- [ ] Bounded concurrency for parallel operations (`.buffer_unordered(n)`)
- [ ] No unbounded channel usage without justification
- [ ] `tokio::select!` used for cancellable waits

## State Management (patterns.md §3)

- [ ] Tauri state uses `RwLock` or `Mutex` appropriately
- [ ] Frontend uses Svelte 5 runes (`$state`, `$derived`, `$effect`)
- [ ] Stores in `$lib/stores/` for shared state
- [ ] No prop drilling beyond 2 levels

## Testing (patterns.md §4)

- [ ] Unit tests in `#[cfg(test)]` modules
- [ ] Integration tests in `tests/` directory
- [ ] Test fixtures use builder pattern for complex data
- [ ] Tests don't depend on external services

## Logging (patterns.md §5)

- [ ] Uses `tracing` macros, not `println!` or `log`
- [ ] No PII in log messages (use IDs or hashed values)
- [ ] Appropriate log levels (error/warn/info/debug/trace)
- [ ] Functions with side effects have `#[instrument]`

## Configuration (patterns.md §6)

- [ ] Uses `directories::ProjectDirs` for paths
- [ ] Config structs implement `Default`
- [ ] Sensitive values (API keys) stored in vault, not config

## API Design (patterns.md §7)

- [ ] Tauri commands return `Result<T, CommandError>`
- [ ] Commands wrapped in `$lib/api/` on frontend
- [ ] Batch operations for multiple items
- [ ] Pagination for large result sets

## Frontend Components (patterns.md §8)

- [ ] Uses shadcn-svelte components as base
- [ ] TypeScript for prop definitions
- [ ] Loading/error/empty states handled
- [ ] Forms use Zod validation

## Security (patterns.md §9)

- [ ] Sensitive data uses `Zeroizing<T>`
- [ ] External input sanitized before use
- [ ] No SQL string formatting (use `sqlx::query!`)
- [ ] LLM inputs sanitized for prompt injection

## Database (patterns.md §10)

- [ ] Uses `sqlx::query!` macro (compile-time checked)
- [ ] Transactions for multi-step operations
- [ ] Migrations in `crates/spectral-db/migrations/`
- [ ] Indexes for frequently queried columns

## Authentication (patterns.md §11)

- [ ] App launch auth checked before sensitive operations
- [ ] Vault unlock required for PII access
- [ ] Credentials encrypted in vault, not config files
- [ ] Session timeout enforced

## Security-Specific Checks

- [ ] No telemetry or analytics code
- [ ] No external network calls that send user data
- [ ] PII encrypted before storage
- [ ] Third-party content sanitized before LLM

## Code Quality

- [ ] No TODO/FIXME without linked issue
- [ ] Public functions have doc comments
- [ ] No dead code or unused imports
- [ ] Consistent naming conventions
