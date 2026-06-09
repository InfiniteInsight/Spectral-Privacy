//! Selector scoring engine.
//!
//! Assigns a quality score to each discovered selector based on multiple
//! heuristics: uniqueness, brevity, attribute presence, semantic relevance,
//! and stability.

use crate::types::{DiscoveredSelector, ScoreBreakdown, SelectorPurpose, SelectorScore};

/// Scorer for ranking discovered CSS selectors.
#[derive(Debug, Clone, Default)]
pub struct SelectorScorer;

impl SelectorScorer {
    /// Score a discovered selector against a purpose.
    /// Returns a full SelectorScore with breakdown.
    pub fn score(&self, sel: &DiscoveredSelector) -> SelectorScore {
        let uniqueness = self.score_uniqueness(sel);
        let brevity = self.score_brevity(sel);
        let attribute = self.score_attributes(sel);
        let semantic = self.score_semantic(sel);
        let stability = self.score_stability(sel);

        let raw_score = (uniqueness * 0.25)
            + (brevity * 0.15)
            + (attribute * 0.20)
            + (semantic * 0.30)
            + (stability * 0.10);

        // Apply specificity bonus for highly specific selectors (e.g., id-based)
        let specificity = self.score_specificity(sel);
        let adjusted_score = (raw_score * 0.9) + (specificity * 0.1);

        // Clamp to [0.0, 1.0]
        let adjusted_score = adjusted_score.min(1.0).max(0.0);

        SelectorScore {
            selector: sel.selector.clone(),
            purpose: sel.purpose.clone(),
            raw_score,
            adjusted_score,
            breakdown: ScoreBreakdown {
                uniqueness_bonus: uniqueness,
                brevity_bonus: brevity,
                attribute_bonus: attribute,
                semantic_bonus: semantic,
                stability_penalty: stability,
                specificity_score: specificity,
            },
        }
    }

    /// Score uniqueness: how unique is this selector on the page?
    /// An id selector (#foo) scores 1.0. A class selector (.bar) scores 0.7.
    /// A generic tag selector (div) scores 0.2.
    fn score_uniqueness(&self, sel: &DiscoveredSelector) -> f64 {
        if sel.selector.starts_with('#') {
            return 1.0;
        }
        if sel.selector.contains("[id=") {
            return 0.95;
        }
        if sel.match_count <= 2 {
            return 0.8;
        }
        if sel.match_count <= 5 {
            return 0.5;
        }
        0.2
    }

    /// Score brevity: shorter selectors are generally more stable.
    /// 0-20 chars = 1.0, 20-40 = 0.7, 40-60 = 0.4, 60+ = 0.2.
    fn score_brevity(&self, sel: &DiscoveredSelector) -> f64 {
        let len = sel.selector.len();
        if len <= 15 {
            1.0
        } else if len <= 25 {
            0.9
        } else if len <= 40 {
            0.7
        } else if len <= 60 {
            0.4
        } else {
            0.2
        }
    }

    /// Score based on presence of useful attributes (id, name, data-*, etc.).
    fn score_attributes(&self, sel: &DiscoveredSelector) -> f64 {
        let mut score = 0.0;
        if sel.selector.starts_with('#') {
            score += 0.5;
        }
        if sel.selector.contains("[name=") {
            score += 0.3;
        }
        if sel.selector.contains("[data-") {
            score += 0.2;
        }
        if sel.selector.contains("form") || sel.selector.contains("input") {
            score += 0.1;
        }
        score.min(1.0)
    }

    /// Score semantic relevance: how well does the element match its claimed purpose?
    fn score_semantic(&self, sel: &DiscoveredSelector) -> f64 {
        let text_lower = sel.selector.to_lowercase();

        // High-confidence purpose matches
        let purpose_match = matches!(
            &sel.purpose,
            SelectorPurpose::FirstNameInput
                | SelectorPurpose::LastNameInput
                | SelectorPurpose::CityInput
                | SelectorPurpose::StateInput
                | SelectorPurpose::PhoneInput
                | SelectorPurpose::EmailInput
                | SelectorPurpose::SearchSubmitButton
                | SelectorPurpose::ResultsContainer
                | SelectorPurpose::ResultItem
                | SelectorPurpose::ListingUrl
                | SelectorPurpose::NoResultsIndicator
                | SelectorPurpose::CaptchaRequired
        );

        if purpose_match {
            0.8
        } else {
            // Check if the text content aligns with the purpose
            let text = sel.source_url.to_lowercase();
            let purpose_str = format!("{:?}", sel.purpose).to_lowercase();

            if text.contains(&purpose_str)
                || purpose_str.contains("search")
                || purpose_str.contains("result")
            {
                0.6
            } else {
                0.3
            }
        }
    }

    /// Score stability: selectors with id or data attributes tend to be stable.
    fn score_stability(&self, sel: &DiscoveredSelector) -> f64 {
        if sel.selector.starts_with('#') {
            return 0.9;
        }
        if sel.selector.contains("[data-") || sel.selector.contains("[data_") {
            return 0.85;
        }
        if sel.selector.contains("[name=") {
            return 0.6;
        }
        if sel.selector.starts_with('.') {
            return 0.5;
        }
        0.3
    }

    /// Score specificity: id-based selectors are the most specific.
    fn score_specificity(&self, sel: &DiscoveredSelector) -> f64 {
        if sel.selector.starts_with('#') {
            return 1.0;
        }
        if sel.selector.contains("[id=") {
            return 0.9;
        }
        if sel.selector.contains("[name=") {
            return 0.7;
        }
        if sel.selector.contains("[data-") || sel.selector.contains("[data_") {
            return 0.7;
        }
        if sel.selector.starts_with('.') {
            return 0.4;
        }
        0.2
    }

    /// Find the best selector for a given purpose from a list of scored selectors.
    pub fn pick_best_for_purpose(
        &self,
        scored: &[SelectorScore],
        purpose: &SelectorPurpose,
    ) -> Option<&SelectorScore> {
        scored
            .iter()
            .filter(|s| &s.purpose == purpose)
            .max_by(|a, b| a.adjusted_score.partial_cmp(&b.adjusted_score).unwrap())
    }

    /// Get the top N selectors across all purposes.
    pub fn top_n(&self, scored: &[SelectorScore], n: usize) -> Vec<&SelectorScore> {
        let mut sorted: Vec<&SelectorScore> = scored.iter().collect();
        sorted.sort_by(|a, b| b.adjusted_score.partial_cmp(&a.adjusted_score).unwrap());
        sorted.into_iter().take(n).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_selector(
        selector: &str,
        purpose: SelectorPurpose,
        match_count: u32,
    ) -> DiscoveredSelector {
        DiscoveredSelector {
            selector: selector.to_string(),
            target_type: SelectorTargetType::Input,
            purpose,
            confidence: 0.5,
            occurrence_count: match_count,
            source_url: "https://example.com/search".into(),
        }
    }

    #[test]
    fn test_id_selector_scores_high() {
        let sel = make_selector("#firstName", SelectorPurpose::FirstNameInput, 1);
        let scorer = SelectorScorer::default();
        let score = scorer.score(&sel);
        assert!(
            score.raw_score > 0.7,
            "id selector should score high, got {}",
            score.raw_score
        );
        assert!(score.adjusted_score > 0.7);
    }

    #[test]
    fn test_generic_selector_scores_low() {
        let sel = make_selector("div", SelectorPurpose::Other("div".into()), 50);
        let scorer = SelectorScorer::default();
        let score = scorer.score(&sel);
        assert!(
            score.raw_score < 0.5,
            "generic selector should score low, got {}",
            score.raw_score
        );
    }

    #[test]
    fn test_scoring_clamps_to_one() {
        let sel = make_selector("#firstName", SelectorPurpose::FirstNameInput, 1);
        let scorer = SelectorScorer::default();
        let score = scorer.score(&sel);
        assert!(score.adjusted_score >= 0.0 && score.adjusted_score <= 1.0);
    }

    #[test]
    fn test_pick_best_for_purpose() {
        let scored = vec![
            SelectorScore {
                selector: "#firstName".into(),
                purpose: SelectorPurpose::FirstNameInput,
                raw_score: 0.9,
                adjusted_score: 0.9,
                breakdown: ScoreBreakdown::default(),
            },
            SelectorScore {
                selector: "#firstName2".into(),
                purpose: SelectorPurpose::FirstNameInput,
                raw_score: 0.7,
                adjusted_score: 0.7,
                breakdown: ScoreBreakdown::default(),
            },
        ];
        let scorer = SelectorScorer::default();
        let best = scorer.pick_best_for_purpose(&scored, &SelectorPurpose::FirstNameInput);
        assert!(best.is_some());
        assert_eq!(best.unwrap().selector, "#firstName");
    }

    #[test]
    fn test_top_n() {
        let scored = vec![
            SelectorScore {
                selector: "a".into(),
                purpose: SelectorPurpose::Other("a".into()),
                raw_score: 0.9,
                adjusted_score: 0.9,
                breakdown: ScoreBreakdown::default(),
            },
            SelectorScore {
                selector: "b".into(),
                purpose: SelectorPurpose::Other("b".into()),
                raw_score: 0.5,
                adjusted_score: 0.5,
                breakdown: ScoreBreakdown::default(),
            },
            SelectorScore {
                selector: "c".into(),
                purpose: SelectorPurpose::Other("c".into()),
                raw_score: 0.7,
                adjusted_score: 0.7,
                breakdown: ScoreBreakdown::default(),
            },
        ];
        let scorer = SelectorScorer::default();
        let top = scorer.top_n(&scored, 2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].selector, "a");
        assert_eq!(top[1].selector, "c");
    }
}
