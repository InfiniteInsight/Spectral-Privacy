//! Example: Load and display broker definitions from the broker-definitions directory.

use spectral_broker::{BrokerLoader, BrokerRegistry};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Try to load from the default directory (broker-definitions/)
    println!("Loading broker definitions from broker-definitions/...\n");

    let loader = match BrokerLoader::with_default_dir() {
        Ok(loader) => loader,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("\nMake sure you're running this from the workspace root!");
            return Err(Box::new(e));
        }
    };

    // Load all brokers
    let definitions = loader.load_all()?;

    println!(
        "✓ Successfully loaded {} broker definitions:\n",
        definitions.len()
    );

    // Display each broker
    for def in &definitions {
        println!("  • {} ({})", def.name(), def.id());
        println!("    Category: {:?}", def.category());
        println!("    Difficulty: {:?}", def.broker.difficulty);
        println!("    Removal time: {} days", def.broker.typical_removal_days);

        // Show search method
        match &def.search {
            spectral_broker::SearchMethod::UrlTemplate {
                template,
                requires_fields,
                result_selectors,
            } => {
                println!("    Search: URL template");
                println!("      Template: {}", template);
                println!("      Requires: {:?}", requires_fields);
                if result_selectors.is_some() {
                    println!("      ✓ Has result selectors");
                } else {
                    println!("      ⚠ Missing result selectors");
                }
            }
            spectral_broker::SearchMethod::WebForm { url, .. } => {
                println!("    Search: Web form at {}", url);
            }
            spectral_broker::SearchMethod::Manual { url, .. } => {
                println!("    Search: Manual at {}", url);
            }
        }

        println!();
    }

    // Create a registry
    let _registry = BrokerRegistry::new();
    println!("\n✓ Created broker registry (empty)");
    println!("  Use BrokerRegistry::load_from(&loader) to populate it.\n");

    Ok(())
}
