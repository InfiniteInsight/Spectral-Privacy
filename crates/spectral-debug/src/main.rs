//! scan-debug: CLI tool for diagnosing broker scan issues.
//!
//! Usage:
//!   scan-debug --broker <id> --first <name> --last <name> [--state <st>] [--city <city>]
//!              [--dump-html <path>]
//!
//! What it does:
//!   1. Loads the broker definition (TOML) from BROKER_DEFS_PATH (or ../broker-definitions)
//!   2. Builds the search URL from the profile fields
//!   3. Fetches the page with a realistic browser User-Agent via reqwest
//!   4. Optionally dumps the raw HTML to a file
//!   5. Applies each CSS selector from the broker definition and reports matches

use scraper::{Html, Selector};
use spectral_broker::{BrokerLoader, BrokerRegistry, SearchMethod};
use std::collections::HashMap;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .init();

    let args: Vec<String> = std::env::args().collect();
    let opts = parse_args(&args);

    let broker_id = opts
        .get("broker")
        .cloned()
        .unwrap_or_else(|| usage_exit("--broker is required"));

    let first = opts
        .get("first")
        .cloned()
        .unwrap_or_else(|| usage_exit("--first is required"));

    let last = opts
        .get("last")
        .cloned()
        .unwrap_or_else(|| usage_exit("--last is required"));

    let state = opts.get("state").cloned();
    let city = opts.get("city").cloned();
    let dump_path = opts.get("dump-html").map(PathBuf::from);

    // Locate broker definitions directory
    let defs_path = std::env::var("BROKER_DEFS_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            // Try relative to current dir, then relative to this binary's location
            let candidates = [
                PathBuf::from("broker-definitions"),
                PathBuf::from("../broker-definitions"),
                PathBuf::from("../../broker-definitions"),
                PathBuf::from("/home/evan/projects/spectral/broker-definitions"),
            ];
            candidates
                .into_iter()
                .find(|p| p.exists())
                .unwrap_or_else(|| {
                    eprintln!(
                        "ERROR: broker-definitions directory not found.\n\
                         Set BROKER_DEFS_PATH env var or run from the spectral repo root."
                    );
                    std::process::exit(1);
                })
        });

    let loader = BrokerLoader::new(defs_path.clone()).unwrap_or_else(|e| {
        eprintln!("ERROR: Failed to create BrokerLoader: {e}");
        std::process::exit(1);
    });
    let registry = BrokerRegistry::load_from(&loader).unwrap_or_else(|e| {
        eprintln!("ERROR: Failed to load broker definitions: {e}");
        std::process::exit(1);
    });

    let bid = spectral_core::BrokerId::new(&broker_id)
        .unwrap_or_else(|_| usage_exit(&format!("Invalid broker ID: {broker_id}")));

    let broker = registry.get(&bid).unwrap_or_else(|_| {
        eprintln!(
            "ERROR: Broker '{}' not found in {}",
            broker_id,
            defs_path.display()
        );
        std::process::exit(1);
    });

    println!("=== scan-debug: {} ({}) ===", broker.name(), broker_id);
    println!();

    // Build search URL
    let url = build_url(
        &broker.search,
        &first,
        &last,
        state.as_deref(),
        city.as_deref(),
    );
    println!("Search URL: {url}");
    println!();

    // Fetch the page
    println!("Fetching page...");
    let html = match fetch_page(&url).await {
        Ok(h) => h,
        Err(e) => {
            eprintln!("ERROR fetching {url}: {e}");
            std::process::exit(1);
        }
    };

    println!(
        "Fetched {} bytes ({} chars)",
        html.len(),
        html.chars().count()
    );

    // Save HTML if requested
    if let Some(ref path) = dump_path {
        std::fs::write(path, &html)
            .unwrap_or_else(|e| eprintln!("WARN: could not write HTML: {e}"));
        println!("HTML dumped to: {}", path.display());
    }
    println!();

    // Analyze page content
    analyze_page_content(&html);
    println!();

    // Test selectors
    match broker.search.result_selectors() {
        None => {
            println!("No result_selectors defined for this broker.");
            println!("This broker cannot produce findings without selectors.");
        }
        Some(selectors) => {
            println!("=== Selector Test Results ===");
            let document = Html::parse_document(&html);

            // Check bot protection
            check_bot_protection(&html);

            // Results container
            test_selector(&document, "results_container", &selectors.results_container);

            // Result items
            let item_count = test_selector(&document, "result_item", &selectors.result_item);

            if item_count == 0 {
                println!(
                    "\n⚠ result_item selector matched 0 elements — no findings can be extracted."
                );
            } else {
                println!("\n✓ Found {item_count} potential result item(s).");
            }

            // Test remaining selectors
            test_optional_selector(&document, "listing_url", Some(&selectors.listing_url));
            test_optional_selector(&document, "name", selectors.name.as_deref());
            test_optional_selector(&document, "age", selectors.age.as_deref());
            test_optional_selector(&document, "location", selectors.location.as_deref());
            test_optional_selector(&document, "phones", selectors.phones.as_deref());
            test_optional_selector(&document, "emails", selectors.emails.as_deref());
            test_optional_selector(&document, "relatives", selectors.relatives.as_deref());
            test_optional_selector(
                &document,
                "no_results_indicator",
                selectors.no_results_indicator.as_deref(),
            );
            test_optional_selector(
                &document,
                "captcha_required",
                selectors.captcha_required.as_deref(),
            );

            // If result_item matched, sample first few
            if item_count > 0 {
                println!("\n--- First result item content (truncated) ---");
                if let Ok(sel) = Selector::parse(&selectors.result_item) {
                    for (i, el) in document.select(&sel).take(2).enumerate() {
                        let text: String = el.text().collect::<Vec<_>>().join(" ");
                        let trimmed = text.split_whitespace().collect::<Vec<_>>().join(" ");
                        let preview: String = trimmed.chars().take(300).collect();
                        println!("[item {i}] {preview}");
                    }
                }
            }
        }
    }

    println!();
    println!("Diagnosis: see selector matches above. If result_item = 0 and page has");
    println!("bot protection indicators, CSS selectors are likely correct but blocked.");
    println!("If result_item = 0 and no bot protection, selectors need updating.");
}

fn build_url(
    method: &SearchMethod,
    first: &str,
    last: &str,
    state: Option<&str>,
    city: Option<&str>,
) -> String {
    match method {
        SearchMethod::UrlTemplate { template, .. } => {
            let mut url = template.clone();
            url = url.replace("{first}", &slug(first));
            url = url.replace("{last}", &slug(last));
            url = url.replace("{state}", state.unwrap_or(""));
            url = url.replace("{city}", &slug(city.unwrap_or("")));
            url = url.replace("{first_name}", &slug(first));
            url = url.replace("{last_name}", &slug(last));
            url
        }
        SearchMethod::WebForm { url, .. } => url.clone(),
        SearchMethod::Manual { url, .. } => url.clone(),
    }
}

fn slug(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

async fn fetch_page(url: &str) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
             AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/120.0.0.0 Safari/537.36",
        )
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    client
        .get(url)
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .header("Accept-Language", "en-US,en;q=0.9")
        .header("Accept-Encoding", "gzip, deflate")
        .header("DNT", "1")
        .header("Upgrade-Insecure-Requests", "1")
        .send()
        .await?
        .text()
        .await
}

fn analyze_page_content(html: &str) {
    println!("=== Page Content Analysis ===");
    let lower = html.to_lowercase();

    let indicators = [
        ("Cloudflare challenge", "just a moment"),
        ("Cloudflare challenge", "cf-browser-verification"),
        ("Cloudflare challenge", "page-manage-challenge"),
        ("Cloudflare challenge", "cf-challenge"),
        ("CAPTCHA", "recaptcha"),
        ("CAPTCHA", "hcaptcha"),
        ("CAPTCHA", "captcha"),
        ("Bot detection", "bot detection"),
        ("Security check", "security check"),
        ("Security challenge", "manage-challenge"),
        ("Access denied", "access denied"),
        ("403 forbidden", "403 forbidden"),
        ("Rate limited", "rate limit"),
    ];

    let mut any_block = false;
    for (label, keyword) in indicators {
        if lower.contains(keyword) {
            println!("  🚫 {label}: found '{keyword}'");
            any_block = true;
        }
    }

    if !any_block {
        println!("  ✓ No obvious bot protection indicators found.");
    }
}

fn check_bot_protection(html: &str) {
    let lower = html.to_lowercase();
    let blocked = lower.contains("just a moment")
        || lower.contains("cf-browser-verification")
        || lower.contains("manage-challenge")
        || lower.contains("access denied")
        || lower.contains("403 forbidden");

    if blocked {
        println!(
            "\n  ⚠ Bot protection detected: selector misses are likely due to blocking, not wrong selectors."
        );
    }
}

fn test_selector(document: &Html, name: &str, selector_str: &str) -> usize {
    match Selector::parse(selector_str) {
        Err(e) => {
            println!("  [{name}] INVALID selector '{selector_str}': {e}");
            0
        }
        Ok(selector) => {
            let count = document.select(&selector).count();
            let status = if count > 0 { "✓" } else { "✗" };
            println!("  {status} [{name}] '{selector_str}' → {count} match(es)");
            count
        }
    }
}

fn test_optional_selector(document: &Html, name: &str, selector_str: Option<&str>) -> usize {
    match selector_str {
        None => {
            println!("  - [{name}] (not defined)");
            0
        }
        Some(s) => test_selector(document, name, s),
    }
}

fn parse_args(args: &[String]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if let Some(key) = arg.strip_prefix("--") {
            if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                map.insert(key.to_string(), args[i + 1].clone());
                i += 2;
            } else {
                map.insert(key.to_string(), String::new());
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    map
}

fn usage_exit(msg: &str) -> ! {
    eprintln!("ERROR: {msg}");
    eprintln!();
    eprintln!(
        "Usage: scan-debug --broker <id> --first <name> --last <name> \
         [--state <st>] [--city <city>] [--dump-html <path>]"
    );
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  scan-debug --broker radaris --first John --last Smith --state CA");
    eprintln!("  scan-debug --broker whitepages --first Jane --last Doe --state NY --city Albany");
    eprintln!("  scan-debug --broker fastpeoplesearch --first Sam --last Jones --state TX --dump-html /tmp/fps.html");
    std::process::exit(1);
}
