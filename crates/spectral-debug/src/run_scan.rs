//! run-scan: Integration test binary that unlocks Sam's vault and runs a real scan.
//!
//! Usage:
//!   run-scan [--vault-id <id>] [--vault-password <pass>] [--brokers <b1,b2,...>] [--max-brokers <n>]
//!
//! Defaults:
//!   --vault-id       sam
//!   --vault-password sam1
//!   --brokers        (all valid people-search brokers)
//!   --max-brokers    5  (limit for quick test run)

use spectral_broker::{BrokerLoader, BrokerRegistry};
use spectral_browser::BrowserEngine;
use spectral_core::BrokerId;
use spectral_db::{scan_jobs, Database};
use spectral_scanner::{BrokerFilter, ScanOrchestrator};
use spectral_vault::Vault;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use zeroize::Zeroizing;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args: Vec<String> = std::env::args().collect();
    let opts = parse_args(&args);

    let vault_id = opts.get("vault-id").map(|s| s.as_str()).unwrap_or("sam");
    let vault_password = opts
        .get("vault-password")
        .map(|s| s.as_str())
        .unwrap_or("sam1");
    let specific_brokers: Option<Vec<String>> = opts.get("brokers").map(|s| {
        s.split(',')
            .map(|b| b.trim().to_string())
            .filter(|b| !b.is_empty())
            .collect()
    });
    let max_brokers: usize = opts
        .get("max-brokers")
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);

    println!("=== Spectral Scan Integration Test ===");
    println!("Vault ID:    {vault_id}");
    println!("Max brokers: {max_brokers}");
    println!();

    // Locate vault
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let vault_path = PathBuf::from(&home)
        .join(".local/share/spectral/vaults")
        .join(vault_id)
        .join("vault.db");

    if !vault_path.exists() {
        eprintln!("ERROR: vault not found at {}", vault_path.display());
        std::process::exit(1);
    }

    println!("Step 1: Unlocking vault at {}", vault_path.display());
    let password = Zeroizing::new(vault_password.to_string());
    let vault = match Vault::unlock(&password, &vault_path).await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("ERROR: Failed to unlock vault: {e}");
            std::process::exit(1);
        }
    };
    println!("  ✓ Vault unlocked");

    // Get encryption key
    let vault_key = match vault.encryption_key() {
        Ok(k) => *k,
        Err(e) => {
            eprintln!("ERROR: Failed to get vault key: {e}");
            std::process::exit(1);
        }
    };

    // List profiles
    println!();
    println!("Step 2: Loading profiles");
    let profile_ids = match vault.list_profiles().await {
        Ok(ids) => ids,
        Err(e) => {
            eprintln!("ERROR: Failed to list profiles: {e}");
            std::process::exit(1);
        }
    };

    if profile_ids.is_empty() {
        eprintln!("ERROR: No profiles found in vault '{vault_id}'");
        std::process::exit(1);
    }

    println!("  Found {} profile(s)", profile_ids.len());
    let profile_id = &profile_ids[0];
    let profile = match vault.load_profile(profile_id).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ERROR: Failed to load profile {profile_id}: {e}");
            std::process::exit(1);
        }
    };

    // Print decrypted profile fields for verification
    println!("  Profile ID: {}", profile.id);
    if let Some(ref f) = profile.first_name {
        match f.decrypt(&vault_key) {
            Ok(v) => println!("  First name: {v}"),
            Err(e) => println!("  First name: [decrypt error: {e}]"),
        }
    }
    if let Some(ref f) = profile.last_name {
        match f.decrypt(&vault_key) {
            Ok(v) => println!("  Last name:  {v}"),
            Err(e) => println!("  Last name:  [decrypt error: {e}]"),
        }
    }
    if let Some(ref f) = profile.city {
        match f.decrypt(&vault_key) {
            Ok(v) => print!("  City:       {v}"),
            Err(_) => print!("  City:       [encrypted]"),
        }
    }
    if let Some(ref f) = profile.state {
        match f.decrypt(&vault_key) {
            Ok(v) => println!(", {v}"),
            Err(_) => println!(), // newline if state fails
        }
    } else {
        println!();
    }
    println!("  ✓ Profile loaded");

    // Load broker definitions
    println!();
    println!("Step 3: Loading broker definitions");
    let defs_path = PathBuf::from("/home/evan/projects/spectral/broker-definitions");
    let loader = match BrokerLoader::new(defs_path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("ERROR: Failed to create BrokerLoader: {e}");
            std::process::exit(1);
        }
    };
    let registry = match BrokerRegistry::load_from(&loader) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ERROR: Failed to load broker registry: {e}");
            std::process::exit(1);
        }
    };

    // Select brokers to scan
    let broker_filter = if let Some(ref ids) = specific_brokers {
        BrokerFilter::Specific(ids.clone())
    } else {
        BrokerFilter::Category("people-search".to_string())
    };

    let all_brokers: Vec<_> = registry
        .get_all()
        .into_iter()
        .filter(|b| broker_filter.matches(b))
        .take(max_brokers)
        .collect();

    println!("  Total brokers in registry: {}", registry.get_all().len());
    println!("  Brokers selected for scan: {}", all_brokers.len());
    for b in &all_brokers {
        println!("    - {} ({})", b.name(), b.id());
    }

    if all_brokers.is_empty() {
        println!("No brokers to scan. Exiting.");
        std::process::exit(0);
    }

    // Open a fresh database connection (separate from vault's internal connection)
    println!();
    println!("Step 4: Opening database connection");
    let db = match Database::new(&vault_path, vault_key.to_vec()).await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("ERROR: Failed to open database: {e}");
            std::process::exit(1);
        }
    };
    println!("  ✓ Database connected");

    // Initialize browser engine
    println!();
    println!("Step 5: Initializing browser engine");
    let browser = match BrowserEngine::new().await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("ERROR: Failed to initialize browser engine: {e}");
            eprintln!("  Note: Make sure Chrome/Chromium is installed and accessible.");
            std::process::exit(1);
        }
    };
    println!("  ✓ Browser engine ready");

    // Create scan orchestrator
    println!();
    println!("Step 6: Running scan");
    let broker_ids: Vec<BrokerId> = all_brokers.iter().map(|b| b.id().clone()).collect();
    println!("  Scanning {} brokers...", broker_ids.len());
    println!();

    let db = Arc::new(db);
    let orchestrator = ScanOrchestrator::new(Arc::new(registry), Arc::new(browser), db.clone())
        .with_max_concurrent_scans(2);

    // Pre-create scan_job record (required for FK constraint in broker_scans)
    let _scan_job = match scan_jobs::create_scan_job(
        db.pool(),
        profile_id.as_str().to_string(),
        all_brokers.len() as u32,
    )
    .await
    {
        Ok(j) => {
            println!("  Created scan job record: {}", j.id);
            // Override our job ID to match what's in DB
            // (we reuse scan_job_id as the key we pass below)
            j
        }
        Err(e) => {
            eprintln!("ERROR: Failed to create scan job record: {e}");
            std::process::exit(1);
        }
    };
    // Use the DB-assigned job ID
    let scan_job_id = _scan_job.id.clone();

    let start = std::time::Instant::now();
    let results = match orchestrator
        .execute_scan_job(
            scan_job_id.clone(),
            broker_ids,
            profile_id.as_str().to_string(),
            vault_key,
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ERROR: Scan job failed: {e}");
            std::process::exit(1);
        }
    };
    let elapsed = start.elapsed();

    // Report results
    println!("=== Scan Results ===");
    println!("Duration:   {:.1}s", elapsed.as_secs_f64());
    println!("Brokers:    {} scanned", results.len());

    let total_findings: usize = results.iter().map(|r| r.findings_count).sum();
    let errors: Vec<_> = results.iter().filter(|r| r.error.is_some()).collect();
    let successes: Vec<_> = results
        .iter()
        .filter(|r| r.error.is_none() && r.findings_count > 0)
        .collect();
    let zero_results: Vec<_> = results
        .iter()
        .filter(|r| r.error.is_none() && r.findings_count == 0)
        .collect();

    println!("Findings:   {total_findings}");
    println!("Successes:  {} (with findings)", successes.len());
    println!("Zero:       {} (no findings, no error)", zero_results.len());
    println!("Errors:     {}", errors.len());
    println!();

    println!("--- Per-broker results ---");
    for r in &results {
        let status = if let Some(ref err) = r.error {
            format!("ERROR: {err}")
        } else if r.findings_count > 0 {
            format!("✓ {} finding(s)", r.findings_count)
        } else {
            "✗ 0 findings".to_string()
        };
        println!("  [{}] {}", r.broker_id, status);
    }

    println!();
    if total_findings == 0 {
        println!("RESULT: ZERO FINDINGS across all brokers scanned.");
        println!("This confirms the known broken scan engine state (see NEV-4/NEV-6).");
        println!();
        println!("Known root causes:");
        println!("  1. Many broker sites block static HTTP (Cloudflare/CAPTCHA)");
        println!("  2. CSS selectors in broker definitions may be stale/incorrect");
        println!("  3. Browser stealth settings may be insufficient for some sites");
        println!("  4. Similarity scoring stub returns no matches (fixed in NEV-6)");
    } else {
        println!(
            "RESULT: {} finding(s) across {} broker(s)!",
            total_findings,
            successes.len()
        );
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
