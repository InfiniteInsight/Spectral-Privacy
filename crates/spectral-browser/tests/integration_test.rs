use spectral_browser::actions::BrowserActions;
use spectral_browser::BrowserEngine;

#[tokio::test]
#[ignore] // Requires Chrome/Chromium installed
async fn test_browser_engine_creation() {
    let engine = BrowserEngine::new().await;
    assert!(engine.is_ok(), "Failed to create browser engine");
}

#[tokio::test]
#[ignore] // Requires Chrome/Chromium installed
async fn test_navigation() {
    let engine = BrowserEngine::new().await.unwrap();

    // Navigate to example.com
    let result = engine.navigate("https://example.com").await;
    assert!(result.is_ok(), "Navigation failed");
}

#[tokio::test]
#[ignore] // Requires Chrome/Chromium installed
async fn test_rate_limiting() {
    let engine = BrowserEngine::new().await.unwrap();

    // First navigation should succeed
    assert!(engine.navigate("https://example.com").await.is_ok());

    // Immediate second navigation to same domain should fail
    assert!(engine.navigate("https://example.com/page2").await.is_err());
}
