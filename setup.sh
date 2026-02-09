#!/usr/bin/env bash
# =============================================================================
# Spectral — Linux / WSL2 Development Environment Setup
# =============================================================================
# Run this script inside WSL2 (Ubuntu 24.04) to install the full toolchain
# for Tauri v2 + Svelte + Rust development.
#
# Usage:
#   chmod +x setup-linux.sh
#   ./setup-linux.sh
#
# What this script installs:
#   - System build dependencies (build-essential, pkg-config, etc.)
#   - Tauri v2 Linux dependencies (WebKitGTK, appindicator, etc.)
#   - Rust toolchain via rustup (stable channel)
#   - Cargo development tools (cargo-watch, cargo-audit, tauri-cli, etc.)
#   - Node.js 22 LTS via nvm (if not already installed)
#   - WSLg GUI verification tools
#   - VS Code extensions (if code CLI is available)
#
# The script is idempotent — safe to run multiple times.
# =============================================================================

set -euo pipefail

# --- Colors for output ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info()    { echo -e "${BLUE}[INFO]${NC}  $*"; }
success() { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error()   { echo -e "${RED}[ERROR]${NC} $*"; }

header() {
  echo ""
  echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
  echo -e "${BLUE}  $*${NC}"
  echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
  echo ""
}

# --- Pre-flight checks ---
header "Pre-flight Checks"

# Check we're on Linux (or WSL)
if [[ "$(uname -s)" != "Linux" ]]; then
  error "This script must be run on Linux or inside WSL2."
  exit 1
fi
success "Running on Linux"

# Check if WSL
if grep -qi microsoft /proc/version 2>/dev/null; then
  IS_WSL=true
  success "WSL2 detected"
else
  IS_WSL=false
  info "Not running in WSL (native Linux)"
fi

# Check Ubuntu version
if command -v lsb_release &>/dev/null; then
  DISTRO=$(lsb_release -is 2>/dev/null || echo "Unknown")
  VERSION=$(lsb_release -rs 2>/dev/null || echo "Unknown")
  info "Detected: $DISTRO $VERSION"
  if [[ "$DISTRO" == "Ubuntu" && "$VERSION" != "24.04" ]]; then
    warn "This script was written for Ubuntu 24.04. You're on $VERSION."
    warn "Package names may differ. Proceeding anyway..."
  fi
else
  warn "Cannot determine distro. Proceeding with apt-based install..."
fi

# =============================================================================
# SECTION 1: System packages
# =============================================================================
header "Section 1: System Build Dependencies"

info "Updating package lists..."
sudo apt update -y

info "Installing core build tools..."
sudo apt install -y \
  build-essential \
  curl \
  wget \
  file \
  git \
  pkg-config \
  libssl-dev

success "Core build tools installed"

# =============================================================================
# SECTION 2: Tauri v2 dependencies
# =============================================================================
header "Section 2: Tauri v2 System Dependencies"

info "Installing Tauri v2 Linux dependencies..."
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  libxdo-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev

success "Tauri v2 dependencies installed"

# Verify key library is findable by pkg-config
if pkg-config --exists webkit2gtk-4.1; then
  success "webkit2gtk-4.1 found by pkg-config"
else
  error "webkit2gtk-4.1 NOT found by pkg-config. Tauri builds will fail."
  error "Try: sudo apt install --reinstall libwebkit2gtk-4.1-dev"
fi

# =============================================================================
# SECTION 3: WSLg verification (WSL only)
# =============================================================================
if [[ "$IS_WSL" == true ]]; then
  header "Section 3: WSLg GUI Verification"

  info "Installing x11-apps for WSLg testing..."
  sudo apt install -y x11-apps

  # Check DISPLAY is set
  if [[ -z "${DISPLAY:-}" ]]; then
    warn "DISPLAY environment variable is not set."
    info "Adding 'export DISPLAY=:0' to ~/.bashrc..."
    if ! grep -q 'export DISPLAY=:0' ~/.bashrc; then
      echo 'export DISPLAY=:0' >> ~/.bashrc
      success "Added DISPLAY=:0 to ~/.bashrc"
    fi
    export DISPLAY=:0
  else
    success "DISPLAY is set to: $DISPLAY"
  fi

  info ""
  info "To verify WSLg works, run:  xeyes"
  info "A small window with animated eyes should appear on your Windows desktop."
  info "(Close it with Ctrl+C or the window X button)"
  info ""
else
  header "Section 3: GUI Verification (skipped — not WSL)"
fi

# =============================================================================
# SECTION 4: Rust toolchain
# =============================================================================
header "Section 4: Rust Toolchain"

if command -v rustc &>/dev/null; then
  RUST_VER=$(rustc --version)
  success "Rust already installed: $RUST_VER"
  info "Updating to latest stable..."
  rustup update stable
else
  info "Installing Rust via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
  source "$HOME/.cargo/env"
  success "Rust installed: $(rustc --version)"
fi

# Ensure cargo env is in shell profile
if ! grep -q 'cargo/env' ~/.bashrc 2>/dev/null; then
  echo '. "$HOME/.cargo/env"' >> ~/.bashrc
  info "Added cargo env to ~/.bashrc"
fi

# Install rustfmt and clippy components
rustup component add rustfmt clippy
success "rustfmt and clippy installed"

# Show versions
info "  rustc:  $(rustc --version)"
info "  cargo:  $(cargo --version)"
info "  rustup: $(rustup --version 2>/dev/null || echo 'unknown')"

# =============================================================================
# SECTION 5: Cargo development tools
# =============================================================================
header "Section 5: Cargo Development Tools"

install_cargo_tool() {
  local tool_name="$1"
  local crate_name="${2:-$1}"

  if command -v "$tool_name" &>/dev/null; then
    success "$tool_name already installed"
  else
    info "Installing $crate_name..."
    cargo install "$crate_name"
    success "$tool_name installed"
  fi
}

# tauri-cli installs as "cargo-tauri" binary
if cargo tauri --version &>/dev/null 2>&1; then
  success "tauri-cli already installed: $(cargo tauri --version 2>/dev/null)"
else
  info "Installing tauri-cli (this may take a few minutes on first install)..."
  cargo install tauri-cli
  success "tauri-cli installed: $(cargo tauri --version 2>/dev/null)"
fi

install_cargo_tool "cargo-watch" "cargo-watch"
install_cargo_tool "cargo-audit" "cargo-audit"

# cargo-nextest requires --locked flag
if command -v cargo-nextest &>/dev/null; then
  success "cargo-nextest already installed"
else
  info "Installing cargo-nextest..."
  cargo install --locked cargo-nextest
  success "cargo-nextest installed"
fi

# cargo-edit provides `cargo add`, `cargo rm`, `cargo upgrade`
# Note: `cargo add` is built into cargo since 1.62, but cargo-edit adds `cargo upgrade`
if cargo upgrade --version &>/dev/null 2>&1; then
  success "cargo-edit already installed"
else
  info "Installing cargo-edit..."
  cargo install cargo-edit
  success "cargo-edit installed"
fi

# ---------------------------------------------------------------------------
# Additional Security & Quality Tools
# ---------------------------------------------------------------------------

# cargo-deny - license and vulnerability checking
install_cargo_tool "cargo-deny" "cargo-deny"

# cargo-geiger - detect unsafe code usage
install_cargo_tool "cargo-geiger" "cargo-geiger"

# cargo-auditable - embed dependency info in binaries
install_cargo_tool "cargo-auditable" "cargo-auditable"

# cargo-careful - extra runtime checks (requires nightly for some features)
if cargo careful --version &>/dev/null 2>&1; then
  success "cargo-careful already installed"
else
  info "Installing cargo-careful..."
  cargo install cargo-careful
  success "cargo-careful installed"
fi

# cargo-fuzz - fuzzing (requires nightly)
if cargo fuzz --version &>/dev/null 2>&1; then
  success "cargo-fuzz already installed"
else
  info "Installing cargo-fuzz..."
  cargo install cargo-fuzz
  success "cargo-fuzz installed"
fi

# cargo-mutants - mutation testing
install_cargo_tool "cargo-mutants" "cargo-mutants"

# cargo-tarpaulin - code coverage
install_cargo_tool "cargo-tarpaulin" "cargo-tarpaulin"

# criterion - benchmarking (installed as dev dependency, not CLI)

# ---------------------------------------------------------------------------
# Performance Profiling Tools
# ---------------------------------------------------------------------------

# flamegraph - CPU profiling visualization
if cargo flamegraph --version &>/dev/null 2>&1; then
  success "cargo-flamegraph already installed"
else
  info "Installing cargo-flamegraph..."
  # Requires perf on Linux
  if command -v perf &>/dev/null || [[ "$IS_WSL" == true ]]; then
    cargo install flamegraph
    success "cargo-flamegraph installed"
  else
    warn "Installing perf for flamegraph support..."
    sudo apt install -y linux-tools-generic linux-tools-$(uname -r) 2>/dev/null || \
      warn "Could not install perf - flamegraph will have limited functionality"
    cargo install flamegraph
    success "cargo-flamegraph installed"
  fi
fi

# ---------------------------------------------------------------------------
# Supply Chain Security - Sigstore
# ---------------------------------------------------------------------------
info "Installing Sigstore cosign..."
if command -v cosign &>/dev/null; then
  success "cosign already installed: $(cosign version 2>/dev/null | head -1)"
else
  # Install cosign
  COSIGN_VERSION="v2.2.3"
  curl -sLO "https://github.com/sigstore/cosign/releases/download/${COSIGN_VERSION}/cosign-linux-amd64"
  sudo install -o root -g root -m 0755 cosign-linux-amd64 /usr/local/bin/cosign
  rm cosign-linux-amd64
  success "cosign installed: $(cosign version 2>/dev/null | head -1)"
fi

# =============================================================================
# SECTION 6: Node.js
# =============================================================================
header "Section 6: Node.js"

# Source nvm if it exists
export NVM_DIR="${HOME}/.nvm"
if [[ -s "$NVM_DIR/nvm.sh" ]]; then
  source "$NVM_DIR/nvm.sh"
fi

if command -v node &>/dev/null; then
  NODE_VER=$(node --version)
  NODE_MAJOR=$(echo "$NODE_VER" | sed 's/v\([0-9]*\).*/\1/')
  if [[ "$NODE_MAJOR" -ge 20 ]]; then
    success "Node.js already installed: $NODE_VER (meets >=20 requirement)"
  else
    warn "Node.js $NODE_VER is too old. Need v20+."
    info "Installing Node.js 22 via nvm..."
    if ! command -v nvm &>/dev/null; then
      curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
      export NVM_DIR="$HOME/.nvm"
      source "$NVM_DIR/nvm.sh"
    fi
    nvm install 22
    nvm use 22
    nvm alias default 22
    success "Node.js installed: $(node --version)"
  fi
else
  info "Node.js not found. Installing via nvm..."
  if ! command -v nvm &>/dev/null; then
    curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
    export NVM_DIR="$HOME/.nvm"
    source "$NVM_DIR/nvm.sh"
  fi
  nvm install 22
  nvm use 22
  nvm alias default 22
  success "Node.js installed: $(node --version)"
fi

info "  node: $(node --version)"
info "  npm:  $(npm --version)"

# =============================================================================
# SECTION 7: VS Code Extensions
# =============================================================================
header "Section 7: VS Code Extensions"

if command -v code &>/dev/null; then
  info "VS Code CLI detected. Installing extensions..."

  EXTENSIONS=(
    # Required
    "rust-lang.rust-analyzer"
    "svelte.svelte-vscode"
    "tauri-apps.tauri-vscode"
    "tamasfe.even-better-toml"
    "esbenp.prettier-vscode"
    "dbaeumer.vscode-eslint"
    # Recommended
    "bradlc.vscode-tailwindcss"
    "usernamehw.errorlens"
    "eamodio.gitlens"
    "serayuzgur.crates"
    "Gruntfuggly.todo-tree"
    "yzhang.markdown-all-in-one"
  )

  for ext in "${EXTENSIONS[@]}"; do
    if code --list-extensions 2>/dev/null | grep -qi "$(echo "$ext" | cut -d. -f2)"; then
      success "  $ext (already installed)"
    else
      code --install-extension "$ext" --force 2>/dev/null && \
        success "  $ext" || \
        warn "  $ext (install failed — install manually in VS Code)"
    fi
  done
else
  warn "VS Code CLI ('code') not found."
  info "Open VS Code, connect to WSL, then install extensions manually."
  info "Required extensions:"
  info "  - rust-lang.rust-analyzer"
  info "  - svelte.svelte-vscode"
  info "  - tauri-apps.tauri-vscode"
  info "  - tamasfe.even-better-toml"
  info "  - esbenp.prettier-vscode"
  info "  - dbaeumer.vscode-eslint"
  info "Recommended:"
  info "  - bradlc.vscode-tailwindcss"
  info "  - usernamehw.errorlens"
  info "  - eamodio.gitlens"
  info "  - serayuzgur.crates"
fi

# =============================================================================
# SECTION 8: GitHub CLI (optional)
# =============================================================================
header "Section 8: GitHub CLI"

if command -v gh &>/dev/null; then
  success "GitHub CLI already installed: $(gh --version | head -1)"
else
  info "Installing GitHub CLI..."
  (type -p wget >/dev/null || sudo apt install wget -y) \
    && sudo mkdir -p -m 755 /etc/apt/keyrings \
    && out=$(mktemp) \
    && wget -nv -O "$out" https://cli.github.com/packages/githubcli-archive-keyring.gpg \
    && cat "$out" | sudo tee /etc/apt/keyrings/githubcli-archive-keyring.gpg > /dev/null \
    && sudo chmod go+r /etc/apt/keyrings/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null \
    && sudo apt update \
    && sudo apt install gh -y
  success "GitHub CLI installed: $(gh --version | head -1)"
  info ""
  info "Authenticate with:  gh auth login"
fi

# =============================================================================
# SECTION 9: Verification Summary
# =============================================================================
header "Verification Summary"

echo ""
printf "%-24s %s\n" "Component" "Status"
printf "%-24s %s\n" "────────────────────────" "──────────────────────────────"

check_cmd() {
  local label="$1"
  local cmd="$2"
  if eval "$cmd" &>/dev/null 2>&1; then
    local ver
    ver=$(eval "$cmd" 2>/dev/null | head -1)
    printf "${GREEN}%-24s${NC} %s\n" "$label" "✓  $ver"
  else
    printf "${RED}%-24s${NC} %s\n" "$label" "✗  NOT FOUND"
  fi
}

check_cmd "gcc"             "gcc --version"
check_cmd "pkg-config"      "pkg-config --version"
check_cmd "git"             "git --version"
check_cmd "rustc"           "rustc --version"
check_cmd "cargo"           "cargo --version"
check_cmd "cargo-tauri"     "cargo tauri --version"
check_cmd "cargo-watch"     "cargo watch --version"
check_cmd "cargo-audit"     "cargo audit --version"
check_cmd "cargo-nextest"   "cargo nextest --version"
check_cmd "node"            "node --version"
check_cmd "npm"             "npm --version"
check_cmd "gh"              "gh --version"

# Check library
echo ""
if pkg-config --exists webkit2gtk-4.1; then
  printf "${GREEN}%-24s${NC} %s\n" "webkit2gtk-4.1"  "✓  $(pkg-config --modversion webkit2gtk-4.1)"
else
  printf "${RED}%-24s${NC} %s\n" "webkit2gtk-4.1" "✗  NOT FOUND"
fi

if pkg-config --exists openssl; then
  printf "${GREEN}%-24s${NC} %s\n" "openssl"  "✓  $(pkg-config --modversion openssl)"
else
  printf "${RED}%-24s${NC} %s\n" "openssl" "✗  NOT FOUND"
fi

echo ""
success "Linux development environment setup complete!"
echo ""
info "Next steps:"
info "  1. If in WSL, test GUI:  xeyes"
info "  2. Authenticate GitHub:  gh auth login"
info "  3. Clone or create the Spectral project"
info "  4. Run:  cargo tauri dev"
echo ""
if [[ "$IS_WSL" == true ]]; then
  info "NOTE: You may need to restart your terminal or run 'source ~/.bashrc'"
  info "      for all PATH changes to take effect."
fi
echo ""
