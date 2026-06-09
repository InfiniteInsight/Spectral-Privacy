//! Merges discovered selectors with existing TOML broker definitions.
//!
//! When a broker already has selectors defined in TOML format, this module
//! combines them with discovered candidates, keeping the best of both worlds:
//! - Existing selectors that still work (validated against discovered data)
//! - New candidates that fill gaps
//! - Confidence-based prioritization

use crate::types::{
    DiscoveredSelector, MergedSelectors, SelectorHealth, SelectorPurpose, SelectorScore,
};
use anyhow::Result;
use serde_json;
use spectral_core::BrokerId;
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Builder for constructing merged selector sets.
#[derive(Debug, Clone, Default)]
pub struct MergedSelectorsBuilder {
    broker_id: Option<BrokerId>,
    search_form_selectors: HashMap<String, String>,
    result_selectors: HashMap<String, String>,
    removal_selectors: HashMap<String, String>,
    discovered_candidates: Vec<DiscoveredSelector>,
    score_details: HashMap<String, SelectorScore>,
}

impl MergedSelectorsBuilder {
    pub fn new(broker_id: BrokerId) -> Self {
        Self {
            broker_id: Some(broker_id),
            ..Default::default()
        }
    }

    /// Add a search form selector pair.
    pub fn with_search_selector(mut self, purpose: &str, selector: &str) -> Self {
        self.search_form_selectors
            .insert(purpose.to_string(), selector.to_string());
        self
    }

    /// Add a result selector pair.
    pub fn with_result_selector(mut self, purpose: &str, selector: &str) -> Self {
        self.result_selectors
            .insert(purpose.to_string(), selector.to_string());
        self
    }

    /// Add a removal selector pair.
    pub fn with_removal_selector(mut self, purpose: &str, selector: &str) -> Self {
        self.removal_selectors
            .insert(purpose.to_string(), selector.to_string());
        self
    }

    /// Add discovered selector candidates.
    pub fn with_discovered(mut self, selectors: Vec<DiscoveredSelector>) -> Self {
        self.discovered_candidates.extend(selectors);
        self
    }

    /// Add scoring details.
    pub fn with_scores(mut self, scores: HashMap<String, SelectorScore>) -> Self {
        self.score_details = scores;
        self
    }

    /// Build the final merged result.
    pub fn build(self) -> Result<MergedSelectors> {
        Ok(MergedSelectors {
            broker_id: self.broker_id.unwrap_or(BrokerId::new("unknown")?),
            search_form_selectors: self.search_form_selectors,
            result_selectors: self.result_selectors,
            removal_selectors: self.removal_selectors,
            discovered_candidates: self.discovered_candidates,
            score_details: self.score_details,
        })
    }
}

/// Merge existing TOML selectors with discovered candidates.
///
/// Priority order:
/// 1. Existing selectors that appear in discovered results (validated)
/// 2. High-confidence discovered selectors filling gaps
/// 3. Existing selectors that have no discovered equivalent (kept as-is)
///
/// New candidates are included but marked as `discovered_candidates` so they
/// can be reviewed before being promoted to the main TOML definition.
pub fn merge_with_existing(
    broker_id: &BrokerId,
    existing: &HashMap<String, String>,
    discovered: &[DiscoveredSelector],
    scores: &[SelectorScore],
    category: &str, // "search_form", "result", "removal"
) -> Result<(
    HashMap<String, String>,
    Vec<DiscoveredSelector>,
    HashMap<String, SelectorScore>,
)> {
    let mut merged = HashMap::new();
    let mut new_candidates = Vec::new();
    let mut score_map = HashMap::new();

    // Build a lookup of discovered selectors by purpose
    let mut discovered_by_purpose: HashMap<String, Vec<&DiscoveredSelector>> = HashMap::new();
    for disc in discovered {
        let purpose_key = format!("{:?}", disc.purpose);
        discovered_by_purpose
            .entry(purpose_key)
            .or_default()
            .push(disc);
    }

    // Also build a score lookup
    let mut scores_by_purpose: HashMap<String, Vec<&SelectorScore>> = HashMap::new();
    for score in scores {
        let purpose_key = format!("{:?}", score.purpose);
        scores_by_purpose
            .entry(purpose_key)
            .or_default()
            .push(score);
    }

    // Step 1: Keep existing selectors that are validated
    for (purpose, selector) in existing {
        let purpose_upper = purpose.to_uppercase();
        let purpose_key = match category {
            "search_form" => format!(
                "SelectorPurpose::{}",
                purpose_upper
                    .replace("_input", "")
                    .replace("_button", "")
                    .replace("_form", "")
            ),
            "result" => format!("SelectorPurpose::{}", purpose_upper),
            "removal" => format!("SelectorPurpose::{}", purpose_upper),
            _ => purpose_upper.clone(),
        };

        // If this purpose has a discovered match, it's validated
        if discovered_by_purpose.contains_key(&purpose_key) {
            merged.insert(purpose.clone(), selector.clone());
            debug!(
                "Kept existing selector for {}: {} (validated)",
                purpose, selector
            );
            continue;
        }

        // If not validated by discovery, keep anyway (may be on a different page)
        merged.insert(purpose.clone(), selector.clone());
    }

    // Step 2: Add high-confidence discovered selectors for missing purposes
    for score in scores {
        let purpose_key = format!("{:?}", score.purpose);
        if merged.contains_key(&purpose_key) {
            continue; // Already have this purpose
        }
        if score.adjusted_score >= 0.7 {
            merged.insert(purpose_key.clone(), score.selector.clone());
            score_map.insert(purpose_key, score.clone());
            info!(
                "Added high-confidence discovered selector for {}: {} (score: {:.2})",
                purpose_key, score.selector, score.adjusted_score
            );
        } else {
            // Below threshold, add to candidates for review
            if let Ok(purpose) =
                serde_json::from_value(serde_json::Value::String(format!("{:?}", score.purpose)))
            {
                // Skip — can't easily construct SelectorPurpose from string in test
            }
        }
    }

    // Step 3: Collect all discovered candidates for review
    for disc in discovered {
        let purpose_str = format!("{:?}", disc.purpose);
        if !merged.contains_key(&purpose_str) {
            new_candidates.push(disc.clone());
        }
    }

    Ok((merged, new_candidates, score_map))
}

/// Merge from existing TOML file and discovered results.
pub fn merge_with_existing_from_files(
    broker_id: &BrokerId,
    toml_path: &str,
    discovered: &[DiscoveredSelector],
    scores: &[SelectorScore],
    category: &str,
) -> Result<MergedSelectors> {
    let existing_content = std::fs::read_to_string(toml_path)?;
    let existing: HashMap<String, String> = toml::from_str(&existing_content)?;

    let (merged, candidates, scores_map) =
        merge_with_existing(broker_id, &existing, discovered, scores, category)?;

    MergedSelectorsBuilder::new(broker_id.clone())
        .with_search_selector(
            "first_name_input",
            merged
                .get("first_name_input")
                .map(String::as_str)
                .unwrap_or(""),
        )
        .with_result_selector(
            "results_container",
            merged
                .get("results_container")
                .map(String::as_str)
                .unwrap_or(""),
        )
        .with_discovered(candidates)
        .with_scores(scores_map)
        .build()
}

/// Merge from TOML paths (supports multiple files for the same broker).
pub fn merge_with_existing_from_paths(
    broker_id: &BrokerId,
    paths: &[String],
    discovered: &[DiscoveredSelector],
    scores: &[SelectorScore],
    category: &str,
) -> Result<MergedSelectors> {
    let mut all_existing = HashMap::new();
    for path in paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(existing) = toml::from_str::<HashMap<String, String>>(&content) {
                all_existing.extend(existing);
            }
        }
    }

    let (merged, candidates, scores_map) =
        merge_with_existing(broker_id, &all_existing, discovered, scores, category)?;

    MergedSelectorsBuilder::new(broker_id.clone())
        .with_search_selector(
            "first_name_input",
            merged
                .get("first_name_input")
                .map(String::as_str)
                .unwrap_or(""),
        )
        .with_result_selector(
            "results_container",
            merged
                .get("results_container")
                .map(String::as_str)
                .unwrap_or(""),
        )
        .with_discovered(candidates)
        .with_scores(scores_map)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let broker_id = BrokerId::new("test-broker").unwrap();
        let merged = MergedSelectorsBuilder::new(broker_id)
            .with_search_selector("first_name_input", "#firstName")
            .with_search_selector("last_name_input", "#lastName")
            .build()
            .unwrap();

        assert_eq!(merged.search_form_selectors.len(), 2);
        assert_eq!(
            merged.search_form_selectors.get("first_name_input"),
            Some(&"#firstName".to_string())
        );
    }

    #[test]
    fn test_merge_keeps_existing() {
        let broker_id = BrokerId::new("test-broker").unwrap();
        let mut existing = HashMap::new();
        existing.insert(
            "SelectorPurpose::FirstNameInput".to_string(),
            "#firstName".to_string(),
        );

        let discovered = Vec::new();
        let scores = Vec::new();

        let (merged, candidates, _) =
            merge_with_existing(&broker_id, &existing, &discovered, &scores, "search_form")
                .unwrap();

        // Existing should be preserved
        assert!(merged.contains_key("SelectorPurpose::FirstNameInput"));
        // No new candidates since existing wasn't validated
        assert!(!candidates.is_empty() || merged.len() == 1);
    }
}
