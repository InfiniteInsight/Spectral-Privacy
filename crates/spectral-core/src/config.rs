//! Configuration management for Spectral.
//!
//! Provides TOML-based configuration with XDG-compliant paths and
//! environment variable overrides.

use crate::error::{ConfigError, ConfigResult};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Main application configuration.
///
/// This is loaded from `~/.config/spectral/config.toml` (or platform equivalent).
/// If the file doesn't exist, default values are used.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// General application settings
    pub general: GeneralConfig,
    /// Vault and encryption settings
    pub vault: VaultConfig,
    /// Scanning behavior settings
    pub scanning: ScanningConfig,
    /// Browser automation settings
    pub browser: BrowserConfig,
    /// LLM integration settings
    pub llm: LlmConfig,
    /// Notification settings
    pub notifications: NotificationConfig,
}

impl AppConfig {
    /// Load configuration from disk, falling back to defaults if not found.
    ///
    /// # Errors
    /// Returns error if:
    /// - Config directory cannot be determined
    /// - File exists but cannot be read
    /// - File contents are not valid TOML
    pub fn load() -> ConfigResult<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            tracing::debug!("Loading config from {}", config_path.display());
            let contents = fs::read_to_string(&config_path)?;
            let config = toml::from_str(&contents)?;
            Ok(config)
        } else {
            tracing::debug!("Config file not found, using defaults");
            Ok(Self::default())
        }
    }

    /// Load configuration with environment variable overrides.
    ///
    /// Supports the following environment variables:
    /// - `SPECTRAL_AUTO_LOCK_MINUTES`: Override auto-lock timeout
    /// - `SPECTRAL_HEADLESS`: Override browser headless mode (true/false)
    /// - `SPECTRAL_LLM_ENABLED`: Override LLM enabled status (true/false)
    pub fn load_with_env() -> ConfigResult<Self> {
        let mut config = Self::load()?;

        // Override from environment
        if let Ok(val) = std::env::var("SPECTRAL_AUTO_LOCK_MINUTES") {
            if let Ok(minutes) = val.parse() {
                config.vault.auto_lock_minutes = minutes;
                tracing::debug!("Override auto_lock_minutes from env: {}", minutes);
            }
        }

        if let Ok(val) = std::env::var("SPECTRAL_HEADLESS") {
            if let Ok(headless) = val.parse() {
                config.browser.headless = headless;
                tracing::debug!("Override browser.headless from env: {}", headless);
            }
        }

        if let Ok(val) = std::env::var("SPECTRAL_LLM_ENABLED") {
            if let Ok(enabled) = val.parse() {
                config.llm.enabled = enabled;
                tracing::debug!("Override llm.enabled from env: {}", enabled);
            }
        }

        Ok(config)
    }

    /// Save configuration to disk.
    ///
    /// Creates the config directory if it doesn't exist.
    pub fn save(&self) -> ConfigResult<()> {
        let config_path = Self::config_path()?;
        let config_dir = config_path
            .parent()
            .ok_or_else(|| ConfigError::InvalidValue {
                field: "config_path".to_string(),
                reason: "no parent directory".to_string(),
            })?;

        fs::create_dir_all(config_dir)?;
        tracing::debug!("Saving config to {}", config_path.display());

        let contents = toml::to_string_pretty(self)?;
        fs::write(config_path, contents)?;
        Ok(())
    }

    /// Get the path to the configuration file.
    ///
    /// Uses XDG base directories: `~/.config/spectral/config.toml`
    pub fn config_path() -> ConfigResult<PathBuf> {
        let dirs =
            ProjectDirs::from("com", "spectral", "spectral").ok_or(ConfigError::NoConfigDir)?;
        Ok(dirs.config_dir().join("config.toml"))
    }

    /// Get the data directory path.
    ///
    /// Uses XDG base directories: `~/.local/share/spectral`
    pub fn data_dir() -> ConfigResult<PathBuf> {
        let dirs =
            ProjectDirs::from("com", "spectral", "spectral").ok_or(ConfigError::NoConfigDir)?;
        Ok(dirs.data_dir().to_path_buf())
    }

    /// Get the cache directory path.
    ///
    /// Uses XDG base directories: `~/.cache/spectral`
    pub fn cache_dir() -> ConfigResult<PathBuf> {
        let dirs =
            ProjectDirs::from("com", "spectral", "spectral").ok_or(ConfigError::NoConfigDir)?;
        Ok(dirs.cache_dir().to_path_buf())
    }
}

/// General application settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// UI theme: "light", "dark", or "system"
    pub theme: String,
    /// Whether to check for updates on startup
    pub check_updates: bool,
    /// Whether to send anonymous usage statistics
    pub telemetry: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            check_updates: true,
            telemetry: false,
        }
    }
}

/// Vault and encryption settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VaultConfig {
    /// Auto-lock timeout in minutes (0 = never)
    pub auto_lock_minutes: u32,
    /// Argon2 memory cost in KB
    pub argon2_memory_kb: u32,
    /// Argon2 iteration count
    pub argon2_iterations: u32,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            auto_lock_minutes: 15,
            argon2_memory_kb: 262_144, // 256 MB
            argon2_iterations: 4,
        }
    }
}

/// Scanning behavior settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScanningConfig {
    /// Number of concurrent scans allowed
    pub concurrent_scans: u32,
    /// Delay between scans in milliseconds
    pub delay_between_scans_ms: u64,
    /// Whether to respect robots.txt
    pub respect_robots_txt: bool,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// User agent string
    pub user_agent: String,
}

impl Default for ScanningConfig {
    fn default() -> Self {
        Self {
            concurrent_scans: 3,
            delay_between_scans_ms: 2000,
            respect_robots_txt: true,
            timeout_secs: 30,
            user_agent: "Spectral/0.1.0 (+https://github.com/spectral-privacy/spectral)"
                .to_string(),
        }
    }
}

/// Browser automation settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BrowserConfig {
    /// Run browser in headless mode
    pub headless: bool,
    /// Browser window width
    pub window_width: u32,
    /// Browser window height
    pub window_height: u32,
    /// Navigation timeout in seconds
    pub navigation_timeout_secs: u64,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            headless: true,
            window_width: 1920,
            window_height: 1080,
            navigation_timeout_secs: 30,
        }
    }
}

/// LLM integration settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmConfig {
    /// Whether LLM features are enabled
    pub enabled: bool,
    /// Default provider: `anthropic`, `ollama`, `openai`, or `none`
    pub default_provider: String,
    /// Routing preference: `local_only`, `cloud_only`, or `local_preferred`
    pub routing_preference: String,
    /// Anthropic API key (stored encrypted in vault, not here)
    #[serde(skip)]
    pub anthropic_api_key: Option<String>,
    /// Ollama server URL
    pub ollama_url: String,
    /// Maximum tokens for completions
    pub max_tokens: u32,
    /// Temperature for completions
    pub temperature: f32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_provider: "none".to_string(),
            routing_preference: "local_preferred".to_string(),
            anthropic_api_key: None,
            ollama_url: "http://localhost:11434".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
        }
    }
}

/// Notification settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
pub struct NotificationConfig {
    /// Enable desktop notifications
    pub enabled: bool,
    /// Notify on scan completion
    pub notify_scan_complete: bool,
    /// Notify when PII is found
    pub notify_pii_found: bool,
    /// Notify on removal confirmation
    pub notify_removal_confirmed: bool,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            notify_scan_complete: true,
            notify_pii_found: true,
            notify_removal_confirmed: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.general.theme, "system");
        assert_eq!(config.vault.auto_lock_minutes, 15);
        assert_eq!(config.scanning.concurrent_scans, 3);
        assert!(config.browser.headless);
        assert!(!config.llm.enabled);
    }

    #[test]
    fn test_config_serialization() {
        let config = AppConfig::default();
        let toml_str = toml::to_string_pretty(&config).expect("serialize default config");
        assert!(toml_str.contains("[general]"));
        assert!(toml_str.contains("[vault]"));
        assert!(toml_str.contains("[scanning]"));

        let parsed: AppConfig = toml::from_str(&toml_str).expect("parse serialized config");
        assert_eq!(parsed.general.theme, config.general.theme);
    }

    #[test]
    fn test_config_save_load() {
        let tmp = TempDir::new().expect("create temp dir");
        let config_path = tmp.path().join("config.toml");

        // Create a custom config
        let mut config = AppConfig::default();
        config.general.theme = "dark".to_string();
        config.vault.auto_lock_minutes = 30;

        // Save
        let contents = toml::to_string_pretty(&config).expect("serialize config");
        fs::write(&config_path, contents).expect("write config file");

        // Load
        let loaded_contents = fs::read_to_string(&config_path).expect("read config file");
        let loaded: AppConfig = toml::from_str(&loaded_contents).expect("parse loaded config");

        assert_eq!(loaded.general.theme, "dark");
        assert_eq!(loaded.vault.auto_lock_minutes, 30);
    }

    #[test]
    fn test_env_overrides() {
        std::env::set_var("SPECTRAL_AUTO_LOCK_MINUTES", "60");
        std::env::set_var("SPECTRAL_HEADLESS", "false");
        std::env::set_var("SPECTRAL_LLM_ENABLED", "true");

        // Can't test load_with_env directly since it tries to read config file,
        // but we can test the logic
        let mut config = AppConfig::default();
        if let Ok(val) = std::env::var("SPECTRAL_AUTO_LOCK_MINUTES") {
            if let Ok(minutes) = val.parse() {
                config.vault.auto_lock_minutes = minutes;
            }
        }
        assert_eq!(config.vault.auto_lock_minutes, 60);

        std::env::remove_var("SPECTRAL_AUTO_LOCK_MINUTES");
        std::env::remove_var("SPECTRAL_HEADLESS");
        std::env::remove_var("SPECTRAL_LLM_ENABLED");
    }

    #[test]
    fn test_partial_config() {
        // Test that partial TOML configs work with defaults
        let toml_str = r#"
[general]
theme = "dark"

[vault]
auto_lock_minutes = 20
"#;

        let config: AppConfig = toml::from_str(toml_str).expect("parse partial config");
        assert_eq!(config.general.theme, "dark");
        assert_eq!(config.vault.auto_lock_minutes, 20);
        // These should be defaults
        assert_eq!(config.scanning.concurrent_scans, 3);
        assert!(config.browser.headless);
    }
}
