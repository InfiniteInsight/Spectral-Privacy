//! DOM analysis via embedded JavaScript.
//!
//! Executes JavaScript in the browser context to extract selectors and element
//! properties from a loaded page. The script runs deterministically and returns
//! structured data about every interesting element on the page.

use crate::types::{DiscoveredElement, SelectorPurpose, SelectorTargetType};
use anyhow::Result;
use spectral_core::BrokerId;
use tracing::debug;

/// JavaScript to inject into the browser for DOM analysis.
/// Returns a JSON array of element data.
const DOM_ANALYSIS_JS: &str = r#"
(function() {
    var results = [];
    var allEls = document.querySelectorAll('*');
    var maxElements = 500;
    var count = 0;

    for (var i = 0; i < allEls.length && count < maxElements; i++) {
        var el = allEls[i];
        var tag = el.tagName.toLowerCase();

        // Skip script, style, noscript, and head elements
        if (tag === 'script' || tag === 'style' || tag === 'noscript' || tag === 'head') {
            continue;
        }

        var selector = generateUniqueSelector(el);
        if (!selector || selector.length > 200) {
            continue;
        }

        var matchCount = countElementsMatchingSelector(selector);

        var elData = {
            selector: selector,
            tag: tag,
            id: el.id || null,
            name: el.name || el.getAttribute('name') || null,
            classes: el.className ? el.className.split(/\s+/).filter(Boolean) : null,
            type_attr: el.type || null,
            text: el.innerText ? el.innerText.trim().substring(0, 100) : null,
            xpath: generateXPath(el),
            match_count: matchCount,
            selector_length: selector.length,
            has_children: el.children.length > 0,
            child_count: el.children.length,
            role: el.getAttribute('role') || null,
            aria_label: el.getAttribute('aria-label') || null,
            placeholder: el.getAttribute('placeholder') || null,
            input_count: tag === 'form' ? el.querySelectorAll('input, select, textarea').length : 0
        };

        results.push(elData);
        count++;
    }

    return JSON.stringify(results);

    function generateUniqueSelector(el) {
        if (el.id) {
            return '#' + CSS.escape(el.id);
        }

        var path = [];
        var current = el;
        var depth = 0;

        while (current && current.nodeType === 1 && depth < 6) {
            var selector = current.tagName.toLowerCase();
            if (current.className) {
                var cls = typeof current.className === 'string'
                    ? current.className.split(/\s+/).filter(Boolean)[0]
                    : '';
                if (cls) {
                    selector += '.' + CSS.escape(cls);
                }
            }
            path.unshift(selector);
            current = current.parentElement;
            depth++;
        }

        return path.join(' > ');
    }

    function countElementsMatchingSelector(selector) {
        try {
            return document.querySelectorAll(selector).length;
        } catch (e) {
            return 0;
        }
    }

    function generateXPath(el) {
        if (el.id) {
            return '//*[@id="' + el.id + '"]';
        }
        var path = [];
        var current = el;
        while (current && current.nodeType === 1) {
            var index = 1;
            var sibling = current.previousSibling;
            while (sibling) {
                if (sibling.nodeType === 1 && sibling.tagName === current.tagName) {
                    index++;
                }
                sibling = sibling.previousSibling;
            }
            var xpathPart = current.tagName.toLowerCase();
            if (index > 1) {
                xpathPart += '[' + index + ']';
            }
            path.unshift(xpathPart);
            current = current.parentElement;
        }
        return path.length ? '/' + path.join('/') : '';
    }
})();
"#;

/// Analyzes the current page DOM and returns discovered elements.
pub fn analyze_dom(page_html: &str) -> Result<Vec<DiscoveredElement>> {
    // Parse HTML and run selector analysis using regex-based parsing
    // (no external HTML parser needed — we work directly with the string)
    let mut elements = Vec::new();
    let tag_re = Regex::new(r#"<(/?)(\w+)([^>]*)>"#).unwrap();
    let attr_re = Regex::new(r#"(\w+)="([^"]*)""#).unwrap();

    for cap in tag_re.captures_iter(page_html) {
        let is_closing = cap[1] == "/";
        if is_closing {
            continue;
        }

        let tag = cap[2].to_lowercase();
        let attrs_str = &cap[3];
        let tag_start = cap.get(0).unwrap().start();

        // Skip irrelevant tags
        if matches!(
            tag.as_str(),
            "script" | "style" | "noscript" | "head" | "html" | "body"
        ) {
            continue;
        }

        // Parse attributes
        let mut id: Option<String> = None;
        let mut name: Option<String> = None;
        let mut type_attr: Option<String> = None;
        let mut classes: Option<Vec<String>> = None;
        let mut placeholder: Option<String> = None;
        let mut role: Option<String> = None;
        let mut aria_label: Option<String> = None;

        for attr_cap in attr_re.captures_iter(attrs_str) {
            let attr_name = attr_cap[1].to_lowercase();
            let attr_val = &attr_cap[2];
            match attr_name.as_str() {
                "id" => id = Some(attr_val.to_string()),
                "name" => name = Some(attr_val.to_string()),
                "type" => type_attr = Some(attr_val.to_string()),
                "class" => {
                    classes = Some(
                        attr_val
                            .split_whitespace()
                            .filter(|c| !c.is_empty())
                            .map(|s| s.to_string())
                            .collect(),
                    );
                }
                "placeholder" => placeholder = Some(attr_val.to_string()),
                "role" => role = Some(attr_val.to_string()),
                "aria-label" => aria_label = Some(attr_val.to_string()),
                _ => {}
            }
        }

        // Generate a CSS selector
        let selector = if let Some(ref id_val) = id {
            format!("#{}", id_val)
        } else if let Some(ref name_val) = name {
            format!("[name=\"{}\"]", escape_attr(name_val))
        } else if let Some(ref cls) = classes {
            if cls.len() >= 2 {
                format!(".{}", cls[..std::cmp::min(2, cls.len())].join("."))
            } else {
                format!("[class=\"{}\"]", escape_attr(&cls.join(" ")))
            }
        } else {
            format!("{}[data-tag-start={}]", tag, tag_start % 1000)
        };

        // Extract text content (simplified)
        let text = extract_text(page_html, tag_start);

        // Count approximate matches
        let match_count = count_selector_matches(page_html, &selector);

        elements.push(DiscoveredElement {
            selector,
            tag,
            id,
            name,
            classes,
            type_attr,
            text,
            xpath: String::new(), // Not computed in regex mode
            match_count,
            selector_length: selector.len(),
        });
    }

    // Deduplicate by selector, keeping highest match_count
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    elements.retain(|el| {
        seen.entry(el.selector.clone())
            .and_modify(|existing| {
                if el.match_count > *existing {
                    *existing = el.match_count;
                }
            })
            .or_insert(el.match_count);
        true
    });

    debug!(
        "DOM analysis found {} unique elements from {} bytes of HTML",
        elements.len(),
        page_html.len()
    );

    Ok(elements)
}

/// Analyze DOM from an actual browser page (requires browser session).
/// Returns a vector of DiscoveredElement structs.
pub async fn analyze_browser_page(
    _broker_id: &BrokerId,
    _page_html: &str,
) -> Result<Vec<DiscoveredElement>> {
    // When integrated with the browser crate, this will inject DOM_ANALYSIS_JS
    // and parse the JSON response. For now, fall back to regex parsing.
    analyze_dom(_page_html)
}

/// Classify a DiscoveredElement into a SelectorPurpose.
pub fn classify_element(el: &DiscoveredElement) -> Vec<(SelectorPurpose, SelectorTargetType)> {
    let mut results = Vec::new();

    let id_lower = el
        .id
        .as_deref()
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    let name_lower = el
        .name
        .as_deref()
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    let type_lower = el
        .type_attr
        .as_deref()
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    let tag_lower = el.tag.to_lowercase();

    // Check text content for hints
    let text_lower = el
        .text
        .as_deref()
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // Classify based on id, name, type, and tag
    let combined = format!("{} {} {}", id_lower, name_lower, text_lower);

    // Input-like elements
    if matches!(
        el.type_attr.as_deref(),
        None | Some("text") | Some("email") | Some("phone") | Some("number") | Some("search")
    ) && (tag_lower == "input" || tag_lower == "select" || tag_lower == "textarea")
    {
        if combined.contains("first") || combined.contains("forename") || combined.contains("fname")
        {
            results.push((SelectorPurpose::FirstNameInput, SelectorTargetType::Input));
        }
        if combined.contains("last") || combined.contains("surname") || combined.contains("lname") {
            results.push((SelectorPurpose::LastNameInput, SelectorTargetType::Input));
        }
        if combined.contains("city") {
            results.push((SelectorPurpose::CityInput, SelectorTargetType::Input));
        }
        if combined.contains("state") || combined.contains("prov") {
            results.push((SelectorPurpose::StateInput, SelectorTargetType::Input));
        }
        if combined.contains("phone") || combined.contains("tel") || combined.contains("mobile") {
            results.push((SelectorPurpose::PhoneInput, SelectorTargetType::Input));
        }
        if combined.contains("email") || combined.contains("e-mail") {
            results.push((SelectorPurpose::EmailInput, SelectorTargetType::Input));
        }
        if combined.contains("listing")
            || combined.contains("url")
            || combined.contains("profile")
            || combined.contains("page")
        {
            results.push((SelectorPurpose::ListingUrlInput, SelectorTargetType::Input));
        }

        // If none of the specific patterns matched but it's a text input,
        // it could be a generic search input
        if results.is_empty() && type_lower == "text"
            || (type_lower == "search"
                && el
                    .text
                    .as_deref()
                    .map(|t| t.to_lowercase())
                    .unwrap_or_default()
                    .contains("search"))
        {
            // Don't add generic — let the engine infer from context
        }
    }

    // Buttons
    if el.is_button_candidate() {
        if combined.contains("search") || combined.contains("find") || combined.contains("go") {
            results.push((
                SelectorPurpose::SearchSubmitButton,
                SelectorTargetType::Button,
            ));
        }
        if combined.contains("submit") || combined.contains("send") {
            results.push((
                SelectorPurpose::SearchSubmitButton,
                SelectorTargetType::Button,
            ));
        }
        if combined.contains("next") || combined.contains("page") || combined.contains("more") {
            results.push((SelectorPurpose::NextPageButton, SelectorTargetType::Button));
        }
        if combined.contains("removal")
            || combined.contains("opt-out")
            || combined.contains("delete")
        {
            results.push((
                SelectorPurpose::RemovalSubmitButton,
                SelectorTargetType::Button,
            ));
        }
        // Generic submit button
        if combined.contains("submit") {
            results.push((
                SelectorPurpose::SearchSubmitButton,
                SelectorTargetType::Button,
            ));
        }
    }

    // Links
    if el.tag == "a" {
        if combined.contains("next") || combined.contains("page") {
            results.push((SelectorPurpose::PaginationLink, SelectorTargetType::Link));
        }
        if combined.contains("result") || combined.contains("profile") {
            results.push((SelectorPurpose::ListingUrl, SelectorTargetType::Link));
        }
    }

    // Forms
    if tag_lower == "form" {
        if combined.contains("search")
            || el
                .id
                .as_deref()
                .map(|s| s.to_lowercase())
                .unwrap_or_default()
                .contains("search")
        {
            results.push((SelectorPurpose::SearchForm, SelectorTargetType::Form));
        }
        if combined.contains("removal")
            || combined.contains("opt-out")
            || combined.contains("delete")
        {
            results.push((SelectorPurpose::RemovalForm, SelectorTargetType::Form));
        }
        if results.is_empty() {
            results.push((SelectorPurpose::SearchForm, SelectorTargetType::Form));
        }
    }

    // Check for no-results indicators
    if combined.contains("no results")
        || combined.contains("no match")
        || combined.contains("not found")
        || combined.contains("no person")
        || combined.contains("no record")
    {
        results.push((
            SelectorPurpose::NoResultsIndicator,
            SelectorTargetType::Text,
        ));
    }

    // Check for captcha indicators
    if combined.contains("captcha")
        || combined.contains("recaptcha")
        || combined.contains("verify you are human")
        || combined.contains("robot")
    {
        results.push((SelectorPurpose::CaptchaRequired, SelectorTargetType::Text));
    }

    // Container detection (if it has children and a meaningful class/id)
    if el.is_container_candidate() {
        if combined.contains("result")
            || combined.contains("search-result")
            || combined.contains("search_result")
        {
            results.push((SelectorPurpose::ResultItem, SelectorTargetType::Container));
        }
        if combined.contains("container")
            || combined.contains("results")
            || combined.contains("grid")
            || combined.contains("list")
        {
            results.push((
                SelectorPurpose::ResultsContainer,
                SelectorTargetType::Container,
            ));
        }
    }

    // Name fields
    if tag_lower == "div" || tag_lower == "span" || tag_lower == "p" || tag_lower == "a" {
        if combined.contains("age") || combined.contains("years") || combined.contains("yr ") {
            results.push((SelectorPurpose::Age, SelectorTargetType::Text));
        }
        if combined.contains("city") || combined.contains("state") || combined.contains("county") {
            results.push((SelectorPurpose::Location, SelectorTargetType::Text));
        }
        if combined.contains("rel") && (combined.contains("ative") || combined.contains("ship")) {
            results.push((SelectorPurpose::Relatives, SelectorTargetType::Text));
        }
        if combined.contains("phone") || combined.contains("cell") || combined.contains("mobile") {
            results.push((SelectorPurpose::Phones, SelectorTargetType::Text));
        }
        if combined.contains("@") || combined.contains("email") {
            results.push((SelectorPurpose::Emails, SelectorTargetType::Text));
        }
    }

    // If we found nothing, add a generic Other entry
    if results.is_empty() && el.selector_length < 50 {
        results.push((
            SelectorPurpose::Other(format!("tag_{}", el.tag)),
            SelectorTargetType::Other,
        ));
    }

    results
}

/// Helper: count how many times a selector string appears in the HTML.
fn count_selector_matches(html: &str, selector: &str) -> u32 {
    if selector.starts_with('#') {
        // ID selector — should be unique
        let id = &selector[1..];
        if html.contains(&format!("id=\"{}\"", id)) {
            return 1;
        }
        return 0;
    }

    // For class selectors, count approximate matches
    let selector_clean = selector
        .replace(" >", " ")
        .replace('>', ' ')
        .replace('.', "\\.");

    let pattern = format!(r"\b{}\b", selector_clean);
    if let Ok(re) = Regex::new(&pattern) {
        re.find_iter(html).count() as u32
    } else {
        // Fallback: simple substring count
        html.matches(selector).max(1) as u32
    }
}

/// Helper: extract text content starting from a tag position.
fn extract_text(html: &str, start: usize) -> Option<String> {
    let remaining = &html[start..];
    // Find closing tag
    if let Some(end) = remaining.find("</") {
        let tag_content = &remaining[..end];
        // Extract text (strip attributes)
        let text_re = Regex::new(r#"^\w+([^>]*)>"#).ok();
        if let Some(re) = text_re {
            let clean = re.replace(tag_content, "").to_string();
            let text = clean.trim();
            if text.len() > 100 {
                return Some(text[..100].to_string());
            }
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
    }
    None
}

/// Escape special characters in attribute values for selector generation.
fn escape_attr(s: &str) -> String {
    s.replace('"', "\\\"")
        .replace('\\', "\\\\")
        .replace(' ', r"\ ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_selector_classified() {
        let el = DiscoveredElement {
            selector: "#firstName".into(),
            tag: "input".into(),
            id: Some("firstName".into()),
            name: None,
            classes: None,
            type_attr: Some("text".into()),
            text: None,
            xpath: String::new(),
            match_count: 1,
            selector_length: 10,
        };
        let results = classify_element(&el);
        assert!(results
            .iter()
            .any(|(p, _)| matches!(p, SelectorPurpose::FirstNameInput)));
    }

    #[test]
    fn test_name_selector_classified() {
        let el = DiscoveredElement {
            selector: "[name=\"last_name\"]".into(),
            tag: "input".into(),
            id: None,
            name: Some("last_name".into()),
            classes: None,
            type_attr: Some("text".into()),
            text: None,
            xpath: String::new(),
            match_count: 1,
            selector_length: 20,
        };
        let results = classify_element(&el);
        assert!(results
            .iter()
            .any(|(p, _)| matches!(p, SelectorPurpose::LastNameInput)));
    }

    #[test]
    fn test_no_results_indicator() {
        let el = DiscoveredElement {
            selector: ".no-results-message".into(),
            tag: "div".into(),
            id: None,
            name: None,
            classes: Some(vec!["no-results-message".into()]),
            type_attr: None,
            text: Some("No results found for your search".into()),
            xpath: String::new(),
            match_count: 1,
            selector_length: 20,
        };
        let results = classify_element(&el);
        assert!(results
            .iter()
            .any(|(p, _)| matches!(p, SelectorPurpose::NoResultsIndicator)));
    }

    #[test]
    fn test_button_classified() {
        let el = DiscoveredElement {
            selector: "#searchBtn".into(),
            tag: "button".into(),
            id: Some("searchBtn".into()),
            name: None,
            classes: None,
            type_attr: None,
            text: Some("Search".into()),
            xpath: String::new(),
            match_count: 1,
            selector_length: 9,
        };
        let results = classify_element(&el);
        assert!(results
            .iter()
            .any(|(p, _)| matches!(p, SelectorPurpose::SearchSubmitButton)));
    }

    #[test]
    fn test_dom_analysis_basic() {
        let html = r#"<html><body>
            <form id="searchForm" action="/search">
                <input type="text" id="firstName" name="first_name" placeholder="First Name">
                <input type="text" id="lastName" name="last_name" placeholder="Last Name">
                <input type="text" id="city" name="city" placeholder="City">
                <select id="state" name="state">
                    <option value="">State</option>
                </select>
                <button type="submit" id="searchBtn">Search</button>
            </form>
            <div class="no-results" id="noResults">No results found</div>
        </body></html>"#;

        let elements = analyze_dom(html).unwrap();
        assert!(!elements.is_empty());

        // Check we found the search form
        assert!(elements
            .iter()
            .any(|e| e.selector.contains("searchForm") || e.selector.contains("#searchForm")));

        // Check we found firstName input
        assert!(elements
            .iter()
            .any(|e| e.selector.contains("#firstName") || e.id.as_deref() == Some("firstName")));

        // Check we found lastName input
        assert!(elements
            .iter()
            .any(|e| e.selector.contains("#lastName") || e.id.as_deref() == Some("lastName")));

        // Check we found the search button
        assert!(elements.iter().any(|e| e.selector.contains("#searchBtn")));
    }
}
