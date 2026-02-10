#!/bin/bash
# Smart dev script that finds an available port and starts Tauri

# Find an available port starting from 5175
PORT=5175
while lsof -Pi :$PORT -sTCP:LISTEN -t >/dev/null 2>&1; do
    echo "Port $PORT is in use, trying next port..."
    PORT=$((PORT + 1))
done

echo "Using port $PORT"

# Update tauri.conf.json with the selected port
sed -i.bak "s|\"devUrl\": \"http://localhost:[0-9]*\"|\"devUrl\": \"http://localhost:$PORT\"|g" src-tauri/tauri.conf.json

# Export port for vite
export PORT=$PORT

# Run tauri dev
cargo tauri dev
