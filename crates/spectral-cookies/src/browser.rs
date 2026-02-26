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
        if let Ok(chrome_profiles) = Self::detect_chrome() {
            profiles.extend(chrome_profiles);
        }

        // Firefox
        if let Ok(firefox_profiles) = Self::detect_firefox() {
            profiles.extend(firefox_profiles);
        }

        // Safari (macOS only)
        #[cfg(target_os = "macos")]
        if let Ok(safari_profile) = Self::detect_safari() {
            profiles.push(safari_profile);
        }

        // Edge
        if let Ok(edge_profiles) = Self::detect_edge() {
            profiles.extend(edge_profiles);
        }

        // Brave
        if let Ok(brave_profiles) = Self::detect_brave() {
            profiles.extend(brave_profiles);
        }

        Ok(profiles)
    }

    /// Detect Chrome installations and profiles.
    fn detect_chrome() -> Result<Vec<BrowserProfile>> {
        let mut profiles = Vec::new();
        let base_path = Self::chrome_base_path()?;

        if !base_path.exists() {
            return Ok(profiles);
        }

        // Default profile
        let default_cookies = base_path.join("Default").join("Cookies");
        if default_cookies.exists() {
            profiles.push(BrowserProfile {
                browser_type: BrowserType::Chrome,
                profile_name: "Default".to_string(),
                cookie_db_path: default_cookies,
                is_default: true,
            });
        }

        // Additional profiles (Profile 1, Profile 2, etc.)
        for entry in std::fs::read_dir(&base_path).map_err(CookieError::IoError)? {
            let entry = entry.map_err(CookieError::IoError)?;
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if name.starts_with("Profile ") {
                let cookies_path = path.join("Cookies");
                if cookies_path.exists() {
                    profiles.push(BrowserProfile {
                        browser_type: BrowserType::Chrome,
                        profile_name: name.to_string(),
                        cookie_db_path: cookies_path,
                        is_default: false,
                    });
                }
            }
        }

        Ok(profiles)
    }

    /// Detect Firefox installations and profiles.
    fn detect_firefox() -> Result<Vec<BrowserProfile>> {
        let mut profiles = Vec::new();
        let base_path = Self::firefox_base_path()?;

        if !base_path.exists() {
            return Ok(profiles);
        }

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

    /// Detect Edge installations and profiles.
    fn detect_edge() -> Result<Vec<BrowserProfile>> {
        let mut profiles = Vec::new();
        let base_path = Self::edge_base_path()?;

        if !base_path.exists() {
            return Ok(profiles);
        }

        // Edge uses same structure as Chrome
        let default_cookies = base_path.join("Default").join("Cookies");
        if default_cookies.exists() {
            profiles.push(BrowserProfile {
                browser_type: BrowserType::Edge,
                profile_name: "Default".to_string(),
                cookie_db_path: default_cookies,
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
            return Ok(profiles);
        }

        // Brave uses same structure as Chrome
        let default_cookies = base_path.join("Default").join("Cookies");
        if default_cookies.exists() {
            profiles.push(BrowserProfile {
                browser_type: BrowserType::Brave,
                profile_name: "Default".to_string(),
                cookie_db_path: default_cookies,
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
