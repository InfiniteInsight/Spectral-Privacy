//! Cookie pattern matching against broker definitions.

use crate::{error::Result, Cookie};
use globset::{Glob, GlobSet, GlobSetBuilder};

/// Broker cookie pattern from TOML configuration.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct BrokerCookiePattern {
    pub broker_id: String,
    pub patterns: Vec<String>,
    pub domains: Vec<String>,
}

/// Cookie matcher that matches cookies against broker patterns.
pub struct CookieMatcher {
    patterns: Vec<(String, GlobSet, Vec<String>)>, // (broker_id, name_patterns, domains)
}

impl CookieMatcher {
    /// Create a new cookie matcher from broker patterns.
    pub fn new(broker_patterns: Vec<BrokerCookiePattern>) -> Result<Self> {
        let mut patterns = Vec::new();

        for broker in broker_patterns {
            let mut builder = GlobSetBuilder::new();
            for pattern in &broker.patterns {
                let glob = Glob::new(pattern)
                    .map_err(|e| crate::error::CookieError::ParseError(e.to_string()))?;
                builder.add(glob);
            }
            let globset = builder
                .build()
                .map_err(|e| crate::error::CookieError::ParseError(e.to_string()))?;

            patterns.push((broker.broker_id.clone(), globset, broker.domains.clone()));
        }

        Ok(Self { patterns })
    }

    /// Match a cookie against all broker patterns.
    /// Returns the broker ID if matched, None otherwise.
    pub fn match_cookie(&self, cookie: &Cookie) -> Option<String> {
        for (broker_id, name_patterns, domains) in &self.patterns {
            // Check if cookie name matches any pattern
            if name_patterns.is_match(&cookie.name) {
                // Check if domain matches
                if Self::domain_matches(&cookie.domain, domains) {
                    return Some(broker_id.clone());
                }
            }
        }

        None
    }

    /// Check if domains match with dot prefix handling.
    fn domains_match_with_dot_prefix(domain1: &str, domain2: &str) -> bool {
        if domain2.starts_with('.') {
            domain1 == &domain2[1..] || domain1.ends_with(domain2)
        } else if domain1.starts_with('.') {
            domain2 == &domain1[1..] || domain2.ends_with(domain1)
        } else {
            false
        }
    }

    /// Check if one domain is a subdomain of another.
    fn is_subdomain(domain1: &str, domain2: &str) -> bool {
        domain1.ends_with(&format!(".{}", domain2)) || domain2.ends_with(&format!(".{}", domain1))
    }

    /// Check if a cookie domain matches any of the broker domains.
    fn domain_matches(cookie_domain: &str, broker_domains: &[String]) -> bool {
        for broker_domain in broker_domains {
            // Exact match
            if cookie_domain == broker_domain {
                return true;
            }

            // Subdomain match with dot prefix handling
            if Self::domains_match_with_dot_prefix(cookie_domain, broker_domain) {
                return true;
            }

            // Check if cookie domain is a subdomain of broker domain
            if Self::is_subdomain(cookie_domain, broker_domain) {
                return true;
            }
        }

        false
    }

    /// Get all broker IDs that have patterns.
    pub fn get_broker_ids(&self) -> Vec<String> {
        self.patterns.iter().map(|(id, _, _)| id.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cookie_matching() {
        let patterns = vec![
            BrokerCookiePattern {
                broker_id: "google-ads".to_string(),
                patterns: vec!["_ga*".to_string(), "_gid".to_string()],
                domains: vec![
                    ".google.com".to_string(),
                    ".google-analytics.com".to_string(),
                ],
            },
            BrokerCookiePattern {
                broker_id: "facebook".to_string(),
                patterns: vec!["_fbp".to_string(), "fr".to_string()],
                domains: vec![".facebook.com".to_string()],
            },
        ];

        // nosemgrep: no-unwrap-in-production
        let matcher = CookieMatcher::new(patterns).unwrap();

        // Test Google Analytics cookie
        let cookie = Cookie {
            name: "_ga_ABC123".to_string(),
            value: "test".to_string(),
            domain: ".google.com".to_string(),
            path: "/".to_string(),
            creation_time: None,
            expiry_time: None,
            last_access_time: None,
            is_secure: false,
            is_httponly: false,
            same_site: None,
        };

        assert_eq!(
            matcher.match_cookie(&cookie),
            Some("google-ads".to_string())
        );

        // Test Facebook cookie
        let cookie = Cookie {
            name: "_fbp".to_string(),
            value: "test".to_string(),
            domain: ".facebook.com".to_string(),
            path: "/".to_string(),
            creation_time: None,
            expiry_time: None,
            last_access_time: None,
            is_secure: false,
            is_httponly: false,
            same_site: None,
        };

        assert_eq!(matcher.match_cookie(&cookie), Some("facebook".to_string()));

        // Test non-matching cookie
        let cookie = Cookie {
            name: "session_id".to_string(),
            value: "test".to_string(),
            domain: ".example.com".to_string(),
            path: "/".to_string(),
            creation_time: None,
            expiry_time: None,
            last_access_time: None,
            is_secure: false,
            is_httponly: false,
            same_site: None,
        };

        assert_eq!(matcher.match_cookie(&cookie), None);
    }
}
