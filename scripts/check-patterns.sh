#!/usr/bin/env bash
# =============================================================================
# Pattern Compliance Checker for Spectral
# =============================================================================
# Validates that code follows patterns defined in patterns.md
#
# Usage:
#   ./scripts/check-patterns.sh           # Check all files
#   ./scripts/check-patterns.sh --staged  # Check only staged files
#   ./scripts/check-patterns.sh src/      # Check specific path
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info()    { echo -e "${BLUE}[INFO]${NC}  $*"; }
success() { echo -e "${GREEN}[PASS]${NC}  $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error()   { echo -e "${RED}[FAIL]${NC}  $*"; }

cd "$PROJECT_ROOT"

# Parse arguments
CHECK_PATH="."
STAGED_ONLY=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --staged)
      STAGED_ONLY=true
      shift
      ;;
    *)
      CHECK_PATH="$1"
      shift
      ;;
  esac
done

ERRORS=0
WARNINGS=0

# ---------------------------------------------------------------------------
# Check 1: Semgrep pattern rules
# ---------------------------------------------------------------------------
info "Running Semgrep pattern checks..."

if command -v semgrep &>/dev/null; then
  SEMGREP_ARGS=(
    --config=.semgrep/spectral-rules.yaml
    --config=.semgrep/pattern-enforcement.yaml
    --quiet
  )

  if [[ "$STAGED_ONLY" == true ]]; then
    # Get staged files
    STAGED_FILES=$(git diff --cached --name-only --diff-filter=ACMR | grep -E '\.(rs|ts|tsx|js|svelte)$' || true)
    if [[ -n "$STAGED_FILES" ]]; then
      echo "$STAGED_FILES" | xargs semgrep "${SEMGREP_ARGS[@]}" && success "Semgrep: No issues" || ((ERRORS++))
    else
      info "No staged files to check"
    fi
  else
    semgrep "${SEMGREP_ARGS[@]}" "$CHECK_PATH" && success "Semgrep: No issues" || ((ERRORS++))
  fi
else
  warn "Semgrep not installed. Install with: pip install semgrep"
  ((WARNINGS++))
fi

# ---------------------------------------------------------------------------
# Check 2: Rust-specific patterns
# ---------------------------------------------------------------------------
info "Checking Rust patterns..."

check_rust_patterns() {
  local file="$1"

  # Check for println! in non-test files
  if ! echo "$file" | grep -qE '(test|bench)'; then
    if grep -n 'println!' "$file" 2>/dev/null | grep -v '^\s*//' | grep -v '#\[cfg(test)\]' | head -1; then
      error "$file: Found println! - use tracing macros instead"
      return 1
    fi
  fi

  # Check for anyhow in library crates
  if echo "$file" | grep -q '^crates/'; then
    if grep -n 'use anyhow' "$file" 2>/dev/null | head -1; then
      error "$file: Found anyhow in library crate - use thiserror instead"
      return 1
    fi
  fi

  return 0
}

if [[ "$STAGED_ONLY" == true ]]; then
  RUST_FILES=$(git diff --cached --name-only --diff-filter=ACMR | grep '\.rs$' || true)
else
  RUST_FILES=$(find "$CHECK_PATH" -name "*.rs" -not -path "*/target/*" 2>/dev/null || true)
fi

RUST_ERRORS=0
for file in $RUST_FILES; do
  if [[ -f "$file" ]]; then
    check_rust_patterns "$file" || ((RUST_ERRORS++))
  fi
done

if [[ $RUST_ERRORS -eq 0 ]]; then
  success "Rust patterns: No issues"
else
  ((ERRORS += RUST_ERRORS))
fi

# ---------------------------------------------------------------------------
# Check 3: Frontend patterns
# ---------------------------------------------------------------------------
info "Checking frontend patterns..."

check_frontend_patterns() {
  local file="$1"

  # Check for direct invoke calls outside $lib/api/
  if echo "$file" | grep -qE 'src/(routes|lib/components)'; then
    if grep -n "invoke('" "$file" 2>/dev/null | grep -v 'from.*\$lib/api' | head -1; then
      warn "$file: Direct invoke() call - consider wrapping in \$lib/api/ module"
      return 1
    fi
  fi

  return 0
}

if [[ "$STAGED_ONLY" == true ]]; then
  TS_FILES=$(git diff --cached --name-only --diff-filter=ACMR | grep -E '\.(ts|tsx|svelte)$' || true)
else
  TS_FILES=$(find "$CHECK_PATH" -name "*.ts" -o -name "*.tsx" -o -name "*.svelte" 2>/dev/null | grep -v node_modules || true)
fi

TS_WARNINGS=0
for file in $TS_FILES; do
  if [[ -f "$file" ]]; then
    check_frontend_patterns "$file" || ((TS_WARNINGS++))
  fi
done

if [[ $TS_WARNINGS -eq 0 ]]; then
  success "Frontend patterns: No issues"
else
  ((WARNINGS += TS_WARNINGS))
fi

# ---------------------------------------------------------------------------
# Check 4: Security patterns
# ---------------------------------------------------------------------------
info "Checking security patterns..."

# Check for potential PII in logs
PII_PATTERNS='(email|password|ssn|address|phone|name)\s*[=:]'
if [[ "$STAGED_ONLY" == true ]]; then
  PII_ISSUES=$(git diff --cached -U0 | grep -E "^\+" | grep -iE "(tracing::|log::).*(${PII_PATTERNS})" || true)
else
  PII_ISSUES=$(grep -rn -E "(tracing::|log::).*(${PII_PATTERNS})" --include="*.rs" "$CHECK_PATH" 2>/dev/null | grep -v target || true)
fi

if [[ -n "$PII_ISSUES" ]]; then
  warn "Potential PII in logs detected:"
  echo "$PII_ISSUES" | head -5
  ((WARNINGS++))
else
  success "Security patterns: No PII in logs detected"
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo ""
echo "═══════════════════════════════════════════════════════════════"
if [[ $ERRORS -eq 0 && $WARNINGS -eq 0 ]]; then
  success "All pattern checks passed!"
  exit 0
elif [[ $ERRORS -eq 0 ]]; then
  warn "Pattern checks completed with $WARNINGS warning(s)"
  exit 0
else
  error "Pattern checks failed with $ERRORS error(s) and $WARNINGS warning(s)"
  exit 1
fi
