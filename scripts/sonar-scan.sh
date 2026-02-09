#!/usr/bin/env bash
# =============================================================================
# SonarQube Analysis Script for Spectral
# =============================================================================
# Prerequisites:
#   - SonarQube server running at http://192.168.1.220:9000
#   - SONAR_TOKEN environment variable set
#   - Docker installed (uses sonar-scanner-cli image)
#
# Usage:
#   ./scripts/sonar-scan.sh           # Full analysis
#   ./scripts/sonar-scan.sh --pr 123  # PR analysis with PR number
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
success() { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error()   { echo -e "${RED}[ERROR]${NC} $*" >&2; }

# Configuration
SONAR_HOST="${SONAR_HOST_URL:-http://192.168.1.220:9000}"
SONAR_TOKEN="${SONAR_TOKEN:-}"

if [[ -z "$SONAR_TOKEN" ]]; then
  error "SONAR_TOKEN environment variable is not set"
  info "Set it with: export SONAR_TOKEN='sqa_...'"
  exit 1
fi

cd "$PROJECT_ROOT"

# Parse arguments
PR_NUMBER=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --pr)
      PR_NUMBER="$2"
      shift 2
      ;;
    *)
      error "Unknown option: $1"
      exit 1
      ;;
  esac
done

# ---------------------------------------------------------------------------
# Step 1: Generate Rust lint report (clippy)
# ---------------------------------------------------------------------------
info "Generating Clippy report..."
if [[ -d "src-tauri" ]]; then
  cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features \
    --message-format=json 2>/dev/null | \
    jq -s '[.[] | select(.reason == "compiler-message")]' > clippy-report.json || true
  success "Clippy report generated: clippy-report.json"
else
  warn "src-tauri not found, skipping Clippy report"
  echo "[]" > clippy-report.json
fi

# ---------------------------------------------------------------------------
# Step 2: Generate coverage reports (if tests exist)
# ---------------------------------------------------------------------------
info "Checking for test coverage..."
mkdir -p coverage/rust

if [[ -d "src-tauri" ]] && command -v cargo-tarpaulin &>/dev/null; then
  info "Running Rust coverage with tarpaulin..."
  cargo tarpaulin --manifest-path src-tauri/Cargo.toml \
    --out Xml --output-dir coverage/rust \
    --skip-clean --ignore-tests 2>/dev/null || warn "Tarpaulin coverage failed (tests may not exist yet)"
elif [[ -d "src-tauri" ]]; then
  warn "cargo-tarpaulin not installed. Install with: cargo install cargo-tarpaulin"
fi

if [[ -f "package.json" ]] && [[ -d "src" ]]; then
  info "Running frontend coverage..."
  npm run test:coverage 2>/dev/null || warn "Frontend coverage failed (tests may not exist yet)"
fi

# ---------------------------------------------------------------------------
# Step 3: Run SonarQube scanner
# ---------------------------------------------------------------------------
info "Running SonarQube analysis..."

SONAR_ARGS=(
  "-Dsonar.host.url=$SONAR_HOST"
  "-Dsonar.token=$SONAR_TOKEN"
)

# Add PR analysis parameters if provided
if [[ -n "$PR_NUMBER" ]]; then
  SONAR_ARGS+=(
    "-Dsonar.pullrequest.key=$PR_NUMBER"
    "-Dsonar.pullrequest.branch=$(git branch --show-current)"
    "-Dsonar.pullrequest.base=main"
  )
  info "Running PR analysis for PR #$PR_NUMBER"
fi

# Run via Docker
docker run --rm \
  --network=host \
  -e SONAR_HOST_URL="$SONAR_HOST" \
  -e SONAR_TOKEN="$SONAR_TOKEN" \
  -v "$PROJECT_ROOT:/usr/src" \
  -w /usr/src \
  sonarsource/sonar-scanner-cli \
  "${SONAR_ARGS[@]}"

success "SonarQube analysis complete!"
info "View results at: $SONAR_HOST/dashboard?id=spectral"
