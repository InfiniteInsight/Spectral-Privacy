use crate::error::{Result, ScanError};
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use spectral_broker::definition::ResultSelectors;
use spectral_core::BrokerId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingMatch {
    pub listing_url: String,
    pub extracted_data: ExtractedData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedData {
    pub name: Option<String>,
    pub age: Option<u32>,
    pub addresses: Vec<String>,
    pub phone_numbers: Vec<String>,
    pub relatives: Vec<String>,
    pub emails: Vec<String>,
}

pub struct ResultParser<'a> {
    selectors: &'a ResultSelectors,
    base_url: String,
}

impl<'a> ResultParser<'a> {
    pub fn new(selectors: &'a ResultSelectors, base_url: String) -> Self {
        Self {
            selectors,
            base_url,
        }
    }

    pub fn parse(&self, html: &str) -> Result<Vec<ListingMatch>> {
        let document = Html::parse_document(html);

        // Check for CAPTCHA
        if let Some(captcha_sel) = &self.selectors.captcha_required {
            if let Ok(selector) = Selector::parse(captcha_sel) {
                if document.select(&selector).next().is_some() {
                    return Err(ScanError::CaptchaRequired {
                        broker_id: BrokerId::new("test-broker").expect("valid broker ID"),
                    });
                }
            }
        }

        // Check for no results
        if let Some(no_results_sel) = &self.selectors.no_results_indicator {
            if let Ok(selector) = Selector::parse(no_results_sel) {
                if document.select(&selector).next().is_some() {
                    return Ok(vec![]);
                }
            }
        }

        // Parse results
        let _container_selector =
            Selector::parse(&self.selectors.results_container).map_err(|e| {
                ScanError::SelectorsOutdated {
                    broker_id: BrokerId::new("test-broker").expect("valid broker ID"),
                    reason: format!("Invalid container selector: {}", e),
                }
            })?;

        let item_selector = Selector::parse(&self.selectors.result_item).map_err(|e| {
            ScanError::SelectorsOutdated {
                broker_id: BrokerId::new("test-broker").expect("valid broker ID"),
                reason: format!("Invalid item selector: {}", e),
            }
        })?;

        let mut matches = Vec::new();

        for item in document.select(&item_selector) {
            if let Some(listing_match) = self.parse_item(&item)? {
                matches.push(listing_match);
            }
        }

        Ok(matches)
    }

    fn parse_item(&self, element: &ElementRef) -> Result<Option<ListingMatch>> {
        // Extract listing URL
        let url_selector = Selector::parse(&self.selectors.listing_url).map_err(|e| {
            ScanError::SelectorsOutdated {
                broker_id: BrokerId::new("test-broker").expect("valid broker ID"),
                reason: format!("Invalid URL selector: {}", e),
            }
        })?;

        let listing_url = element
            .select(&url_selector)
            .next()
            .and_then(|el| el.value().attr("href"))
            .map(|href| {
                if href.starts_with("http") {
                    href.to_string()
                } else {
                    format!("{}{}", self.base_url, href)
                }
            });

        if listing_url.is_none() {
            return Ok(None);
        }

        // Extract data fields
        let name = self.extract_text(element, &self.selectors.name);
        let age = self
            .extract_text(element, &self.selectors.age)
            .and_then(|s| s.parse::<u32>().ok());
        let location = self.extract_text(element, &self.selectors.location);

        Ok(Some(ListingMatch {
            listing_url: listing_url.expect("listing_url is Some after is_none check"),
            extracted_data: ExtractedData {
                name,
                age,
                addresses: location.into_iter().collect(),
                phone_numbers: vec![],
                relatives: vec![],
                emails: vec![],
            },
        }))
    }

    fn extract_text(&self, element: &ElementRef, selector: &Option<String>) -> Option<String> {
        selector.as_ref().and_then(|sel| {
            Selector::parse(sel)
                .ok()
                .and_then(|s| element.select(&s).next())
                .map(|el| el.text().collect::<String>().trim().to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_search_results() {
        let html = r#"
            <div class="search-results">
                <div class="result-card">
                    <a class="profile-link" href="/profile/john-doe-123">View Profile</a>
                    <div class="name">John Doe</div>
                    <div class="age">35</div>
                    <div class="location">Springfield, CA</div>
                </div>
                <div class="result-card">
                    <a class="profile-link" href="/profile/jane-doe-456">View Profile</a>
                    <div class="name">Jane Doe</div>
                    <div class="age">32</div>
                    <div class="location">Los Angeles, CA</div>
                </div>
            </div>
        "#;

        let selectors = ResultSelectors {
            results_container: ".search-results".to_string(),
            result_item: ".result-card".to_string(),
            listing_url: "a.profile-link".to_string(),
            name: Some(".name".to_string()),
            age: Some(".age".to_string()),
            location: Some(".location".to_string()),
            relatives: None,
            phones: None,
            emails: None,
            no_results_indicator: None,
            captcha_required: None,
        };

        let parser = ResultParser::new(&selectors, "https://example.com".to_string());
        let matches = parser.parse(html).expect("parse should succeed");

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].extracted_data.name, Some("John Doe".to_string()));
        assert_eq!(matches[0].extracted_data.age, Some(35));
        assert_eq!(
            matches[0].listing_url,
            "https://example.com/profile/john-doe-123"
        );
    }
}
