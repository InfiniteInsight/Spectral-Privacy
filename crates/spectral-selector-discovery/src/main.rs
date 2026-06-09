//! spectral-discover: CLI tool for automated CSS selector discovery on broker sites.
//!
//! Usage:
//!   spectral-discover --broker test-broker --url https://test-broker.com/search --output broker.toml
//!
//! This tool navigates to one or more URLs, analyzes the DOM for form fields and
//! result containers, scores discovered selectors, and outputs a TOML fragment
//! that can be merged into a broker definition.

use anyhow::Result;
use clap::Parser;
use spectral_selector_discovery::{
    analyze_dom, consolidate_results, merge_with_existing, run_discovery, DiscoveryConfig,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing_subscriber;

/// CLI arguments for selector discovery.
#[derive(Parser, Debug)]
#[command(
    name = "spectral-discover",
    about = "Discover and score CSS selectors for data broker search forms",
    version
)]
struct Cli {
    /// Broker ID to discover selectors for.
    #[arg(short, long)]
    broker: String,

    /// URLs to analyze (can be specified multiple times).
    #[arg(short, long, required = true)]
    url: Vec<String>,

    /// Output file path for TOML results (prints to stdout if omitted).
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Minimum confidence threshold (0.0–1.0, default: 0.5).
    #[arg(short, long, default_value = "0.5")]
    min_confidence: f64,

    /// Maximum selectors per purpose (default: 3).
    #[arg(long, default_value = "3")]
    max_per_purpose: u32,

    /// Path to existing TOML broker definition (for merging).
    #[arg(long)]
    existing: Option<PathBuf>,

    /// HTML input file (skip browser, use local HTML for testing).
    #[arg(long)]
    html_input: Option<PathBuf>,

    /// Verbose output.
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(if cli.verbose {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let broker_id = spectral_broker::BrokerId::new(&cli.broker)?;

    // If HTML input is provided, skip browser and use local file
    let results = if let Some(html_path) = &cli.html_input {
        let html = fs::read_to_string(html_path)?;
        // Analyze directly without browser
        let config = DiscoveryConfig {
            urls: cli.url.clone(),
            min_confidence: cli.min_confidence,
            max_selectors_per_purpose: cli.max_per_purpose,
        };
        let mut results = Vec::new();
        for url in &cli.url {
            use spectral_selector_discovery::engine::analyze_page;
            let result = analyze_page(&broker_id, url, &html, &config)?;
            results.push(result);
        }
        results
    } else {
        // Use browser-based discovery (placeholder — integrates with spectral-browser)
        let config = DiscoveryConfig {
            urls: cli.url.clone(),
            min_confidence: cli.min_confidence,
            max_selectors_per_purpose: cli.max_per_purpose,
        };
        run_discovery(&broker_id, &cli.url, &config).await?
    };

    // Consolidate results
    let consolidated = consolidate_results(&results);

    // Print summary
    println!("=== Selector Discovery Results ===\n");
    println!("Broker: {}", broker_id);
    println!("URLs analyzed: {}", cli.url.len());
    println!("Discovered selectors: {}\n", consolidated.len());

    for (purpose, selector, score) in &consolidated {
        println!(
            "  {:<30} {} (score: {:.2})",
            purpose.label(),
            selector,
            score
        );
    }

    // If merging with existing TOML
    if let Some(existing_path) = &cli.existing {
        if let Ok(content) = fs::read_to_string(existing_path) {
            if let Ok(existing) = toml::from_str::<HashMap<String, String>>(&content) {
                println!("\n=== Existing selectors: {} ===", existing.len());
                for (key, selector) in &existing {
                    println!("  {:<30} {}", key, selector);
                }

                // Merge
                let discovered_selectors: Vec<_> = results
                    .iter()
                    .flat_map(|r| r.candidate_selectors.clone())
                    .collect();

                let scored: Vec<_> = results
                    .iter()
                    .flat_map(|r| r.scored_selectors.clone())
                    .collect();

                let (merged, candidates, scores) = merge_with_existing(
                    &broker_id,
                    &existing,
                    &discovered_selectors,
                    &scored,
                    "search_form",
                )?;

                println!("\n=== Merged selectors: {} ===", merged.len());
                for (key, selector) in &merged {
                    println!("  {:<30} {}", key, selector);
                }

                if !candidates.is_empty() {
                    println!("\n=== Candidates for review: {} ===", candidates.len());
                    for c in &candidates {
                        println!("  {:<30} (confidence: {:.2})", c.selector, c.confidence);
                    }
                }
            }
        }
    }

    // Output to file or stdout
    let output_text = serde_json::to_string_pretty(&consolidated)?;

    if let Some(output_path) = &cli.output {
        fs::write(output_path, &output_text)?;
        println!("\nResults written to: {}", output_path.display());
    } else {
        println!("\n--- JSON Output ---");
        println!("{}", output_text);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broker_id_creation() {
        let id = spectral_broker::BrokerId::new("test-broker").unwrap();
        assert_eq!(id.as_str(), "test-broker");
    }
}
