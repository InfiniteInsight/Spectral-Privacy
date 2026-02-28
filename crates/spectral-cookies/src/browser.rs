//! Browser detection and cookie database location.

use crate::error::{CookieError, Result};
use std::path::PathBuf;

/// Supported browser types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BrowserType {
    Chrome,
    Firefox,
    Safari,
    Edge,
    Brave,
    Other,
}

impl BrowserType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BrowserType::Chrome => "Chrome",
            BrowserType::Firefox => "Firefox",
            BrowserType::Safari => "Safari",
            BrowserType::Edge => "Edge",
            BrowserType::Brave => "Brave",
            BrowserType::Other => "Other",
        }
    }
}

impl std::fmt::Display for BrowserType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for BrowserType {
    type Err = CookieError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "chrome" => Ok(BrowserType::Chrome),
            "firefox" => Ok(BrowserType::Firefox),
            "safari" => Ok(BrowserType::Safari),
            "edge" => Ok(BrowserType::Edge),
            "brave" => Ok(BrowserType::Brave),
            _ => Ok(BrowserType::Other),
        }
    }
}

/// Browser profile information.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BrowserProfile {
    pub browser_type: BrowserType,
    pub profile_name: String,
    pub cookie_db_path: PathBuf,
    pub is_default: bool,
}

/// Browser detection and database location.
pub struct Browser;

impl Browser {
    /// Detect all installed browsers and their profiles.
    pub fn detect_installed() -> Result<Vec<BrowserProfile>> {
        let mut profiles = Vec::new();

        // Chrome
        match Self::detect_chrome() {
            Ok(chrome_profiles) => {
                tracing::info!("Found {} Chrome profiles", chrome_profiles.len());
                profiles.extend(chrome_profiles);
            }
            Err(e) => {
                tracing::warn!("Chrome detection failed: {}", e);
            }
        }

        // Firefox
        match Self::detect_firefox() {
            Ok(firefox_profiles) => {
                tracing::info!("Found {} Firefox profiles", firefox_profiles.len());
                profiles.extend(firefox_profiles);
            }
            Err(e) => {
                tracing::warn!("Firefox detection failed: {}", e);
            }
        }

        // Safari (macOS only)
        #[cfg(target_os = "macos")]
        match Self::detect_safari() {
            Ok(safari_profile) => {
                tracing::info!("Found Safari profile");
                profiles.push(safari_profile);
            }
            Err(e) => {
                tracing::warn!("Safari detection failed: {}", e);
            }
        }

        // Edge
        match Self::detect_edge() {
            Ok(edge_profiles) => {
                tracing::info!("Found {} Edge profiles", edge_profiles.len());
                profiles.extend(edge_profiles);
            }
            Err(e) => {
                tracing::warn!("Edge detection failed: {}", e);
            }
        }

        // Brave
        match Self::detect_brave() {
            Ok(brave_profiles) => {
                tracing::info!("Found {} Brave profiles", brave_profiles.len());
                profiles.extend(brave_profiles);
            }
            Err(e) => {
                tracing::warn!("Brave detection failed: {}", e);
            }
        }

        // Firefox-based browsers (Zen, Floorp, Librewolf, Waterfox, etc.)
        match Self::detect_firefox_forks() {
            Ok(fork_profiles) => {
                tracing::info!(
                    "Found {} Firefox-based browser profiles",
                    fork_profiles.len()
                );
                profiles.extend(fork_profiles);
            }
            Err(e) => {
                tracing::warn!("Firefox fork detection failed: {}", e);
            }
        }

        tracing::info!("Total browser profiles detected: {}", profiles.len());
        Ok(profiles)
    }

    /// Detect Chrome installations and profiles.
    fn detect_chrome() -> Result<Vec<BrowserProfile>> {
        let mut profiles = Vec::new();
        let base_path = Self::chrome_base_path()?;

        if !base_path.exists() {
            tracing::debug!("Chrome base path does not exist: {}", base_path.display());
            return Ok(profiles);
        }

        tracing::debug!("Chrome base path exists: {}", base_path.display());

        // Default profile - check both old and new Chrome cookie locations
        let default_profile_path = base_path.join("Default");
        tracing::debug!(
            "Checking Default profile at: {}",
            default_profile_path.display()
        );

        if default_profile_path.exists() {
            tracing::debug!("Default profile directory exists");

            // Try new location first (Chrome 96+): Default/Network/Cookies
            let new_cookies_path = default_profile_path.join("Network").join("Cookies");
            tracing::debug!(
                "Checking new cookie path: {} [exists: {}]",
                new_cookies_path.display(),
                new_cookies_path.exists()
            );

            // Try old location: Default/Cookies
            let old_cookies_path = default_profile_path.join("Cookies");
            tracing::debug!(
                "Checking old cookie path: {} [exists: {}]",
                old_cookies_path.display(),
                old_cookies_path.exists()
            );

            let cookies_path = if new_cookies_path.exists() {
                tracing::info!("Found Chrome Default profile (new location)");
                new_cookies_path
            } else if old_cookies_path.exists() {
                tracing::info!("Found Chrome Default profile (old location)");
                old_cookies_path
            } else {
                tracing::warn!("Chrome Default profile directory exists but no Cookies file found");
                tracing::debug!(
                    "Expected at: {} or {}",
                    new_cookies_path.display(),
                    old_cookies_path.display()
                );
                return Ok(profiles);
            };

            profiles.push(BrowserProfile {
                browser_type: BrowserType::Chrome,
                profile_name: "Default".to_string(),
                cookie_db_path: cookies_path,
                is_default: true,
            });
        } else {
            tracing::debug!(
                "Default profile directory does not exist at: {}",
                default_profile_path.display()
            );
        }

        // Additional profiles (Profile 1, Profile 2, etc.)
        for entry in std::fs::read_dir(&base_path).map_err(CookieError::IoError)? {
            let entry = entry.map_err(CookieError::IoError)?;
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if name.starts_with("Profile ") {
                tracing::debug!("Checking profile: {}", name);

                // Try new location first (Chrome 96+)
                let new_cookies_path = path.join("Network").join("Cookies");
                let old_cookies_path = path.join("Cookies");

                let cookies_path = if new_cookies_path.exists() {
                    tracing::info!("Found Chrome {} (new location)", name);
                    new_cookies_path
                } else if old_cookies_path.exists() {
                    tracing::info!("Found Chrome {} (old location)", name);
                    old_cookies_path
                } else {
                    tracing::debug!("No Cookies file found for {}", name);
                    continue;
                };

                profiles.push(BrowserProfile {
                    browser_type: BrowserType::Chrome,
                    profile_name: name.to_string(),
                    cookie_db_path: cookies_path,
                    is_default: false,
                });
            }
        }

        Ok(profiles)
    }

    /// Detect Firefox installations and profiles.
    fn detect_firefox() -> Result<Vec<BrowserProfile>> {
        let mut profiles = Vec::new();
        let base_path = Self::firefox_base_path()?;

        if !base_path.exists() {
            tracing::debug!("Firefox base path does not exist: {}", base_path.display());
            return Ok(profiles);
        }

        tracing::debug!("Firefox base path exists: {}", base_path.display());

        // Firefox stores cookies in cookies.sqlite in each profile directory
        for entry in std::fs::read_dir(&base_path).map_err(CookieError::IoError)? {
            let entry = entry.map_err(CookieError::IoError)?;
            let path = entry.path();

            if path.is_dir() {
                let cookies_path = path.join("cookies.sqlite");
                if cookies_path.exists() {
                    let profile_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown")
                        .to_string();

                    profiles.push(BrowserProfile {
                        browser_type: BrowserType::Firefox,
                        profile_name: profile_name.clone(),
                        cookie_db_path: cookies_path,
                        is_default: profile_name.contains("default"),
                    });
                }
            }
        }

        Ok(profiles)
    }

    /// Detect Firefox-based browsers (Zen, Floorp, Librewolf, Waterfox, etc.).
    fn detect_firefox_forks() -> Result<Vec<BrowserProfile>> {
        let mut profiles = Vec::new();

        // Common Firefox-based browsers
        let fork_names = vec![
            ("zen", "Zen"),
            ("floorp", "Floorp"),
            ("librewolf", "LibreWolf"),
            ("Waterfox", "Waterfox"),
            ("palemoon", "Pale Moon"),
        ];

        for (dir_name, display_name) in fork_names {
            match Self::detect_firefox_fork(dir_name, display_name) {
                Ok(fork_profiles) => {
                    if !fork_profiles.is_empty() {
                        tracing::info!("Found {} {} profiles", fork_profiles.len(), display_name);
                        profiles.extend(fork_profiles);
                    }
                }
                Err(e) => {
                    tracing::debug!("{} detection failed: {}", display_name, e);
                }
            }
        }

        Ok(profiles)
    }

    /// Detect a specific Firefox-based browser.
    fn detect_firefox_fork(dir_name: &str, display_name: &str) -> Result<Vec<BrowserProfile>> {
        let mut profiles = Vec::new();
        let home = dirs::home_dir()
            .ok_or_else(|| CookieError::BrowserNotFound(display_name.to_string()))?;

        #[cfg(target_os = "macos")]
        let base_path = home.join(format!("Library/Application Support/{}", dir_name));

        #[cfg(target_os = "linux")]
        let base_path = home.join(format!(".{}", dir_name));

        #[cfg(target_os = "windows")]
        let base_path = home.join(format!("AppData/Roaming/{}", dir_name));

        if !base_path.exists() {
            tracing::debug!(
                "{} base path does not exist: {}",
                display_name,
                base_path.display()
            );
            return Ok(profiles);
        }

        tracing::info!(
            "Found {} installation at: {}",
            display_name,
            base_path.display()
        );

        // Check for Profiles subdirectory (like Firefox)
        let profiles_dir = base_path.join("Profiles");
        let search_dir = if profiles_dir.exists() {
            tracing::debug!("{} uses Profiles subdirectory", display_name);
            profiles_dir
        } else {
            tracing::debug!("{} stores profiles in base directory", display_name);
            base_path
        };

        // Firefox-based browsers store cookies in cookies.sqlite in each profile directory
        for entry in std::fs::read_dir(&search_dir).map_err(CookieError::IoError)? {
            let entry = entry.map_err(CookieError::IoError)?;
            let path = entry.path();

            if path.is_dir() {
                let cookies_path = path.join("cookies.sqlite");
                if cookies_path.exists() {
                    let profile_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown")
                        .to_string();

                    tracing::info!("Found {} profile: {}", display_name, profile_name);

                    profiles.push(BrowserProfile {
                        browser_type: BrowserType::Firefox, // Use Firefox type for compatibility
                        profile_name: format!("{} ({})", display_name, profile_name),
                        cookie_db_path: cookies_path,
                        is_default: profile_name.contains("default"),
                    });
                }
            }
        }

        Ok(profiles)
    }

    /// Detect Safari installation (macOS only).
    #[cfg(target_os = "macos")]
    fn detect_safari() -> Result<BrowserProfile> {
        let cookies_path = dirs::home_dir()
            .ok_or_else(|| CookieError::BrowserNotFound("Safari".to_string()))?
            .join("Library/Cookies/Cookies.binarycookies");

        if !cookies_path.exists() {
            return Err(CookieError::BrowserNotFound("Safari".to_string()));
        }

        Ok(BrowserProfile {
            browser_type: BrowserType::Safari,
            profile_name: "Default".to_string(),
            cookie_db_path: cookies_path,
            is_default: true,
        })
    }

    /// Log contents of a directory for debugging purposes.
    fn log_directory_contents(path: &std::path::Path, label: &str) {
        if let Ok(entries) = std::fs::read_dir(path) {
            tracing::info!("Contents of {}:", label);
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    let prefix = if file_type.is_dir() {
                        "[DIR] "
                    } else {
                        "[FILE]"
                    };
                    tracing::info!("  {} {}", prefix, name_str);
                }
            }
        } else {
            tracing::error!("Failed to read directory: {}", path.display());
        }
    }

    /// Find cookie database path for Chromium-based browsers (Chrome, Edge, Brave).
    /// Tries new location (Browser/Network/Cookies) first, falls back to old location (Browser/Cookies).
    fn find_chromium_cookie_path(profile_path: &std::path::Path) -> Option<PathBuf> {
        let new_cookies_path = profile_path.join("Network").join("Cookies");
        let old_cookies_path = profile_path.join("Cookies");

        tracing::info!(
            "Checking new cookie path: {} [exists: {}]",
            new_cookies_path.display(),
            new_cookies_path.exists()
        );
        tracing::info!(
            "Checking old cookie path: {} [exists: {}]",
            old_cookies_path.display(),
            old_cookies_path.exists()
        );

        if new_cookies_path.exists() {
            tracing::info!("✓ Found profile (new location)");
            Some(new_cookies_path)
        } else if old_cookies_path.exists() {
            tracing::info!("✓ Found profile (old location)");
            Some(old_cookies_path)
        } else {
            tracing::error!("✗ Profile directory exists but no Cookies file found");
            tracing::error!(
                "Expected at: {} or {}",
                new_cookies_path.display(),
                old_cookies_path.display()
            );
            None
        }
    }

    /// Detect Edge installations and profiles.
    fn detect_edge() -> Result<Vec<BrowserProfile>> {
        let mut profiles = Vec::new();
        let base_path = Self::edge_base_path()?;

        tracing::info!("=== Edge Detection Debug ===");
        tracing::info!("Edge base path: {}", base_path.display());
        tracing::info!("Edge base path exists: {}", base_path.exists());

        if !base_path.exists() {
            tracing::warn!(
                "Edge base path does not exist - Edge not installed or in non-standard location"
            );
            return Ok(profiles);
        }

        Self::log_directory_contents(&base_path, "Edge User Data directory");

        let default_profile_path = base_path.join("Default");
        tracing::info!(
            "Checking Default profile at: {}",
            default_profile_path.display()
        );
        tracing::info!("Default profile exists: {}", default_profile_path.exists());

        if !default_profile_path.exists() {
            tracing::debug!(
                "Default profile directory does not exist at: {}",
                default_profile_path.display()
            );
            return Ok(profiles);
        }

        Self::log_directory_contents(&default_profile_path, "Edge Default profile");

        if let Some(cookies_path) = Self::find_chromium_cookie_path(&default_profile_path) {
            profiles.push(BrowserProfile {
                browser_type: BrowserType::Edge,
                profile_name: "Default".to_string(),
                cookie_db_path: cookies_path,
                is_default: true,
            });
        }

        Ok(profiles)
    }

    /// Detect Brave installations and profiles.
    fn detect_brave() -> Result<Vec<BrowserProfile>> {
        let mut profiles = Vec::new();
        let base_path = Self::brave_base_path()?;

        if !base_path.exists() {
            tracing::debug!("Brave base path does not exist: {}", base_path.display());
            return Ok(profiles);
        }

        tracing::debug!("Brave base path exists: {}", base_path.display());

        let default_profile_path = base_path.join("Default");

        if !default_profile_path.exists() {
            return Ok(profiles);
        }

        if let Some(cookies_path) = Self::find_chromium_cookie_path(&default_profile_path) {
            profiles.push(BrowserProfile {
                browser_type: BrowserType::Brave,
                profile_name: "Default".to_string(),
                cookie_db_path: cookies_path,
                is_default: true,
            });
        }

        Ok(profiles)
    }

    /// Get Chrome base path for current OS.
    fn chrome_base_path() -> Result<PathBuf> {
        let home =
            dirs::home_dir().ok_or_else(|| CookieError::BrowserNotFound("Chrome".to_string()))?;

        #[cfg(target_os = "macos")]
        let path = home.join("Library/Application Support/Google/Chrome");

        #[cfg(target_os = "linux")]
        let path = home.join(".config/google-chrome");

        #[cfg(target_os = "windows")]
        let path = home.join("AppData/Local/Google/Chrome/User Data");

        tracing::debug!("Chrome base path: {}", path.display());
        Ok(path)
    }

    /// Get Firefox base path for current OS.
    fn firefox_base_path() -> Result<PathBuf> {
        let home =
            dirs::home_dir().ok_or_else(|| CookieError::BrowserNotFound("Firefox".to_string()))?;

        #[cfg(target_os = "macos")]
        let path = home.join("Library/Application Support/Firefox/Profiles");

        #[cfg(target_os = "linux")]
        let path = home.join(".mozilla/firefox");

        #[cfg(target_os = "windows")]
        let path = home.join("AppData/Roaming/Mozilla/Firefox/Profiles");

        tracing::debug!("Firefox base path: {}", path.display());
        Ok(path)
    }

    /// Get Edge base path for current OS.
    fn edge_base_path() -> Result<PathBuf> {
        let home =
            dirs::home_dir().ok_or_else(|| CookieError::BrowserNotFound("Edge".to_string()))?;

        #[cfg(target_os = "macos")]
        let path = home.join("Library/Application Support/Microsoft Edge");

        #[cfg(target_os = "linux")]
        let path = home.join(".config/microsoft-edge");

        #[cfg(target_os = "windows")]
        let path = home.join("AppData/Local/Microsoft/Edge/User Data");

        tracing::debug!("Edge base path: {}", path.display());
        Ok(path)
    }

    /// Get Brave base path for current OS.
    fn brave_base_path() -> Result<PathBuf> {
        let home =
            dirs::home_dir().ok_or_else(|| CookieError::BrowserNotFound("Brave".to_string()))?;

        #[cfg(target_os = "macos")]
        let path = home.join("Library/Application Support/BraveSoftware/Brave-Browser");

        #[cfg(target_os = "linux")]
        let path = home.join(".config/BraveSoftware/Brave-Browser");

        #[cfg(target_os = "windows")]
        let path = home.join("AppData/Local/BraveSoftware/Brave-Browser/User Data");

        Ok(path)
    }

    /// Check if a browser is currently running (would lock the database).
    pub fn is_browser_running(browser_type: BrowserType) -> bool {
        #[cfg(target_os = "macos")]
        {
            let process_name = match browser_type {
                BrowserType::Chrome => "Google Chrome",
                BrowserType::Firefox => "Firefox",
                BrowserType::Safari => "Safari",
                BrowserType::Edge => "Microsoft Edge",
                BrowserType::Brave => "Brave Browser",
                BrowserType::Other => return false,
            };

            std::process::Command::new("pgrep")
                .arg("-x")
                .arg(process_name)
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        }

        #[cfg(target_os = "linux")]
        {
            let process_name = match browser_type {
                BrowserType::Chrome => "chrome",
                BrowserType::Firefox => "firefox",
                BrowserType::Edge => "msedge",
                BrowserType::Brave => "brave",
                _ => return false,
            };

            std::process::Command::new("pgrep")
                .arg("-x")
                .arg(process_name)
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        }

        #[cfg(target_os = "windows")]
        {
            let process_name = match browser_type {
                BrowserType::Chrome => "chrome.exe",
                BrowserType::Firefox => "firefox.exe",
                BrowserType::Safari => return false, // Safari not on Windows
                BrowserType::Edge => "msedge.exe",
                BrowserType::Brave => "brave.exe",
                BrowserType::Other => return false,
            };

            std::process::Command::new("tasklist")
                .args(["/FI", &format!("IMAGENAME eq {}", process_name)])
                .output()
                .map(|output| String::from_utf8_lossy(&output.stdout).contains(process_name))
                .unwrap_or(false)
        }
    }
}
