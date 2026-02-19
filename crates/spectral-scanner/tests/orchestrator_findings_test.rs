use spectral_broker::definition::{
    BrokerCategory, BrokerDefinition, BrokerMetadata, RemovalDifficulty, RemovalMethod,
    ResultSelectors, SearchMethod,
};
use spectral_broker::BrokerRegistry;
use spectral_browser::BrowserEngine;
use spectral_core::{BrokerId, PiiField};
use spectral_db::Database;
use spectral_scanner::ScanOrchestrator;
use std::sync::Arc;

/// Helper to create a test broker definition with result selectors
fn create_test_broker_with_selectors(
    broker_id: &str,
    result_selectors: Option<ResultSelectors>,
) -> BrokerDefinition {
    BrokerDefinition {
        broker: BrokerMetadata {
            id: BrokerId::new(broker_id).expect("valid broker ID"),
            name: format!("Test Broker {}", broker_id),
            url: format!("https://{}.example.com", broker_id),
            domain: format!("{}.example.com", broker_id),
            category: BrokerCategory::PeopleSearch,
            difficulty: RemovalDifficulty::Easy,
            typical_removal_days: 7,
            recheck_interval_days: 30,
            last_verified: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid date"),
            scan_priority: spectral_broker::ScanPriority::OnRequest,
            region_relevance: vec!["Global".to_string()],
        },
        search: SearchMethod::UrlTemplate {
            template: format!(
                "https://{}.example.com/search?name={{first}}-{{last}}",
                broker_id
            ),
            requires_fields: vec![PiiField::FirstName, PiiField::LastName],
            result_selectors,
        },
        removal: RemovalMethod::Manual {
            instructions: "Manual removal instructions".to_string(),
        },
    }
}

/// Helper to create a scan job and broker scan in the database
async fn create_test_scan_context(db: &Database) -> (String, String, String) {
    let profile_id = "test-profile-id";

    // Create profile first (to satisfy foreign key constraint)
    // Using dummy encrypted data and nonce
    sqlx::query(
        "INSERT INTO profiles (id, data, nonce, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(profile_id)
    .bind(vec![0u8; 32]) // Dummy encrypted data
    .bind(vec![0u8; 12]) // Dummy nonce
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(db.pool())
    .await
    .expect("create profile");

    // Create scan job
    let scan_job = spectral_db::scan_jobs::create_scan_job(db.pool(), profile_id.to_string(), 1)
        .await
        .expect("create scan job");

    // Create broker scan
    let broker_scan = spectral_db::broker_scans::create_broker_scan(
        db.pool(),
        scan_job.id.clone(),
        "test-broker".to_string(),
    )
    .await
    .expect("create broker scan");

    (scan_job.id, broker_scan.id, profile_id.to_string())
}

#[tokio::test]
#[ignore = "Requires Chrome browser - run with --ignored"]
async fn test_parse_findings_with_valid_selectors() {
    // Setup
    let key = [0x42; 32];
    let db = Database::new(":memory:", key.to_vec())
        .await
        .expect("create db");
    db.run_migrations().await.expect("run migrations");

    let db = Arc::new(db);

    // Create broker registry with test broker
    let broker_registry = BrokerRegistry::new();
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
    let broker_def = create_test_broker_with_selectors("test-broker", Some(selectors));
    broker_registry
        .insert(broker_def.clone())
        .expect("insert broker");

    let broker_registry = Arc::new(broker_registry);
    let browser_engine = Arc::new(BrowserEngine::new().await.expect("create browser"));

    let orchestrator = ScanOrchestrator::new(broker_registry, browser_engine, db.clone());

    // Create scan context
    let (scan_job_id, broker_scan_id, profile_id) = create_test_scan_context(&db).await;

    // HTML with 2 matching listings
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

    // Parse and store findings
    let broker_id = BrokerId::new("test-broker").expect("valid broker ID");
    let findings_count = orchestrator
        .parse_and_store_findings(html, &broker_scan_id, &broker_id, &profile_id)
        .await
        .expect("parse and store findings");

    // Verify 2 findings were created
    assert_eq!(findings_count, 2);

    // Verify findings in database
    let findings = spectral_db::findings::get_by_scan_job(db.pool(), &scan_job_id)
        .await
        .expect("get findings");

    assert_eq!(findings.len(), 2);

    // Verify first finding
    let finding1 = &findings[0];
    assert_eq!(finding1.broker_scan_id, broker_scan_id);
    assert_eq!(finding1.broker_id, "test-broker");
    assert_eq!(finding1.profile_id, profile_id);
    assert!(finding1.listing_url.contains("/profile/"));
    assert_eq!(
        finding1.verification_status,
        spectral_db::findings::VerificationStatus::PendingVerification
    );

    // Verify extracted data contains expected fields
    let data = &finding1.extracted_data;
    assert!(data["name"].is_string() || data["name"].is_null());
    assert!(data["age"].is_number() || data["age"].is_null());
    assert!(data["addresses"].is_array());
}

#[tokio::test]
#[ignore = "Requires Chrome browser - run with --ignored"]
async fn test_deduplication_prevents_duplicates() {
    // Setup
    let key = [0x42; 32];
    let db = Database::new(":memory:", key.to_vec())
        .await
        .expect("create db");
    db.run_migrations().await.expect("run migrations");

    let db = Arc::new(db);

    // Create broker registry
    let broker_registry = BrokerRegistry::new();
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
    let broker_def = create_test_broker_with_selectors("test-broker", Some(selectors));
    broker_registry
        .insert(broker_def.clone())
        .expect("insert broker");

    let broker_registry = Arc::new(broker_registry);
    let browser_engine = Arc::new(BrowserEngine::new().await.expect("create browser"));

    let orchestrator = ScanOrchestrator::new(broker_registry, browser_engine, db.clone());

    // Create scan context
    let (scan_job_id, broker_scan_id, profile_id) = create_test_scan_context(&db).await;

    // HTML with the same listing twice (same URL)
    let html = r#"
        <div class="search-results">
            <div class="result-card">
                <a class="profile-link" href="/profile/john-doe-123">View Profile</a>
                <div class="name">John Doe</div>
                <div class="age">35</div>
                <div class="location">Springfield, CA</div>
            </div>
            <div class="result-card">
                <a class="profile-link" href="/profile/john-doe-123">View Profile</a>
                <div class="name">John Doe</div>
                <div class="age">35</div>
                <div class="location">Springfield, CA</div>
            </div>
        </div>
    "#;

    // Parse and store findings
    let broker_id = BrokerId::new("test-broker").expect("valid broker ID");
    let findings_count = orchestrator
        .parse_and_store_findings(html, &broker_scan_id, &broker_id, &profile_id)
        .await
        .expect("parse and store findings");

    // Verify only 1 finding was created (duplicate was skipped)
    assert_eq!(findings_count, 1);

    // Verify only 1 finding in database
    let findings = spectral_db::findings::get_by_scan_job(db.pool(), &scan_job_id)
        .await
        .expect("get findings");

    assert_eq!(findings.len(), 1);
}

#[tokio::test]
#[ignore = "Requires Chrome browser - run with --ignored"]
async fn test_missing_selectors_logs_warning() {
    // Setup
    let key = [0x42; 32];
    let db = Database::new(":memory:", key.to_vec())
        .await
        .expect("create db");
    db.run_migrations().await.expect("run migrations");

    let db = Arc::new(db);

    // Create broker registry with no selectors
    let broker_registry = BrokerRegistry::new();
    let broker_def = create_test_broker_with_selectors("test-broker", None);
    broker_registry
        .insert(broker_def.clone())
        .expect("insert broker");

    let broker_registry = Arc::new(broker_registry);
    let browser_engine = Arc::new(BrowserEngine::new().await.expect("create browser"));

    let orchestrator = ScanOrchestrator::new(broker_registry, browser_engine, db.clone());

    // Create scan context
    let (_scan_job_id, broker_scan_id, profile_id) = create_test_scan_context(&db).await;

    // HTML content (doesn't matter since selectors are missing)
    let html = r#"<div class="search-results"></div>"#;

    // Parse and store findings
    let broker_id = BrokerId::new("test-broker").expect("valid broker ID");
    let findings_count = orchestrator
        .parse_and_store_findings(html, &broker_scan_id, &broker_id, &profile_id)
        .await
        .expect("should return Ok(0) when selectors missing");

    // Verify returns 0
    assert_eq!(findings_count, 0);
}

#[tokio::test]
#[ignore = "Requires Chrome browser - run with --ignored"]
async fn test_parse_failure_returns_ok_zero() {
    // Setup
    let key = [0x42; 32];
    let db = Database::new(":memory:", key.to_vec())
        .await
        .expect("create db");
    db.run_migrations().await.expect("run migrations");

    let db = Arc::new(db);

    // Create broker registry with invalid selectors
    let broker_registry = BrokerRegistry::new();
    let selectors = ResultSelectors {
        results_container: "[[[[invalid".to_string(), // Invalid CSS selector
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
    let broker_def = create_test_broker_with_selectors("test-broker", Some(selectors));
    broker_registry
        .insert(broker_def.clone())
        .expect("insert broker");

    let broker_registry = Arc::new(broker_registry);
    let browser_engine = Arc::new(BrowserEngine::new().await.expect("create browser"));

    let orchestrator = ScanOrchestrator::new(broker_registry, browser_engine, db.clone());

    // Create scan context
    let (scan_job_id, broker_scan_id, profile_id) = create_test_scan_context(&db).await;

    // HTML content
    let html = r#"<div class="search-results"></div>"#;

    // Parse and store findings
    let broker_id = BrokerId::new("test-broker").expect("valid broker ID");
    let findings_count = orchestrator
        .parse_and_store_findings(html, &broker_scan_id, &broker_id, &profile_id)
        .await
        .expect("should return Ok(0) even on parse failure");

    // Verify returns 0
    assert_eq!(findings_count, 0);

    // Verify no findings in database
    let findings = spectral_db::findings::get_by_scan_job(db.pool(), &scan_job_id)
        .await
        .expect("get findings");

    assert_eq!(findings.len(), 0);
}
