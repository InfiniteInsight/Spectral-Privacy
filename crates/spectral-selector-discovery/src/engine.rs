//! Discovery engine: coordinates DOM analysis, classification, and candidate selection
//! across multiple URLs for a single broker.

use crate::dom_analyzer::{analyze_dom, classify_element};
use crate::scorer::{ScoreBreakdown, SelectorScorer};
use crate::types::{
    DiscoveredElement, DiscoveredSelector, DiscoveryConfig, DiscoveryResult, SelectorPurpose,
    SelectorTargetType,
};
use anyhow::Result;
use spectral_core::BrokerId;
use tracing::{debug, info, warn};

/// Run a discovery scan against a set of URLs for a given broker.
pub async fn run_discovery(
    broker_id: &BrokerId,
    urls: &[String],
    config: &DiscoveryConfig,
) -> Result<Vec<DiscoveryResult>> {
    let mut results = Vec::new();

    for url in urls {
        info!("Discovering selectors for broker {} at {}", broker_id, url);

        // In production, this would navigate the browser to the URL, wait for
        // the page to load, and capture the HTML. For now we accept HTML as a
        // parameter.
        let html = fetch_page_html(broker_id, url).await?;

        let mut result = analyze_page(broker_id, url, &html, config)?;
        results.push(result);
    }

    Ok(results)
}

/// Analyze a single page: discover elements, classify them, and score candidates.
fn analyze_page(
    broker_id: &BrokerId,
    url: &str,
    html: &str,
    config: &DiscoveryConfig,
) -> Result<DiscoveryResult> {
    // Step 1: Discover raw elements from the DOM
    let elements = analyze_dom(html)?;
    debug!("Found {} elements on {}", elements.len(), url);

    // Step 2: Classify each element into purposes
    let mut classified: Vec<DiscoveredSelector> = Vec::new();
    for el in &elements {
        let purposes = classify_element(el);
        for (purpose, target_type) in purposes {
            let selector = DiscoveredSelector {
                selector: el.selector.clone(),
                target_type,
                purpose: purpose.clone(),
                confidence: 0.6, // Base confidence before scoring
                occurrence_count: el.match_count,
                source_url: url.to_string(),
            };
            classified.push(selector);
        }
    }

    // Step 3: Score candidates
    let scorer = SelectorScorer::default();
    let mut scored_selectors = Vec::new();
    let mut candidate_selectors = Vec::new();

    for disc_sel in &classified {
        let score = scorer.score(disc_sel);
        scored_selectors.push(score);

        if disc_sel.confidence >= config.min_confidence {
            candidate_selectors.push(disc_sel.clone());
        }
    }

    // Sort scores descending
    scored_selectors.sort_by(|a, b| b.raw_score.partial_cmp(&a.raw_score).unwrap());

    let discovery_result = DiscoveryResult {
        broker_id: broker_id.clone(),
        url: url.to_string(),
        elements,
        candidate_selectors,
        scored_selectors,
    };

    debug!(
        "Discovery complete for {}: {} elements, {} candidates",
        broker_id,
        discovery_result.elements.len(),
        discovery_result.candidate_selectors.len()
    );

    Ok(discovery_result)
}

/// Fetch a page's HTML from a browser context.
/// In production this integrates with spectral-browser to navigate and capture.
async fn fetch_page_html(_broker_id: &BrokerId, _url: &str) -> Result<String> {
    // Placeholder — integrates with spectral-browser when it's available.
    // The browser session would:
    // 1. Navigate to the URL
    // 2. Wait for network idle / timeout
    // 3. Return innerHTML of the body
    warn!(
        "fetch_page_html: browser integration not yet wired up for {} — returning placeholder",
        _url
    );
    Err(anyhow::anyhow!(
        "Browser integration not yet wired up. Pass HTML directly via analyze_page()."
    ))
}

/// Consolidate discovery results from multiple pages into per-purpose top selectors.
pub fn consolidate_results(results: &[DiscoveryResult]) -> Vec<(SelectorPurpose, String, f64)> {
    let mut purpose_scores: std::collections::HashMap<String, Vec<(f64, String)>> =
        std::collections::HashMap::new();

    for result in results {
        for scored in &result.scored_selectors {
            let key = format!("{:?}", scored.purpose);
            purpose_scores
                .entry(key)
                .or_default()
                .push((scored.raw_score, scored.selector.clone()));
        }
    }

    let mut consolidated = Vec::new();
    for (purpose_str, mut scores) in purpose_scores {
        scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        if let Some((best_score, best_selector)) = scores.first() {
            let purpose = parse_purpose_from_key(&purpose_str);
            consolidated.push((purpose, best_selector.clone(), *best_score));
        }
    }

    consolidated.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    consolidated
}

/// Parse a SelectorPurpose from its Debug-formatted key string.
fn parse_purpose_from_key(key: &str) -> SelectorPurpose {
    let key = key.trim_start_matches('\"').trim_end_matches('\"');
    // Remove the "Other(...)" wrapper for generic entries
    if key.starts_matches("Other(") {
        let inner = key.trim_start_matches("Other(").trim_end_matches(')');
        return SelectorPurpose::Other(inner.to_string());
    }
    SelectorPurpose::from_toml_key(key).unwrap_or(SelectorPurpose::Other(key.to_string()))
}

/// Run discovery using existing broker definition URLs as a fallback.
pub async fn discover_for_broker(
    broker_id: &BrokerId,
    definition_urls: &[String],
) -> Result<Vec<DiscoveryResult>> {
    let config = DiscoveryConfig::default();
    let urls = if definition_urls.is_empty() {
        vec![format!("https://{}/search", broker_id)]
    } else {
        definition_urls.to_vec()
    };
    run_discovery(broker_id, &urls, &config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DiscoveredSelector, SelectorScore};

    #[test]
    fn test_consolidate_results() {
        let broker_id = BrokerId::new("test-broker").unwrap();
        let results = vec![DiscoveryResult {
            broker_id: broker_id.clone(),
            url: "https://example.com/search".into(),
            elements: Vec::new(),
            candidate_selectors: Vec::new(),
            scored_selectors: vec![
                SelectorScore {
                    selector: "#firstNameInput".into(),
                    purpose: SelectorPurpose::FirstNameInput,
                    raw_score: 0.9,
                    adjusted_score: 0.9,
                    breakdown: ScoreBreakdown {
                        uniqueness_bonus: 0.2,
                        brevity_bonus: 0.1,
                        attribute_bonus: 0.1,
                        semantic_bonus: 0.4,
                        stability_penalty: 0.0,
                        specificity_score: 0.2,
                    },
                },
                SelectorScore {
                    selector: "#lastNameInput".into(),
                    purpose: SelectorPurpose::LastNameInput,
                    raw_score: 0.85,
                    adjusted_score: 0.85,
                    breakdown: ScoreBreakdown {
                        uniqueness_bonus: 0.2,
                        brevity_bonus: 0.1,
                        attribute_bonus: 0.1,
                        semantic_bonus: 0.35,
                        stability_penalty: 0.0,
                        specificity_score: 0.2,
                    },
                },
            ],
        }];

        let consolidated = consolidate_results(&results);
        assert!(!consolidated.is_empty());
        // Should be sorted by score descending
        for i in 1..consolidated.len() {
            assert!(
                consolidated[i - 1].2 >= consolidated[i].2,
                "consolidated results not sorted by score"
            );
        }
    }

    #[test]
    fn test_parse_purpose_from_key() {
        assert!(matches!(
            parse_purpose_from_key("\"FirstNameInput\""),
            SelectorPurpose::FirstNameInput
        ));
        assert!(matches!(
            parse_purpose_from_key("\"Other(tag_input)\""),
            SelectorPurpose::Other(_)
        ));
    }
}
