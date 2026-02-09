#!/usr/bin/env bash
# =============================================================================
# Dependency Track SBOM Generation and Upload Script for Spectral
# =============================================================================
# Prerequisites:
#   - Dependency Track server running at http://192.168.1.220:8081
#   - DTRACK_API_KEY environment variable set
#   - cargo-cyclonedx installed: cargo install cargo-cyclonedx
#   - @cyclonedx/cdxgen installed: npm install -g @cyclonedx/cdxgen
#
# Usage:
#   ./scripts/dependency-track.sh              # Generate and upload SBOMs
#   ./scripts/dependency-track.sh --generate   # Only generate SBOMs
#   ./scripts/dependency-track.sh --upload     # Only upload existing SBOMs
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
DTRACK_HOST="${DTRACK_HOST:-http://192.168.1.220:8081}"
DTRACK_API_KEY="${DTRACK_API_KEY:-}"
PROJECT_NAME="spectral"
PROJECT_VERSION="${PROJECT_VERSION:-$(git describe --tags --always 2>/dev/null || echo "0.1.0-dev")}"

cd "$PROJECT_ROOT"

# Parse arguments
DO_GENERATE=true
DO_UPLOAD=true
while [[ $# -gt 0 ]]; do
  case "$1" in
    --generate)
      DO_UPLOAD=false
      shift
      ;;
    --upload)
      DO_GENERATE=false
      shift
      ;;
    *)
      error "Unknown option: $1"
      exit 1
      ;;
  esac
done

# Create output directory
mkdir -p sbom

# ---------------------------------------------------------------------------
# Step 1: Generate SBOMs
# ---------------------------------------------------------------------------
if [[ "$DO_GENERATE" == true ]]; then
  info "Generating Software Bill of Materials (SBOM)..."

  # Rust SBOM via cargo-cyclonedx
  if [[ -f "src-tauri/Cargo.toml" ]]; then
    if command -v cargo-cyclonedx &>/dev/null; then
      info "Generating Rust SBOM..."
      cargo cyclonedx --manifest-path src-tauri/Cargo.toml \
        --format json \
        --output-prefix sbom/rust
      success "Rust SBOM generated: sbom/rust-bom.json"
    else
      warn "cargo-cyclonedx not installed. Install with: cargo install cargo-cyclonedx"
    fi
  fi

  # NPM SBOM via cdxgen or npm sbom
  if [[ -f "package.json" ]]; then
    if command -v cdxgen &>/dev/null; then
      info "Generating NPM SBOM via cdxgen..."
      cdxgen -o sbom/npm-bom.json --spec-version 1.5
      success "NPM SBOM generated: sbom/npm-bom.json"
    elif npm sbom --help &>/dev/null 2>&1; then
      info "Generating NPM SBOM via npm sbom..."
      npm sbom --sbom-format cyclonedx --sbom-type library > sbom/npm-bom.json
      success "NPM SBOM generated: sbom/npm-bom.json"
    else
      warn "No SBOM generator found for NPM. Install: npm install -g @cyclonedx/cdxgen"
    fi
  fi

  # Merge SBOMs if both exist
  if [[ -f "sbom/rust-bom.json" ]] && [[ -f "sbom/npm-bom.json" ]]; then
    info "Both Rust and NPM SBOMs generated. Consider merging for a complete view."
  fi
fi

# ---------------------------------------------------------------------------
# Step 2: Upload to Dependency Track
# ---------------------------------------------------------------------------
if [[ "$DO_UPLOAD" == true ]]; then
  if [[ -z "$DTRACK_API_KEY" ]]; then
    error "DTRACK_API_KEY environment variable is not set"
    info "Get an API key from: $DTRACK_HOST/admin/accessManagement"
    info "Set it with: export DTRACK_API_KEY='...'"
    exit 1
  fi

  info "Uploading SBOMs to Dependency Track..."
  info "  Server: $DTRACK_HOST"
  info "  Project: $PROJECT_NAME"
  info "  Version: $PROJECT_VERSION"

  upload_sbom() {
    local sbom_file="$1"
    local sbom_name="$2"

    if [[ ! -f "$sbom_file" ]]; then
      warn "SBOM file not found: $sbom_file"
      return 1
    fi

    info "Uploading $sbom_name SBOM..."

    # Base64 encode the SBOM
    local sbom_base64
    sbom_base64=$(base64 -w 0 "$sbom_file")

    # Upload via API
    local response
    response=$(curl -s -w "\n%{http_code}" -X PUT \
      "$DTRACK_HOST/api/v1/bom" \
      -H "X-Api-Key: $DTRACK_API_KEY" \
      -H "Content-Type: application/json" \
      -d "{
        \"projectName\": \"$PROJECT_NAME\",
        \"projectVersion\": \"$PROJECT_VERSION\",
        \"autoCreate\": true,
        \"bom\": \"$sbom_base64\"
      }")

    local http_code
    http_code=$(echo "$response" | tail -n1)
    local body
    body=$(echo "$response" | sed '$d')

    if [[ "$http_code" == "200" ]] || [[ "$http_code" == "201" ]]; then
      success "$sbom_name SBOM uploaded successfully"
      local token
      token=$(echo "$body" | jq -r '.token // empty')
      if [[ -n "$token" ]]; then
        info "  Processing token: $token"
      fi
    else
      error "Failed to upload $sbom_name SBOM (HTTP $http_code)"
      error "Response: $body"
      return 1
    fi
  }

  # Upload each SBOM
  [[ -f "sbom/rust-bom.json" ]] && upload_sbom "sbom/rust-bom.json" "Rust"
  [[ -f "sbom/npm-bom.json" ]] && upload_sbom "sbom/npm-bom.json" "NPM"

  success "SBOM upload complete!"
  info "View results at: $DTRACK_HOST/projects"
fi
