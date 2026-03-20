//! PII pattern matching engine

use crate::types::{AddressInfo, PiiMatch, PiiType, ScanConfig, UserPii};
use regex::{Regex, RegexBuilder};

/// Pattern matcher for finding user-specific PII in text
pub struct Matcher {
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

impl Matcher {
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
            // Match exact email - escape special chars and match case-insensitively
            let escaped = regex::escape(&email.to_lowercase());
            if let Ok(regex) = RegexBuilder::new(&escaped).case_insensitive(true).build() {
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
                    format!(r"\({area}\)\s*{prefix}-{line}\b"), // Added end boundary
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
            let street_regex = addr.street.as_deref().and_then(compile_street_regex);
            let city_regex = addr.city.as_deref().and_then(|c| compile_field_regex(c, 3));
            let state_regex = addr
                .state
                .as_deref()
                .and_then(|s| compile_field_regex(s, 2));
            let zip_regex = addr.zip.as_ref().and_then(|z| {
                // nosemgrep: use-zeroize-for-secrets
                let normalized: String = z.chars().filter(|c| c.is_ascii_digit()).collect();
                if normalized.len() >= 5 {
                    // Add word boundaries to prevent matching zip as part of longer number
                    Regex::new(&format!(r"\b{}\b", normalized)).ok()
                } else {
                    None
                }
            });

            if street_regex.is_some()
                || city_regex.is_some()
                || state_regex.is_some()
                || zip_regex.is_some()
            {
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
    /// Scan for simple PII patterns (email, phone, SSN, name, DOB)
    fn scan_simple_patterns(&self, line: &str, line_number: usize, matches: &mut Vec<PiiMatch>) {
        // Email matches
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

        // Phone matches
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

        // SSN matches
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

        // Name matches
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

        // DOB matches
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

    /// Collect address component matches for proximity checking
    fn collect_address_components<'a>(
        &'a self,
        line: &str,
        line_num: usize,
        street_matches: &mut Vec<(usize, &'a AddressInfo)>,
        city_matches: &mut Vec<(usize, &'a AddressInfo)>,
        state_matches: &mut Vec<(usize, &'a AddressInfo)>,
        zip_matches: &mut Vec<(usize, &'a AddressInfo)>,
    ) {
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
    }

    pub fn find_all(&self, text: &str) -> Vec<PiiMatch> {
        let mut matches = Vec::new();
        let lines: Vec<&str> = text.lines().collect();

        // First pass: collect simple pattern matches and address component positions
        let mut street_matches: Vec<(usize, &AddressInfo)> = Vec::new();
        let mut city_matches: Vec<(usize, &AddressInfo)> = Vec::new();
        let mut state_matches: Vec<(usize, &AddressInfo)> = Vec::new();
        let mut zip_matches: Vec<(usize, &AddressInfo)> = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            // nosemgrep: llm-prompt-injection-risk
            let line_number = line_num + 1;
            self.scan_simple_patterns(line, line_number, &mut matches);
            self.collect_address_components(
                line,
                line_num,
                &mut street_matches,
                &mut city_matches,
                &mut state_matches,
                &mut zip_matches,
            );
        }

        // Second pass: find valid address matches using proximity checking
        use std::collections::HashSet;
        let address_components = build_address_component_map(
            &street_matches,
            &city_matches,
            &state_matches,
            &zip_matches,
        );

        let mut matched_addresses: HashSet<(*const AddressInfo, usize)> = HashSet::new();

        check_address_proximity(
            &address_components,
            &lines,
            &mut matched_addresses,
            &mut matches,
        );
        self.find_street_only_addresses(
            &street_matches,
            &lines,
            &mut matched_addresses,
            &mut matches,
        );

        // Dedupe same type on same line
        matches.sort_by(|a, b| {
            a.line_number
                .cmp(&b.line_number)
                .then(a.pii_type.as_str().cmp(b.pii_type.as_str()))
        });
        matches.dedup_by(|a, b| a.line_number == b.line_number && a.pii_type == b.pii_type);
        matches
    }

    fn find_street_only_addresses(
        &self,
        street_matches: &[(usize, &AddressInfo)],
        lines: &[&str],
        matched_addresses: &mut std::collections::HashSet<(*const AddressInfo, usize)>,
        matches: &mut Vec<PiiMatch>,
    ) {
        for (street_line, street_addr) in street_matches {
            let addr_pattern = self
                .address_patterns
                .iter()
                .find(|p| std::ptr::eq(&p.original, *street_addr));

            if let Some(pattern) = addr_pattern {
                // If address has only street (no city/state/zip patterns), match it directly
                if pattern.city_regex.is_none()
                    && pattern.state_regex.is_none()
                    && pattern.zip_regex.is_none()
                    && matched_addresses.insert((*street_addr as *const AddressInfo, *street_line))
                {
                    let line_content = lines.get(*street_line).copied().unwrap_or("");
                    matches.push(PiiMatch {
                        pii_type: PiiType::Address,
                        matched_value: format_address(street_addr),
                        line_number: street_line + 1, // nosemgrep: llm-prompt-injection-risk
                        line_content: truncate(line_content, 200),
                    });
                }
            }
        }
    }
}

/// Build a regex for a street address string (word boundary at start only).
fn compile_street_regex(s: &str) -> Option<Regex> {
    if s.len() >= 5 {
        // Add word boundary at start to prevent matching in middle of text.
        // End boundary not added as street addresses often have suffixes.
        RegexBuilder::new(&format!(r"\b{}", regex::escape(s)))
            .case_insensitive(true)
            .build()
            .ok()
    } else {
        None
    }
}

/// Build a word-boundary regex for a generic address field with a minimum length requirement.
fn compile_field_regex(s: &str, min_len: usize) -> Option<Regex> {
    if s.len() >= min_len {
        RegexBuilder::new(&format!(r"\b{}\b", regex::escape(s)))
            .case_insensitive(true)
            .build()
            .ok()
    } else {
        None
    }
}

/// Group collected address component matches by address pointer into a single map.
fn build_address_component_map<'a>(
    street_matches: &[(usize, &'a AddressInfo)],
    city_matches: &[(usize, &'a AddressInfo)],
    state_matches: &[(usize, &'a AddressInfo)],
    zip_matches: &[(usize, &'a AddressInfo)],
) -> std::collections::HashMap<*const AddressInfo, Vec<(usize, &'static str)>> {
    use std::collections::HashMap;
    let mut map: HashMap<*const AddressInfo, Vec<(usize, &'static str)>> = HashMap::new();

    for (line, addr) in street_matches {
        map.entry(*addr as *const AddressInfo)
            .or_default()
            .push((*line, "street"));
    }
    for (line, addr) in city_matches {
        map.entry(*addr as *const AddressInfo)
            .or_default()
            .push((*line, "city"));
    }
    for (line, addr) in state_matches {
        map.entry(*addr as *const AddressInfo)
            .or_default()
            .push((*line, "state"));
    }
    for (line, addr) in zip_matches {
        map.entry(*addr as *const AddressInfo)
            .or_default()
            .push((*line, "zip"));
    }
    map
}

/// Determine the best line to report for a matched component pair.
/// Prefers street > city > first component.
fn pick_report_line(line1: usize, comp1: &str, line2: usize, comp2: &str) -> usize {
    if comp1 == "street" {
        line1
    } else if comp2 == "street" {
        line2
    } else if comp1 == "city" {
        line1
    } else if comp2 == "city" {
        line2
    } else {
        line1
    }
}

/// Returns true if the two address component types form a valid matchable pair.
///
/// Valid combinations:
/// 1. Street + any other component
/// 2. City + State
/// 3. City + Zip
///    State + Zip alone is NOT matched (too generic)
fn is_valid_address_pair(comp1: &str, comp2: &str) -> bool {
    let has_street = comp1 == "street" || comp2 == "street";
    let has_city_state =
        (comp1 == "city" && comp2 == "state") || (comp1 == "state" && comp2 == "city");
    let has_city_zip = (comp1 == "city" && comp2 == "zip") || (comp1 == "zip" && comp2 == "city");
    has_street || has_city_state || has_city_zip
}

/// Find the first valid component pair within proximity, returning (line1, comp1, line2, comp2).
fn find_first_valid_pair<'a>(
    components: &[(usize, &'a str)],
    proximity: usize,
) -> Option<(usize, &'a str, usize, &'a str)> {
    for i in 0..components.len() {
        for j in (i + 1)..components.len() {
            let (line1, comp1) = components[i];
            let (line2, comp2) = components[j];
            if line1.abs_diff(line2) <= proximity && is_valid_address_pair(comp1, comp2) {
                return Some((line1, comp1, line2, comp2));
            }
        }
    }
    None
}

/// Check all addresses in the component map for valid pairs within proximity and
/// push matches into `matches`. Updates `matched_addresses` to avoid duplicates.
fn check_address_proximity(
    address_components: &std::collections::HashMap<*const AddressInfo, Vec<(usize, &'static str)>>,
    lines: &[&str],
    matched_addresses: &mut std::collections::HashSet<(*const AddressInfo, usize)>,
    matches: &mut Vec<PiiMatch>,
) {
    const PROXIMITY_LINES: usize = 5;

    for (addr_ptr, components) in address_components {
        if let Some((line1, comp1, line2, comp2)) =
            find_first_valid_pair(components, PROXIMITY_LINES)
        {
            let report_line = pick_report_line(line1, comp1, line2, comp2);
            if matched_addresses.insert((*addr_ptr, report_line)) {
                let line_content = lines.get(report_line).copied().unwrap_or("");
                // SAFETY: addr_ptr is a raw pointer derived from a reference to an AddressInfo
                // in self.addresses, which lives for the duration of find_all. The pointer is
                // not null, not dangling, and not aliased mutably — the borrow ends before this
                // unsafe block and we only read through the pointer here.
                #[allow(unsafe_code)]
                let addr_value = unsafe { format_address(&**addr_ptr) }; // nosemgrep: no-unsafe-blocks
                matches.push(PiiMatch {
                    pii_type: PiiType::Address,
                    matched_value: addr_value,
                    line_number: report_line + 1, // nosemgrep: llm-prompt-injection-risk
                    line_content: truncate(line_content, 200),
                });
            }
        }
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
        let patterns = Matcher::new(&pii, &ScanConfig::default());
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
        let patterns = Matcher::new(&pii, &ScanConfig::default());
        let matches = patterns.find_all("Call (555) 123-4567");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pii_type, PiiType::Phone);
    }

    #[test]
    fn test_ssn_masking() {
        assert_eq!(mask_ssn("123-45-6789"), "***-**-6789");
    }
}
