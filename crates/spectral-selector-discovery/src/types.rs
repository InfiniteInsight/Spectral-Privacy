//! Core types for the selector discovery system.

use serde::{Deserialize, Serialize};
use spectral_core::BrokerId;
use std::collections::HashMap;

/// Represents a discovered CSS selector and its properties.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveredSelector {
    /// The CSS selector string.
    pub selector: String,
    /// What the selector targets (input, container, link, button, text).
    pub target_type: SelectorTargetType,
    /// The semantic purpose (first_name_input, results_container, listing_url, etc.).
    pub purpose: SelectorPurpose,
    /// A confidence score from 0.0 to 1.0.
    pub confidence: f64,
    /// How many times this exact selector appeared on the page.
    pub occurrence_count: u32,
    /// The page URL where this selector was found.
    pub source_url: String,
}

/// What kind of DOM element the selector targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SelectorTargetType {
    Input,
    Button,
    Link,
    Container,
    Text,
    Image,
    Form,
    Other,
}

/// Semantic purpose of a discovered selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum SelectorPurpose {
    // --- Search form fields ---
    FirstNameInput,
    LastNameInput,
    CityInput,
    StateInput,
    PhoneInput,
    EmailInput,
    ListingUrlInput,
    SearchSubmitButton,
    SearchForm,

    // --- Result page elements ---
    ResultsContainer,
    ResultItem,
    ListingUrl,
    Name,
    Age,
    Location,
    Relatives,
    Phones,
    Emails,
    NoResultsIndicator,
    CaptchaRequired,

    // --- Removal form fields ---
    RemovalSubmitButton,
    RemovalForm,
    RemovalEmailInput,
    RemovalListingUrlInput,

    // --- Navigation ---
    NextPageButton,
    PaginationLink,
    SearchResultsCount,

    // --- Fallback / generic ---
    Other(String),
}

impl SelectorPurpose {
    /// Convert a TOML-formatted field name to a SelectorPurpose.
    pub fn from_toml_key(key: &str) -> Option<Self> {
        match key {
            "first_name_input" => Some(Self::FirstNameInput),
            "last_name_input" => Some(Self::LastNameInput),
            "city_input" => Some(Self::CityInput),
            "state_input" => Some(Self::StateInput),
            "phone_input" => Some(Self::PhoneInput),
            "email_input" => Some(Self::EmailInput),
            "listing_url_input" => Some(Self::ListingUrlInput),
            "submit_button" => Some(Self::SearchSubmitButton),
            "form" => Some(Self::SearchForm),
            "results_container" => Some(Self::ResultsContainer),
            "result_item" => Some(Self::ResultItem),
            "listing_url" => Some(Self::ListingUrl),
            "name" => Some(Self::Name),
            "age" => Some(Self::Age),
            "location" => Some(Self::Location),
            "relatives" => Some(Self::Relatives),
            "phones" => Some(Self::Phones),
            "emails" => Some(Self::Emails),
            "no_results_indicator" => Some(Self::NoResultsIndicator),
            "captcha_required" => Some(Self::CaptchaRequired),
            "removal_submit_button" => Some(Self::RemovalSubmitButton),
            "removal_form" => Some(Self::RemovalForm),
            "removal_email_input" => Some(Self::RemovalEmailInput),
            "removal_listing_url_input" => Some(Self::RemovalListingUrlInput),
            "next_page_button" => Some(Self::NextPageButton),
            "pagination_link" => Some(Self::PaginationLink),
            "search_results_count" => Some(Self::SearchResultsCount),
            _ => None,
        }
    }

    /// Convert a SelectorPurpose back to a TOML field name string.
    pub fn to_toml_key(&self) -> &str {
        match self {
            Self::FirstNameInput => "first_name_input",
            Self::LastNameInput => "last_name_input",
            Self::CityInput => "city_input",
            Self::StateInput => "state_input",
            Self::PhoneInput => "phone_input",
            Self::EmailInput => "email_input",
            Self::ListingUrlInput => "listing_url_input",
            Self::SearchSubmitButton => "submit_button",
            Self::SearchForm => "form",
            Self::ResultsContainer => "results_container",
            Self::ResultItem => "result_item",
            Self::ListingUrl => "listing_url",
            Self::Name => "name",
            Self::Age => "age",
            Self::Location => "location",
            Self::Relatives => "relatives",
            Self::Phones => "phones",
            Self::Emails => "emails",
            Self::NoResultsIndicator => "no_results_indicator",
            Self::CaptchaRequired => "captcha_required",
            Self::RemovalSubmitButton => "removal_submit_button",
            Self::RemovalForm => "removal_form",
            Self::RemovalEmailInput => "removal_email_input",
            Self::RemovalListingUrlInput => "removal_listing_url_input",
            Self::NextPageButton => "next_page_button",
            Self::PaginationLink => "pagination_link",
            Self::SearchResultsCount => "search_results_count",
            Self::Other(s) => s.as_str(),
        }
    }

    /// Get a human-readable label for UI display.
    pub fn label(&self) -> &str {
        match self {
            Self::FirstNameInput => "First Name Input",
            Self::LastNameInput => "Last Name Input",
            Self::CityInput => "City Input",
            Self::StateInput => "State Input",
            Self::PhoneInput => "Phone Input",
            Self::EmailInput => "Email Input",
            Self::ListingUrlInput => "Listing URL Input",
            Self::SearchSubmitButton => "Search Submit Button",
            Self::SearchForm => "Search Form",
            Self::ResultsContainer => "Results Container",
            Self::ResultItem => "Result Item",
            Self::ListingUrl => "Listing URL",
            Self::Name => "Name Field",
            Self::Age => "Age Field",
            Self::Location => "Location Field",
            Self::Relatives => "Relatives Field",
            Self::Phones => "Phone Numbers Field",
            Self::Emails => "Email Addresses Field",
            Self::NoResultsIndicator => "No Results Indicator",
            Self::CaptchaRequired => "CAPTCHA Required",
            Self::RemovalSubmitButton => "Removal Submit Button",
            Self::RemovalForm => "Removal Form",
            Self::RemovalEmailInput => "Removal Email Input",
            Self::RemovalListingUrlInput => "Removal Listing URL Input",
            Self::NextPageButton => "Next Page Button",
            Self::PaginationLink => "Pagination Link",
            Self::SearchResultsCount => "Search Results Count",
            Self::Other(s) => s.as_str(),
        }
    }
}

/// A DOM element discovered during page analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredElement {
    /// The CSS selector for this element.
    pub selector: String,
    /// The HTML tag name (e.g., "input", "a", "div").
    pub tag: String,
    /// The element's id attribute, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The element's name attribute, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The element's class attributes, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classes: Option<Vec<String>>,
    /// The element's type attribute (for inputs), if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_attr: Option<String>,
    /// The visible text content (truncated to 100 chars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The XPath of this element.
    pub xpath: String,
    /// How many times this exact selector matches on the page.
    pub match_count: u32,
    /// The length of the selector string.
    pub selector_length: usize,
}

impl DiscoveredElement {
    /// Estimate whether this element is a good candidate for a search form field.
    #[must_use]
    pub fn is_form_input_candidate(&self) -> bool {
        let is_input = self.tag == "input" || self.tag == "select" || self.tag == "textarea";
        let is_textlike = matches!(
            self.type_attr.as_deref(),
            None | Some("text") | Some("email") | Some("phone") | Some("number")
        );
        let has_useful_attr = self.id.is_some() || self.name.is_some() || self.classes.is_some();
        is_input && is_textlike && has_useful_attr
    }

    /// Estimate whether this element is a submit button.
    #[must_use]
    pub fn is_button_candidate(&self) -> bool {
        if self.tag == "button" {
            return true;
        }
        matches!(self.type_attr.as_deref(), Some("submit") | Some("image")) && self.tag == "input"
    }

    /// Estimate whether this element is a link (for pagination / listings).
    #[must_use]
    pub fn is_link_candidate(&self) -> bool {
        self.tag == "a" && self.selector_length < 100
    }

    /// Estimate whether this element is a potential results container.
    #[must_use]
    pub fn is_container_candidate(&self) -> bool {
        matches!(
            self.tag.as_str(),
            "div" | "section" | "article" | "ul" | "table"
        ) && self.match_count <= 3
            && self.selector_length < 80
    }
}

/// A scoring report for a single discovered selector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectorScore {
    pub selector: String,
    pub purpose: SelectorPurpose,
    pub raw_score: f64,
    pub adjusted_score: f64,
    pub breakdown: ScoreBreakdown,
}

/// Detailed scoring breakdown for transparency and debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub uniqueness_bonus: f64,
    pub brevity_bonus: f64,
    pub attribute_bonus: f64,
    pub semantic_bonus: f64,
    pub stability_penalty: f64,
    pub specificity_score: f64,
}

/// Result of a single discovery scan across one or more broker pages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    pub broker_id: BrokerId,
    pub url: String,
    pub elements: Vec<DiscoveredElement>,
    pub candidate_selectors: Vec<DiscoveredSelector>,
    pub scored_selectors: Vec<SelectorScore>,
}

/// Configuration for a discovery scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Which pages to analyze.
    pub urls: Vec<String>,
    /// Minimum confidence threshold to include in results.
    pub min_confidence: f64,
    /// Maximum number of selectors to return per purpose.
    pub max_selectors_per_purpose: u32,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            urls: Vec::new(),
            min_confidence: 0.5,
            max_selectors_per_purpose: 3,
        }
    }
}

/// A merged set of selectors for a single broker, combining
/// TOML config + discovered candidates + scored results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedSelectors {
    pub broker_id: BrokerId,
    pub search_form_selectors: HashMap<String, String>,
    pub result_selectors: HashMap<String, String>,
    pub removal_selectors: HashMap<String, String>,
    pub discovered_candidates: Vec<DiscoveredSelector>,
    pub score_details: HashMap<String, SelectorScore>,
}

impl MergedSelectors {
    /// Serialize to a TOML fragment suitable for appending to a broker definition.
    pub fn to_toml_fragment(&self) -> anyhow::Result<String> {
        let mut map = serde_json::Map::new();

        let search_sel =
            self.search_form_selectors
                .iter()
                .fold(serde_json::Map::new(), |mut m, (k, v)| {
                    m.insert(k.clone(), serde_json::Value::String(v.clone()));
                    m
                });
        map.insert(
            "form_selectors".to_string(),
            serde_json::Value::Object(search_sel),
        );

        let result_sel =
            self.result_selectors
                .iter()
                .fold(serde_json::Map::new(), |mut m, (k, v)| {
                    m.insert(k.clone(), serde_json::Value::String(v.clone()));
                    m
                });
        map.insert(
            "result_selectors".to_string(),
            serde_json::Value::Object(result_sel),
        );

        if !self.removal_selectors.is_empty() {
            let removal_sel =
                self.removal_selectors
                    .iter()
                    .fold(serde_json::Map::new(), |mut m, (k, v)| {
                        m.insert(k.clone(), serde_json::Value::String(v.clone()));
                        m
                    });
            map.insert(
                "removal_form_selectors".to_string(),
                serde_json::Value::Object(removal_sel),
            );
        }

        let value = serde_json::Value::Object(map);
        Ok(toml::to_string_pretty(&value)?)
    }
}

/// Health status for a selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectorHealth {
    Healthy,
    Degraded,
    Broken,
    Unknown,
}

/// Health check result for a single selector on a broker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectorHealthReport {
    pub broker_id: BrokerId,
    pub selector_purpose: String,
    pub current_selector: String,
    pub health: SelectorHealth,
    pub last_checked: String,
    pub error_message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selector_purpose_roundtrip() {
        let purposes = [
            SelectorPurpose::FirstNameInput,
            SelectorPurpose::LastNameInput,
            SelectorPurpose::CityInput,
            SelectorPurpose::StateInput,
            SelectorPurpose::PhoneInput,
            SelectorPurpose::EmailInput,
            SelectorPurpose::ListingUrlInput,
            SelectorPurpose::SearchSubmitButton,
            SelectorPurpose::SearchForm,
            SelectorPurpose::ResultsContainer,
            SelectorPurpose::ResultItem,
            SelectorPurpose::ListingUrl,
            SelectorPurpose::Name,
            SelectorPurpose::Age,
            SelectorPurpose::Location,
            SelectorPurpose::Relatives,
            SelectorPurpose::Phones,
            SelectorPurpose::Emails,
            SelectorPurpose::NoResultsIndicator,
            SelectorPurpose::CaptchaRequired,
            SelectorPurpose::NextPageButton,
            SelectorPurpose::PaginationLink,
            SelectorPurpose::SearchResultsCount,
        ];
        for p in purposes {
            let key = p.to_toml_key();
            assert_eq!(
                SelectorPurpose::from_toml_key(key).unwrap(),
                p,
                "roundtrip failed for {key}"
            );
        }
    }

    #[test]
    fn test_unrecognized_toml_key_returns_none() {
        assert!(SelectorPurpose::from_toml_key("bogus_field").is_none());
    }

    #[test]
    fn test_discovered_element_form_input() {
        let el = DiscoveredElement {
            selector: "#firstName".into(),
            tag: "input".into(),
            id: Some("firstName".into()),
            name: None,
            classes: None,
            type_attr: Some("text".into()),
            text: None,
            xpath: "//input[@id='firstName']".into(),
            match_count: 1,
            selector_length: 10,
        };
        assert!(el.is_form_input_candidate());
    }

    #[test]
    fn test_discovered_element_non_input() {
        let el = DiscoveredElement {
            selector: "div.container".into(),
            tag: "div".into(),
            id: None,
            name: None,
            classes: Some(vec!["container".into()]),
            type_attr: None,
            text: None,
            xpath: "//div[@class='container']".into(),
            match_count: 1,
            selector_length: 13,
        };
        assert!(!el.is_form_input_candidate());
        assert!(el.is_container_candidate());
    }

    #[test]
    fn test_discovered_element_button() {
        let el = DiscoveredElement {
            selector: "#searchBtn".into(),
            tag: "button".into(),
            id: Some("searchBtn".into()),
            name: None,
            classes: None,
            type_attr: None,
            text: Some("Search".into()),
            xpath: "//button[@id='searchBtn']".into(),
            match_count: 1,
            selector_length: 9,
        };
        assert!(el.is_button_candidate());
    }
}
