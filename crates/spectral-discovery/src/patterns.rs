//! PII pattern matching engine

use crate::types::{AddressInfo, PiiMatch, PiiType, ScanConfig, UserPii};
use regex::{Regex, RegexBuilder};

/// Pattern matcher for finding user-specific PII in text
pub struct PiiPatterns {
    email_patterns: Vec<(String, Regex)>,
    phone_patterns: Vec<(String, Vec<Regex>)>,
    ssn_pattern: Option<(String, Regex)>,
    address_patterns: Vec<AddressPattern>,
    name_patterns: Vec<(String, Regex)>,
    dob_pattern: Option<(String, Regex)>,
    #[allow(dead_code)]
    config: ScanConfig,
}

struct AddressPattern {
    original: AddressInfo,
    street_regex: Option<Regex>,
    city_regex: Option<Regex>,
    state_regex: Option<Regex>,
    zip_regex: Option<Regex>,
}

impl PiiPatterns {
    /// Create a new pattern matcher from user PII and scan configuration
    #[must_use]
    pub fn new(user_pii: &UserPii, config: &ScanConfig) -> Self {
        let mut patterns = Self {
            email_patterns: Vec::new(),
            phone_patterns: Vec::new(),
            ssn_pattern: None,
            address_patterns: Vec::new(),
            name_patterns: Vec::new(),
            dob_pattern: None,
            config: config.clone(),
        };

        if config.scan_emails {
            patterns.compile_emails(&user_pii.emails);
        }
        if config.scan_phones {
            patterns.compile_phones(&user_pii.phones);
        }
        if config.scan_ssn {
            if let Some(ssn) = &user_pii.ssn {
                patterns.compile_ssn(ssn);
            }
        }
        if config.scan_addresses {
            patterns.compile_addresses(&user_pii.addresses);
        }
        if config.scan_names {
            patterns.compile_names(&user_pii.names);
        }
        if config.scan_dob {
            if let Some(dob) = &user_pii.date_of_birth {
                patterns.compile_dob(dob);
            }
        }

        patterns
    }

    fn compile_emails(&mut self, emails: &[String]) {
        for email in emails {
            // Add word boundaries to prevent matching email as substring
            let pattern = format!(r"\b{}\b", regex::escape(&email.to_lowercase()));
            if let Ok(regex) = RegexBuilder::new(&pattern)
                .case_insensitive(true)
                .build()
            {
                self.email_patterns.push((email.clone(), regex));
            }
        }
    }

    fn compile_phones(&mut self, phones: &[String]) {
        for phone in phones {
            // nosemgrep: use-zeroize-for-secrets
            let normalized: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
            if normalized.len() >= 10 {
                let mut regexes = Vec::new();
                let (area, prefix, line) =
                    (&normalized[0..3], &normalized[3..6], &normalized[6..10]);

                // Add word boundaries to prevent matching phone as part of longer number
                let patterns = [
                    format!(r"\({area}\)\s*{prefix}-{line}"),
                    format!(r"\b{area}-{prefix}-{line}\b"),
                    format!(r"\b{area}\.{prefix}\.{line}\b"),
                    format!(r"\b{}\b", normalized),
                ];

                tracing::debug!("Compiling phone patterns for {}: {:?}", phone, patterns);

                for pattern in patterns {
                    if let Ok(regex) = Regex::new(&pattern) {
                        regexes.push(regex);
                    }
                }

                if !regexes.is_empty() {
                    self.phone_patterns.push((phone.clone(), regexes));
                }
            }
        }
    }

    fn compile_ssn(&mut self, ssn: &str) {
        // nosemgrep: use-zeroize-for-secrets
        let normalized: String = ssn.chars().filter(|c| c.is_ascii_digit()).collect();
        if normalized.len() == 9 {
            // Add word boundaries to prevent matching SSN as part of longer number sequence
            let pattern = format!(
                r"\b{}[-\s]?{}[-\s]?{}\b",
                &normalized[0..3],
                &normalized[3..5],
                &normalized[5..9]
            );
            if let Ok(regex) = Regex::new(&pattern) {
                self.ssn_pattern = Some((ssn.to_string(), regex));
            }
        }
    }

    fn compile_addresses(&mut self, addresses: &[AddressInfo]) {
        for addr in addresses {
            let street_regex = addr.street.as_ref().and_then(|s| {
                if s.len() >= 5 {
                    RegexBuilder::new(&regex::escape(s))
                        .case_insensitive(true)
                        .build()
                        .ok()
                } else {
                    None
                }
            });

            let city_regex = addr.city.as_ref().and_then(|c| {
                if c.len() >= 3 {
                    RegexBuilder::new(&format!(r"\b{}\b", regex::escape(c)))
                        .case_insensitive(true)
                        .build()
                        .ok()
                } else {
                    None
                }
            });

            let state_regex = addr.state.as_ref().and_then(|s| {
                if s.len() >= 2 {
                    RegexBuilder::new(&format!(r"\b{}\b", regex::escape(s)))
                        .case_insensitive(true)
                        .build()
                        .ok()
                } else {
                    None
                }
            });

            let zip_regex = addr.zip.as_ref().and_then(|z| {
                // nosemgrep: use-zeroize-for-secrets
                let normalized: String = z.chars().filter(|c| c.is_ascii_digit()).collect();
                if normalized.len() >= 5 {
                    Regex::new(&normalized).ok()
                } else {
                    None
                }
            });

            if street_regex.is_some() || city_regex.is_some() || state_regex.is_some() || zip_regex.is_some() {
                self.address_patterns.push(AddressPattern {
                    original: addr.clone(),
                    street_regex,
                    city_regex,
                    state_regex,
                    zip_regex,
                });
            }
        }
    }

    fn compile_names(&mut self, names: &[String]) {
        for name in names {
            if name.len() >= 3 {
                if let Ok(regex) = RegexBuilder::new(&format!(r"\b{}\b", regex::escape(name)))
                    .case_insensitive(true)
                    .build()
                {
                    self.name_patterns.push((name.clone(), regex));
                }
            }
        }
    }

    fn compile_dob(&mut self, dob: &str) {
        if let Some((month, day, year)) = parse_date(dob) {
            // Add word boundaries to prevent matching date as part of longer string
            let pattern = format!(
                r"\b{month:02}/{day:02}/{year}\b|\b{month:02}-{day:02}-{year}\b|\b{year}-{month:02}-{day:02}\b"
            );
            if let Ok(regex) = Regex::new(&pattern) {
                self.dob_pattern = Some((dob.to_string(), regex));
            }
        }
    }

    /// Check if there are any patterns to match against
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.email_patterns.is_empty()
            && self.phone_patterns.is_empty()
            && self.ssn_pattern.is_none()
            && self.address_patterns.is_empty()
            && self.name_patterns.is_empty()
            && self.dob_pattern.is_none()
    }

    /// Find all PII matches in the given text
    pub fn find_all(&self, text: &str) -> Vec<PiiMatch> {
        let mut matches = Vec::new();
        let lines: Vec<&str> = text.lines().collect();

        // First pass: collect potential address component matches
        let mut street_matches: Vec<(usize, &AddressInfo)> = Vec::new();
        let mut city_matches: Vec<(usize, &AddressInfo)> = Vec::new();
        let mut state_matches: Vec<(usize, &AddressInfo)> = Vec::new();
        let mut zip_matches: Vec<(usize, &AddressInfo)> = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            // nosemgrep: llm-prompt-injection-risk
            let line_number = line_num + 1;

            for (original, regex) in &self.email_patterns {
                if regex.is_match(line) {
                    matches.push(PiiMatch {
                        pii_type: PiiType::Email,
                        matched_value: original.clone(),
                        line_number,
                        line_content: truncate(line, 200),
                    });
                }
            }

            for (original, regexes) in &self.phone_patterns {
                if let Some(matched_regex) = regexes.iter().find(|r| r.is_match(line)) {
                    tracing::debug!(
                        "Phone match on line {}: user_phone='{}', matched_pattern='{:?}'",
                        line_number,
                        original,
                        matched_regex.as_str()
                    );
                    matches.push(PiiMatch {
                        pii_type: PiiType::Phone,
                        matched_value: original.clone(),
                        line_number,
                        line_content: truncate(line, 200),
                    });
                    break;
                }
            }

            if let Some((original, regex)) = &self.ssn_pattern {
                if regex.is_match(line) {
                    matches.push(PiiMatch {
                        pii_type: PiiType::Ssn,
                        matched_value: mask_ssn(original),
                        line_number,
                        line_content: truncate(line, 200),
                    });
                }
            }

            // Collect address component matches for proximity checking
            for addr in &self.address_patterns {
                if addr.street_regex.as_ref().is_some_and(|r| r.is_match(line)) {
                    street_matches.push((line_num, &addr.original));
                }
                if addr.city_regex.as_ref().is_some_and(|r| r.is_match(line)) {
                    city_matches.push((line_num, &addr.original));
                }
                if addr.state_regex.as_ref().is_some_and(|r| r.is_match(line)) {
                    state_matches.push((line_num, &addr.original));
                }
                if addr.zip_regex.as_ref().is_some_and(|r| r.is_match(line)) {
                    zip_matches.push((line_num, &addr.original));
                }
            }

            for (original, regex) in &self.name_patterns {
                if regex.is_match(line) {
                    matches.push(PiiMatch {
                        pii_type: PiiType::Name,
                        matched_value: original.clone(),
                        line_number,
                        line_content: truncate(line, 200),
                    });
                }
            }

            if let Some((original, regex)) = &self.dob_pattern {
                if regex.is_match(line) {
                    matches.push(PiiMatch {
                        pii_type: PiiType::DateOfBirth,
                        matched_value: original.clone(),
                        line_number,
                        line_content: truncate(line, 200),
                    });
                }
            }
        }

        // Second pass: find valid address matches
        // Require at least 2 components within proximity (street is worth 2 points if present)
        const PROXIMITY_LINES: usize = 5;

        // Group all component matches by address
        use std::collections::{HashMap, HashSet};
        let mut address_components: HashMap<*const AddressInfo, Vec<(usize, &'static str)>> = HashMap::new();

        for (line, addr) in &street_matches {
            address_components.entry(*addr as *const AddressInfo).or_default().push((*line, "street"));
        }
        for (line, addr) in &city_matches {
            address_components.entry(*addr as *const AddressInfo).or_default().push((*line, "city"));
        }
        for (line, addr) in &state_matches {
            address_components.entry(*addr as *const AddressInfo).or_default().push((*line, "state"));
        }
        for (line, addr) in &zip_matches {
            address_components.entry(*addr as *const AddressInfo).or_default().push((*line, "zip"));
        }

        let mut matched_addresses: HashSet<(*const AddressInfo, usize)> = HashSet::new();

        // Check each address for valid component combinations within proximity
        for (addr_ptr, components) in &address_components {
            // Check all pairs of components for proximity
            for i in 0..components.len() {
                for j in (i + 1)..components.len() {
                    let (line1, comp1) = components[i];
                    let (line2, comp2) = components[j];

                    let distance = if line1 > line2 { line1 - line2 } else { line2 - line1 };

                    if distance <= PROXIMITY_LINES {
                        // Valid match if:
                        // 1. Street + any other component
                        // 2. City + State (both specific)
                        // 3. Any component + zip (if street exists in profile)
                        let has_street = comp1 == "street" || comp2 == "street";
                        let has_city_state = (comp1 == "city" && comp2 == "state") || (comp1 == "state" && comp2 == "city");

                        if has_street || has_city_state {
                            // Use the most informative line (prefer street, then city, then zip)
                            let report_line = if comp1 == "street" {
                                line1
                            } else if comp2 == "street" {
                                line2
                            } else if comp1 == "city" {
                                line1
                            } else if comp2 == "city" {
                                line2
                            } else {
                                line1
                            };

                            // Avoid duplicates
                            if matched_addresses.insert((*addr_ptr, report_line)) {
                                let line_number = report_line + 1;
                                let line_content = if report_line < lines.len() {
                                    lines[report_line]
                                } else {
                                    ""
                                };

                                unsafe {
                                    matches.push(PiiMatch {
                                        pii_type: PiiType::Address,
                                        matched_value: format_address(&**addr_ptr),
                                        line_number,
                                        line_content: truncate(line_content, 200),
                                    });
                                }
                            }
                            break; // Found a valid match for this address
                        }
                    }
                }
                if matched_addresses.contains(&(*addr_ptr, components[i].0)) {
                    break; // Already matched this address
                }
            }
        }

        // Handle street-only addresses (when address has no other components in profile)
        for (street_line, street_addr) in &street_matches {
            let addr_pattern = self.address_patterns.iter()
                .find(|p| std::ptr::eq(&p.original, *street_addr));

            if let Some(pattern) = addr_pattern {
                // If address has only street (no city/state/zip patterns), match it
                if pattern.city_regex.is_none() && pattern.state_regex.is_none() && pattern.zip_regex.is_none() {
                    if matched_addresses.insert((*street_addr as *const AddressInfo, *street_line)) {
                        let line_number = street_line + 1;
                        let line_content = if *street_line < lines.len() {
                            lines[*street_line]
                        } else {
                            ""
                        };

                        matches.push(PiiMatch {
                            pii_type: PiiType::Address,
                            matched_value: format_address(street_addr),
                            line_number,
                            line_content: truncate(line_content, 200),
                        });
                    }
                }
            }
        }

        // Dedupe same type on same line
        matches.sort_by(|a, b| {
            a.line_number
                .cmp(&b.line_number)
                .then(a.pii_type.as_str().cmp(b.pii_type.as_str()))
        });
        matches.dedup_by(|a, b| a.line_number == b.line_number && a.pii_type == b.pii_type);
        matches
    }
}

fn parse_date(date: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = date.split(['/', '-', '.']).collect();
    if parts.len() == 3 {
        let nums: Vec<u32> = parts.iter().filter_map(|p| p.parse().ok()).collect();
        if nums.len() == 3 {
            if nums[0] > 31 {
                return Some((nums[1], nums[2], nums[0]));
            }
            if nums[2] > 31 {
                return Some((nums[0], nums[1], nums[2]));
            }
        }
    }
    None
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        // Find a valid char boundary at or before max
        let mut boundary = max;
        while boundary > 0 && !s.is_char_boundary(boundary) {
            boundary -= 1;
        }
        format!("{}...", &s[..boundary])
    }
}

fn mask_ssn(ssn: &str) -> String {
    // nosemgrep: use-zeroize-for-secrets
    let digits: String = ssn.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() >= 4 {
        format!("***-**-{}", &digits[digits.len() - 4..])
    } else {
        "***-**-****".to_string()
    }
}

fn format_address(addr: &AddressInfo) -> String {
    [
        addr.street.as_deref(),
        addr.city.as_deref(),
        addr.state.as_deref(),
        addr.zip.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_matching() {
        let pii = UserPii {
            emails: vec!["test@example.com".into()],
            ..Default::default()
        };
        let patterns = PiiPatterns::new(&pii, &ScanConfig::default());
        let matches = patterns.find_all("Contact: test@example.com");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pii_type, PiiType::Email);
    }

    #[test]
    fn test_phone_matching() {
        let pii = UserPii {
            phones: vec!["555-123-4567".into()],
            ..Default::default()
        };
        let patterns = PiiPatterns::new(&pii, &ScanConfig::default());
        let matches = patterns.find_all("Call (555) 123-4567");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pii_type, PiiType::Phone);
    }

    #[test]
    fn test_ssn_masking() {
        assert_eq!(mask_ssn("123-45-6789"), "***-**-6789");
    }
}
