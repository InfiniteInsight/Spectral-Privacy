use spectral_broker::BrokerDefinition;
use std::fs;
use std::path::Path;

fn main() {
    let files = vec![
        "broker-definitions/people-search/nuwber.toml",
        "broker-definitions/people-search/neighborwho.toml",
        "broker-definitions/phone-lookup/callersmart.toml",
        "broker-definitions/phone-lookup/truecaller.toml",
    ];

    for file in files {
        let path = Path::new(file);
        println!("\n=== Checking {} ===", file);

        match fs::read_to_string(path) {
            Ok(content) => match toml::from_str::<BrokerDefinition>(&content) {
                Ok(_) => println!("✓ Parsed successfully"),
                Err(e) => println!("✗ Parse error: {}", e),
            },
            Err(e) => println!("✗ Failed to read file: {}", e),
        }
    }
}
