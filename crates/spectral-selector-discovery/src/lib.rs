//! spectral-selector-discovery: automatically discovers and scores CSS selectors
//! for broker search forms, result pages, and removal forms.
//!
//! # Workflow
//! 1. **DOM Analysis** — injects JS or parses raw HTML to extract element selectors
//! 2. **Classification** — maps raw elements to semantic purposes (FirstNameInput, etc.)
//! 3. **Scoring** — ranks selectors by uniqueness, brevity, stability, semantic fit
//! 4. **Merging** — combines discovered selectors with existing TOML definitions
//! 5. **Health Checking** — periodically validates that current selectors still work
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐     ┌──────────────┐     ┌─────────────┐
//! │  DOM Analyzer │──▶│  Discovery   │──▶│  Selector   │
//! │  (DOM + JS)  │     │  Engine      │     │  Scorer     │
//! └─────────────┘     └──────────────┘     └─────────────┘
//!                                              │
//!                                              ▼
//!                                      ┌──────────────┐
//!                                      │  TOML Merger │
//!                                      └──────────────┘
//! ```

pub mod dom_analyzer;
pub mod engine;
pub mod scorer;
pub mod toml_merger;
pub mod types;

pub use dom_analyzer::{analyze_dom, classify_element};
pub use engine::{consolidate_results, run_discovery};
pub use scorer::SelectorScorer;
pub use toml_merger::{
    merge_with_existing, merge_with_existing_from_files, merge_with_existing_from_paths,
    MergedSelectorsBuilder,
};
pub use types::*;
