//! Tray mode integration — cross-platform system tray support.

/// Returns true if tray support is available on the current platform.
///
/// On macOS and Windows this is always true.
/// On Linux, requires `libappindicator3` or `libayatana-appindicator`.
pub fn is_tray_supported() -> bool {
    #[cfg(target_os = "linux")]
    {
        // Check for appindicator at runtime by attempting to load it
        std::process::Command::new("ldconfig")
            .arg("-p")
            .output()
            .map(|o| {
                let libs = String::from_utf8_lossy(&o.stdout);
                libs.contains("libappindicator3") || libs.contains("libayatana-appindicator3")
            })
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}

/// Tray icon menu item IDs
pub const MENU_OPEN: &str = "open";
pub const MENU_SCAN: &str = "scan_now";
pub const MENU_QUIT: &str = "quit";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_supported_detection_does_not_panic() {
        // On any platform, should return without panicking
        let _ = is_tray_supported();
    }
}
