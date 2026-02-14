# Browser Detection Implementation

## Goal
Detect and use existing Chrome/Chromium installations before downloading.

**Target:** Everyone (mainstream + privacy users)
**Key Insight:** 65%+ of users already have Chrome - use it!

## Detection Priority Order

1. **Google Chrome** (most common globally)
2. **Microsoft Edge** (Chromium-based, Windows default)
3. **Chromium** (common on Linux)
4. **CHROME_PATH** environment variable (manual override)
5. **Auto-download** Chromium (fallback)

## Implementation Sketch

### Rust Side (crates/spectral-browser/src/engine.rs)

```rust
use std::path::PathBuf;
use std::process::Command;

/// Detect Chrome/Chromium on the system
pub fn detect_chrome_executable() -> Option<PathBuf> {
    // 1. Check CHROME_PATH environment variable (user override)
    if let Ok(path) = std::env::var("CHROME_PATH") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }

    // 2. Common Chrome locations (cross-platform)
    let candidates = if cfg!(target_os = "windows") {
        vec![
            // Google Chrome
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
            // Microsoft Edge (Chromium-based)
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
        ]
    } else {
        // Linux
        vec![
            "/usr/bin/google-chrome",
            "/usr/bin/google-chrome-stable",
            "/usr/bin/chromium",
            "/usr/bin/chromium-browser",
            "/snap/bin/chromium",
            "/usr/bin/microsoft-edge",
        ]
    };

    // 3. Check if any candidate exists
    for path in candidates {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }

    // 4. Try `which` command (Linux/Mac)
    if !cfg!(target_os = "windows") {
        for browser in &["google-chrome", "chromium", "chromium-browser"] {
            if let Ok(output) = Command::new("which").arg(browser).output() {
                if output.status.success() {
                    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    let path = PathBuf::from(path_str);
                    if path.exists() {
                        return Some(path);
                    }
                }
            }
        }
    }

    // 5. Nothing found - need to download
    None
}

/// Get or download Chrome/Chromium
pub async fn get_chrome_executable() -> Result<PathBuf, BrowserError> {
    // Try detection first
    if let Some(path) = detect_chrome_executable() {
        log::info!("Found existing browser at: {:?}", path);
        return Ok(path);
    }

    // Nothing found - download Chromium
    log::info!("No browser found, downloading Chromium...");
    download_chromium().await
}

/// Download Chromium to app data directory
async fn download_chromium() -> Result<PathBuf, BrowserError> {
    use chromiumoxide::fetcher::BrowserFetcher;

    // Download to app data directory
    let app_data = tauri::api::path::app_data_dir(&tauri::Config::default())
        .ok_or_else(|| BrowserError::ChromiumError("No app data dir".into()))?;

    let cache_dir = app_data.join("chromium-cache");
    std::fs::create_dir_all(&cache_dir)?;

    let fetcher = BrowserFetcher::new(cache_dir);
    let info = fetcher.fetch().await
        .map_err(|e| BrowserError::ChromiumError(format!("Download failed: {}", e)))?;

    Ok(info.get_chrome_path())
}

/// Create browser with detected/downloaded Chrome
pub async fn with_fingerprint(fingerprint: FingerprintConfig) -> Result<Self> {
    // Get Chrome executable (detect or download)
    let chrome_path = get_chrome_executable().await?;

    let config = BrowserConfig::builder()
        .chrome_executable(chrome_path)
        .no_sandbox()
        .disable_default_args()
        .arg("--headless")
        .arg("--disable-gpu")
        .arg("--no-first-run")
        .arg("--disable-dev-shm-usage")
        .build()
        .map_err(|e| BrowserError::ChromiumError(e.to_string()))?;

    let (browser, mut handler) = Browser::launch(config).await
        .map_err(|e| BrowserError::ChromiumError(e.to_string()))?;

    // ... rest of browser setup
}
```

### Frontend Side (src/lib/api/scan.ts)

Add a pre-flight check before starting scan:

```typescript
export const scanAPI = {
  /**
   * Check if browser is available (pre-flight check)
   */
  async checkBrowser(): Promise<{
    available: boolean;
    path?: string;
    needsDownload: boolean;
    downloadSize?: number;
  }> {
    return await invoke('check_browser_availability');
  },

  /**
   * Start scan (existing)
   */
  async start(vaultId: string, profileId: string, brokerFilter?: string): Promise<ScanJobStatus> {
    // Could add pre-flight check here
    const browserCheck = await this.checkBrowser();

    if (browserCheck.needsDownload) {
      // Show download dialog
      // ... download with progress
    }

    return await invoke('start_scan', { vaultId, profileId, brokerFilter });
  }
};
```

### UI Flow (src/routes/scan/start/+page.svelte)

Show browser status and download dialog:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { scanAPI } from '$lib/api/scan';

  let browserStatus = $state<{
    available: boolean;
    path?: string;
    needsDownload: boolean;
  } | null>(null);

  let downloading = $state(false);
  let downloadProgress = $state(0);

  onMount(async () => {
    // Check browser availability
    browserStatus = await scanAPI.checkBrowser();
  });

  async function handleStartScan() {
    // If browser needs download, show confirmation
    if (browserStatus?.needsDownload) {
      const confirmed = confirm(
        'No browser found for scanning. Download Chromium (~100MB)?'
      );
      if (!confirmed) return;

      // TODO: Show progress dialog during download
      downloading = true;
      // Download happens automatically in scanAPI.start()
    }

    // Start scan
    const scanId = await scanStore.startScan(vaultStore.currentVaultId, selectedProfileId);
    goto(`/scan/progress/${scanId}`);
  }
</script>

<!-- Browser Status Indicator -->
{#if browserStatus}
  <div class="mb-4 p-3 rounded-lg"
       class:bg-green-50={browserStatus.available}
       class:bg-yellow-50={browserStatus.needsDownload}>
    {#if browserStatus.available}
      <p class="text-sm text-green-700">
        ✓ Browser ready: {browserStatus.path}
      </p>
    {:else}
      <p class="text-sm text-yellow-700">
        ℹ️ No browser found - will download Chromium (~100MB) when you start scan
      </p>
    {/if}
  </div>
{/if}
```

## Benefits

### For Users with Chrome (65%+)
- ✅ Zero download
- ✅ Instant scan capability
- ✅ Uses familiar browser
- ✅ 100MB+ saved in app size

### For Users without Browser
- ✅ Automatic download
- ✅ Progress indicator
- ✅ Cached for future scans
- ✅ Privacy-focused (Chromium, not Chrome)

### For All Users
- ✅ No manual setup
- ✅ Clear status indicators
- ✅ Works offline after first download
- ✅ Best of both worlds

## Testing Priority

1. Test detection on all platforms (Windows/Mac/Linux)
2. Test download fallback
3. Test caching (second scan doesn't re-download)
4. Test manual override via CHROME_PATH
5. Test with snap Chromium (Linux edge case)

## Migration Path

**Current (Phase 1):** Manual install, better error messages
**Next (Phase 2):** Implement this detection + auto-download
**Future (Phase 3):** Optional bundled Chromium for offline installers

This approach maximizes accessibility while minimizing friction.
