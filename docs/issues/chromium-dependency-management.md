# Chromium Dependency Management

## Problem

The scan orchestrator requires Chrome/Chromium for browser automation, but currently:
- ❌ Users must manually install it before scanning works
- ❌ No detection or helpful UI when it's missing
- ❌ Not bundled with the application
- ❌ Error shows in console, not in UI
- ❌ Snap-packaged Chromium has flag compatibility issues

This creates a poor first-run experience.

## Known Issues

### Ubuntu 24.04 Snap Chromium Incompatibility

**Problem:** Ubuntu's default `chromium-browser` package is a snap that runs in a confined environment. It doesn't support all Chrome flags used by chromiumoxide, specifically:
- `--disable-background-networking`
- Other Chromium automation flags

**Error:** `Browser process exited with status ExitStatus(unix_wait_status(16384)) before websocket URL could be resolved, stderr: BrowserStderr("error: unknown flag 'disable-background-networking'")`

**Solution:** Install non-snap Chromium from PPA:
```bash
# Remove snap Chromium
sudo snap remove chromium
sudo apt-get remove -y chromium-browser

# Install from Savoury1 PPA (provides native .deb)
sudo add-apt-repository -y ppa:savoury1/chromium
sudo apt-get update
sudo apt-get install -y chromium-browser chromium-chromedriver

# Verify
chromium-browser --version
```

**Code Fix:** Added `disable_default_args()` to browser config to use minimal flag set compatible with snap Chromium. However, PPA version is still recommended for best compatibility.

## Current State

**Browser Engine:** Uses `chromiumoxide` crate which launches a Chrome/Chromium process
**Error Handling:** Backend shows error, but UI just shows generic "Failed to start scan"
**User Experience:** Scan fails silently until user checks console logs

## Proposed Solutions

### Option 1: Bundle Chromium (Recommended for Desktop App)

**Approach:** Use Tauri's sidecar feature to bundle a headless Chromium binary with the app.

**Pros:**
- No external dependencies
- Works offline
- Guaranteed compatibility
- Users never see missing browser errors

**Cons:**
- Increases app download size (~100-150MB)
- Need to bundle per-platform (Linux, macOS, Windows)
- Updates require new app release

**Implementation:**
```toml
# src-tauri/tauri.conf.json
{
  "bundle": {
    "resources": {
      "chromium": "path/to/chromium-${platform}"
    }
  }
}
```

### Option 2: Auto-Download on First Use

**Approach:** Download Chromium programmatically when first scan is initiated.

**Pros:**
- Smaller initial download
- Can update browser independently
- User chooses when to download

**Cons:**
- Requires internet connection
- Download could fail
- Need storage for browser (~100MB)
- Complex error handling

**Implementation:**
- Use `chromiumoxide`'s auto-download feature
- Or use `chrome-downloader` crate
- Show progress dialog during download
- Cache in app data directory

### Option 3: Detect System Browser + Fallback

**Approach:** Try to use system Chrome/Chromium, fall back to download if missing.

**Pros:**
- Uses existing browser if available
- Smaller app size
- Flexible

**Cons:**
- Complex detection logic
- Version compatibility issues
- Still needs download fallback

**Implementation:**
```rust
async fn get_browser() -> Result<Browser> {
    // 1. Try CHROME_PATH env var
    // 2. Try common install locations
    // 3. Offer to download if not found
    // 4. Show UI prompt with options
}
```

### Option 4: Hybrid Approach (Best UX)

**Approach:**
1. Check for system Chrome/Chromium first
2. If not found, show UI dialog:
   - "Chrome/Chromium required for scanning"
   - [Download Bundled Browser] (downloads minimal Chromium)
   - [Use System Browser] (opens instructions)
   - [Cancel]
3. Cache downloaded browser in app data dir
4. Remember user's choice

**Pros:**
- Best of both worlds
- Small initial download
- Works offline after first download
- Clear user communication

**Cons:**
- Most complex to implement
- Need UI for download progress
- Need storage management

## UI/UX Improvements Needed

Regardless of solution, we need:

1. **Pre-flight Check**
   - Detect browser availability before starting scan
   - Show friendly error in UI (not just console)

2. **Setup Wizard**
   - First-time setup screen
   - Check dependencies
   - Download/install missing components
   - Test browser connection

3. **Settings Page**
   - Browser path configuration
   - "Test Browser Connection" button
   - Clear cache/re-download option

4. **Better Error Messages**
   - "Browser not found" → Show install instructions in UI
   - Link to troubleshooting docs
   - Retry button after installation

## Recommended Implementation Plan

**Target Audience:** Everyone (mainstream users + privacy-conscious users)
**Strategy:** Detect existing browser → Use it → Fall back to bundled/download

### Phase 1: Immediate (Next PR)
- [ ] Add browser availability check before scan starts
- [ ] Show error in UI (not just console) with install instructions
- [ ] Add "Test Browser" button to settings/dashboard
- [ ] Document Chromium requirement in README

### Phase 2: Smart Detection (Next Release) - **PRIORITY**
- [ ] Implement tiered browser detection:
  - [ ] Check for Google Chrome (most common)
  - [ ] Check for Microsoft Edge (Chromium-based, Windows default)
  - [ ] Check for Chromium (Linux users)
  - [ ] Check CHROME_PATH environment variable
- [ ] Use detected browser if found (65%+ of users = zero download)
- [ ] Fall back to auto-download minimal Chromium if nothing found
- [ ] Show progress dialog during download
- [ ] Cache downloaded browser in app data directory
- [ ] Add setup wizard for first-time users

### Phase 3: Optional Bundling (Future Release)
- [ ] Optionally bundle Chromium as Tauri sidecar for offline installers
- [ ] Make browser path manually configurable in settings
- [ ] Add browser version compatibility checks
- [ ] Add browser update mechanism for cached downloads

**Key Insight:** Most users already have Chrome installed. Don't force them to download another 100MB browser. Only download when actually needed.

## Related Tools/Crates

- **chrome-downloader**: Auto-download Chromium
- **tauri-plugin-shell**: For bundling sidecars
- **chromiumoxide**: Current browser automation (supports auto-download)
- **fantoccini**: Alternative WebDriver approach (requires geckodriver/chromedriver)

## Technical Notes

**Chromiumoxide Auto-Download:**
```rust
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::fetcher::BrowserFetcher;

// Download Chromium if needed
let fetcher = BrowserFetcher::new();
let chrome = fetcher.fetch().await?;

let config = BrowserConfig::builder()
    .chrome_executable(chrome.get_chrome_path())
    .build()?;
```

**Tauri Sidecar:**
```rust
use tauri::api::process::{Command, CommandEvent};

// Launch bundled Chromium
let (mut rx, _child) = Command::new_sidecar("chromium")
    .expect("failed to create sidecar")
    .spawn()
    .expect("failed to spawn sidecar");
```

## User Impact

**Current:** Scan fails with cryptic error, users blocked
**After Phase 1:** Clear error with instructions, users can self-fix
**After Phase 2:** Automatic download, users can scan immediately
**After Phase 3:** Zero configuration, works out of the box

## Priority

**HIGH** - This blocks the core scanning functionality and creates a terrible first-run experience.

## See Also

- Issue: Profile loading race condition (#0503937)
- Docs: [Browser Automation Architecture](../architecture/browser-automation.md) (to be created)
- Tauri Sidecar: https://tauri.app/v1/guides/building/sidecar
