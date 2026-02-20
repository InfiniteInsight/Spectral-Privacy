# Smart dev script that finds an available port and syncs it between Vite and Tauri
$ErrorActionPreference = "Stop"

# Color output functions
function Write-Success { Write-Host "✓ $args" -ForegroundColor Green }
function Write-Warning { Write-Host "$args" -ForegroundColor Yellow }

# Function to check if port is in use
function Test-Port {
    param([int]$Port)
    $connection = Test-NetConnection -ComputerName localhost -Port $Port -WarningAction SilentlyContinue -InformationLevel Quiet
    return $connection
}

# Find an available port starting from 5737
$PORT = 5737
while (Test-Port -Port $PORT) {
    Write-Warning "Port $PORT is in use, trying next port..."
    $PORT++
}

Write-Success "Found available port: $PORT"

# Backup and update tauri.conf.json with the selected port
$TAURI_CONF = "src-tauri\tauri.conf.json"
Copy-Item $TAURI_CONF "$TAURI_CONF.bak"

# Read the config file
$config = Get-Content $TAURI_CONF -Raw

# Update the devUrl with regex
$config = $config -replace '"devUrl":\s*"http://localhost:\d+"', "`"devUrl`": `"http://localhost:$PORT`""

# Write back to file
Set-Content -Path $TAURI_CONF -Value $config

Write-Success "Updated Tauri config to use port $PORT"

# Cleanup function
$cleanupScript = {
    Write-Host ""
    Write-Warning "Restoring original Tauri config..."
    if (Test-Path "$TAURI_CONF.bak") {
        Move-Item -Path "$TAURI_CONF.bak" -Destination $TAURI_CONF -Force
        Write-Success "Config restored"
    }
}

# Register cleanup on exit
Register-EngineEvent PowerShell.Exiting -Action $cleanupScript | Out-Null
try {
    # Also try to cleanup on Ctrl+C
    [Console]::TreatControlCAsInput = $false
    $null = Register-ObjectEvent -InputObject ([Console]) -EventName CancelKeyPress -Action {
        & $cleanupScript
        [Environment]::Exit(0)
    }
} catch {
    # Silently ignore if we can't register Ctrl+C handler
}

# Set environment variable for Vite
$env:PORT = $PORT

Write-Success "Starting Tauri dev server..."
Write-Host ""

# Run tauri dev
try {
    cargo tauri dev
} finally {
    # Cleanup on exit
    & $cleanupScript
}
