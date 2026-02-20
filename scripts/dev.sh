#!/bin/bash
# Smart dev script that finds an available port and syncs it between Vite and Tauri
set -e

# Color output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Find an available port starting from 5737
PORT=5737
while lsof -Pi :$PORT -sTCP:LISTEN -t >/dev/null 2>&1; do
    echo -e "${YELLOW}Port $PORT is in use, trying next port...${NC}"
    PORT=$((PORT + 1))
done

echo -e "${GREEN}✓ Found available port: $PORT${NC}"

# Backup and update tauri.conf.json with the selected port
TAURI_CONF="src-tauri/tauri.conf.json"
cp "$TAURI_CONF" "$TAURI_CONF.bak"

# Update the devUrl in tauri.conf.json
sed -i.tmp "s|\"devUrl\": \"http://localhost:[0-9]*\"|\"devUrl\": \"http://localhost:$PORT\"|g" "$TAURI_CONF"
rm -f "$TAURI_CONF.tmp"

echo -e "${GREEN}✓ Updated Tauri config to use port $PORT${NC}"

# Cleanup function to restore original config on exit
cleanup() {
    echo -e "\n${YELLOW}Restoring original Tauri config...${NC}"
    if [ -f "$TAURI_CONF.bak" ]; then
        mv "$TAURI_CONF.bak" "$TAURI_CONF"
        echo -e "${GREEN}✓ Config restored${NC}"
    fi
}
trap cleanup EXIT INT TERM

# Export port for Vite to use
export PORT=$PORT

echo -e "${GREEN}✓ Starting Tauri dev server...${NC}"
echo ""

# Run tauri dev
cargo tauri dev
