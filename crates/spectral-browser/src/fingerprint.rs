use rand::Rng;

/// Fingerprint configuration for anti-detection
#[derive(Debug, Clone)]
pub struct FingerprintConfig {
    pub user_agent: String,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub timezone: String,
}

impl FingerprintConfig {
    /// Generate a randomized fingerprint configuration
    pub fn randomized() -> Self {
        let mut rng = rand::thread_rng();

        // Common desktop user agents
        let user_agents = [
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        ];

        // Common viewport sizes
        let viewports = [(1920, 1080), (1366, 768), (1536, 864), (1440, 900)];

        let ua_idx = rng.gen_range(0..user_agents.len());
        let vp_idx = rng.gen_range(0..viewports.len());
        let (width, height) = viewports[vp_idx];

        Self {
            user_agent: user_agents[ua_idx].to_string(),
            viewport_width: width,
            viewport_height: height,
            timezone: "America/New_York".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_randomized_fingerprint() {
        let config = FingerprintConfig::randomized();
        assert!(!config.user_agent.is_empty());
        assert!(config.viewport_width > 0);
        assert!(config.viewport_height > 0);
        assert!(!config.timezone.is_empty());
    }

    #[test]
    fn test_fingerprint_variation() {
        let _config1 = FingerprintConfig::randomized();
        let _config2 = FingerprintConfig::randomized();

        // Configs should be different at least some of the time
        // (This is probabilistic but very unlikely to fail)
        let configs: Vec<_> = (0..10).map(|_| FingerprintConfig::randomized()).collect();

        let first_ua = &configs[0].user_agent;
        let all_same = configs.iter().all(|c| &c.user_agent == first_ua);
        assert!(!all_same, "Expected variation in user agents");
    }
}
