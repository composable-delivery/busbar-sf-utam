//! Salesforce live integration tests
//!
//! Runs against a real Salesforce org. Authenticates via busbar-sf-api,
//! seeds test data, then exercises page objects against the live DOM.
//!
//! Uses DynamicPageObject with PageObjectRegistry for custom component
//! type resolution and compose method chaining.
//!
//! Skipped locally (no CHROMEDRIVER_URL), panics in CI if credentials missing.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use busbar_sf_api::{SObjectRecord, SalesforceClient, SfdxAuthUrl};
use utam_runtime::prelude::*;

// ---------------------------------------------------------------------------
// Allure reporting (minimal, focused)
// ---------------------------------------------------------------------------

fn now_millis() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

fn make_uuid(seed: &str) -> String {
    let mut h = DefaultHasher::new();
    seed.hash(&mut h);
    let n = h.finish();
    let mut h2 = DefaultHasher::new();
    now_millis().hash(&mut h2);
    seed.hash(&mut h2);
    let n2 = h2.finish();
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        (n >> 32) as u32,
        (n >> 16) as u16,
        (n & 0xffff) as u16,
        (n2 >> 48) as u16,
        n2 & 0x0000_ffff_ffff_ffff
    )
}

fn allure_results_dir() -> PathBuf {
    let dir = std::env::var("ALLURE_RESULTS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/allure-results"));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

struct AllureStep {
    name: String,
    status: String,
    start: u64,
    stop: u64,
    attachments: Vec<serde_json::Value>,
    details: Option<String>,
}

struct AllureReport {
    test_uuid: String,
    test_name: String,
    suite: String,
    start: u64,
    steps: Vec<AllureStep>,
    current_step_start: u64,
    top_attachments: Vec<serde_json::Value>,
}

impl AllureReport {
    fn new(test_name: &str, suite: &str) -> Self {
        let start = now_millis();
        Self {
            test_uuid: make_uuid(&format!("{test_name}-{start}")),
            test_name: test_name.to_string(),
            suite: suite.to_string(),
            start,
            steps: Vec::new(),
            current_step_start: start,
            top_attachments: Vec::new(),
        }
    }

    fn begin_step(&mut self) {
        self.current_step_start = now_millis();
    }

    fn save_screenshot(&self, png_data: &[u8], name: &str) -> Option<serde_json::Value> {
        let att_uuid = make_uuid(&format!("{}-{name}", self.test_uuid));
        let filename = format!("{att_uuid}-attachment.png");
        std::fs::write(allure_results_dir().join(&filename), png_data).ok()?;
        Some(serde_json::json!({
            "name": name, "source": filename, "type": "image/png"
        }))
    }

    fn end_step(&mut self, name: &str, status: &str, attachments: Vec<serde_json::Value>) {
        self.steps.push(AllureStep {
            name: name.to_string(),
            status: status.to_string(),
            start: self.current_step_start,
            stop: now_millis(),
            attachments,
            details: None,
        });
    }

    fn end_step_failed(&mut self, name: &str, message: &str, attachments: Vec<serde_json::Value>) {
        self.steps.push(AllureStep {
            name: name.to_string(),
            status: "failed".to_string(),
            start: self.current_step_start,
            stop: now_millis(),
            attachments,
            details: Some(message.to_string()),
        });
    }

    fn finish(&self, status: &str, message: Option<&str>) {
        let steps_json: Vec<serde_json::Value> = self
            .steps
            .iter()
            .map(|s| {
                let mut step = serde_json::json!({
                    "name": s.name,
                    "status": s.status,
                    "stage": "finished",
                    "start": s.start,
                    "stop": s.stop,
                    "attachments": s.attachments,
                    "steps": [],
                    "parameters": []
                });
                if let Some(ref msg) = s.details {
                    step["statusDetails"] = serde_json::json!({ "message": msg, "trace": "" });
                }
                step
            })
            .collect();

        let mut result = serde_json::json!({
            "uuid": self.test_uuid,
            "historyId": make_uuid(&self.test_name),
            "fullName": format!("salesforce_live::{}", self.test_name),
            "name": self.test_name,
            "status": status,
            "stage": "finished",
            "start": self.start,
            "stop": now_millis(),
            "labels": [
                { "name": "parentSuite", "value": "Salesforce Integration" },
                { "name": "suite", "value": self.suite },
                { "name": "subSuite", "value": self.test_name },
                { "name": "framework", "value": "busbar-sf-utam" },
                { "name": "language", "value": "rust" },
                { "name": "feature", "value": "Salesforce Browser Testing" },
                { "name": "severity", "value": "normal" }
            ],
            "steps": steps_json,
            "attachments": self.top_attachments,
            "parameters": [],
            "links": []
        });

        if let Some(msg) = message {
            result["statusDetails"] = serde_json::json!({ "message": msg, "trace": "" });
        }

        let path = allure_results_dir().join(format!("{}-result.json", self.test_uuid));
        let json = serde_json::to_string_pretty(&result).unwrap_or_default();
        let _ = std::fs::write(&path, &json);
    }
}

fn write_allure_environment(instance_url: &str) {
    let content = format!(
        "sf.instance={instance_url}\n\
         browser=Chrome\n\
         driver=chromedriver\n\
         framework=busbar-sf-utam\n\
         language=Rust\n"
    );
    let _ = std::fs::write(allure_results_dir().join("environment.properties"), content);
}

fn write_allure_categories() {
    let categories = serde_json::json!([
        { "name": "Auth failures", "messageRegex": ".*login.*|.*frontdoor.*|.*auth.*", "matchedStatuses": ["failed"] },
        { "name": "Element not found", "messageRegex": ".*not found.*|.*Unable to locate.*", "matchedStatuses": ["failed"] },
        { "name": "Infrastructure", "messageRegex": ".*WebDriver.*|.*connection.*", "matchedStatuses": ["broken"] }
    ]);
    let _ = std::fs::write(
        allure_results_dir().join("categories.json"),
        serde_json::to_string_pretty(&categories).unwrap_or_default(),
    );
}

// ---------------------------------------------------------------------------
// Login detection
// ---------------------------------------------------------------------------

fn is_login_page(url: &str) -> bool {
    let dominated_by_login = url.contains("/login")
        || url.contains("Login&")
        || url.contains("login.salesforce.com")
        || url.contains("test.salesforce.com/login");
    let is_lightning = url.contains("/lightning/");
    dominated_by_login && !is_lightning
}

async fn wait_for_lightning(driver: &dyn UtamDriver) -> bool {
    for attempt in 1..=20 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let url = driver.current_url().await.unwrap_or_default();
        if is_login_page(&url) {
            return false;
        }
        let result = driver
            .execute_script(
                "return !!(document.querySelector('.oneHeader') || \
                 document.querySelector('.desktop.container.forceStyle') || \
                 document.querySelector('one-app-nav-bar'))",
                vec![],
            )
            .await;
        if matches!(result, Ok(serde_json::Value::Bool(true))) {
            eprintln!("  Lightning detected on attempt {attempt}");
            return true;
        }
        if attempt % 5 == 0 {
            eprintln!("  Attempt {attempt}/20 — waiting... URL: {url}");
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

async fn connect_salesforce() -> Option<SalesforceClient> {
    let auth_url = match std::env::var("SF_AUTH_URL") {
        Ok(url) if !url.is_empty() => url,
        _ => {
            if std::env::var("CHROMEDRIVER_URL").is_ok() {
                panic!("SF_AUTH_URL must be set in CI!");
            }
            return None;
        }
    };
    let parsed = SfdxAuthUrl::parse(&auth_url).expect("Failed to parse SF_AUTH_URL");
    let client =
        SalesforceClient::from_auth_url(&parsed).await.expect("Failed to exchange refresh token");
    eprintln!("Authenticated to {}", client.instance_url);
    Some(client)
}

async fn seed_test_data(client: &SalesforceClient) -> Vec<(String, String)> {
    eprintln!("Seeding test data...");
    let records = vec![
        (
            "account1",
            "Account",
            SObjectRecord::new()
                .field("Name", "Acme Corp")
                .field("Industry", "Technology")
                .field("Phone", "(555) 123-4567"),
        ),
        (
            "contact1",
            "Contact",
            SObjectRecord::new()
                .field("FirstName", "Jane")
                .field("LastName", "Doe")
                .field("Email", "jane.doe@example.com")
                .field("AccountId", "@{account1.id}"),
        ),
        (
            "opportunity1",
            "Opportunity",
            SObjectRecord::new()
                .field("Name", "Acme Deal")
                .field("StageName", "Prospecting")
                .field("CloseDate", "2026-12-31")
                .field("AccountId", "@{account1.id}"),
        ),
        (
            "lead1",
            "Lead",
            SObjectRecord::new()
                .field("FirstName", "John")
                .field("LastName", "Smith")
                .field("Company", "Smith Industries"),
        ),
        (
            "case1",
            "Case",
            SObjectRecord::new()
                .field("Subject", "Test Support Case")
                .field("Status", "New")
                .field("Origin", "Web")
                .field("AccountId", "@{account1.id}"),
        ),
    ];

    match client.create_related(records).await {
        Ok(ids) => {
            let types = ["Account", "Contact", "Opportunity", "Lead", "Case"];
            let pairs: Vec<(String, String)> =
                types.iter().zip(ids.iter()).map(|(t, id)| (t.to_string(), id.clone())).collect();
            for (t, id) in &pairs {
                eprintln!("  Created {t}: {id}");
            }
            // Mark as recently viewed
            for sobject_type in &types {
                let soql = format!(
                    "SELECT Id FROM {sobject_type} ORDER BY CreatedDate DESC LIMIT 200 FOR VIEW"
                );
                match client.query(&soql).await {
                    Ok(rows) => eprintln!("  Marked {} {sobject_type} recently viewed", rows.len()),
                    Err(e) => eprintln!("  WARNING: FOR VIEW failed for {sobject_type}: {e}"),
                }
            }
            pairs
        }
        Err(e) => {
            eprintln!("WARNING: Failed to seed test data: {e}");
            Vec::new()
        }
    }
}

async fn cleanup_test_data(client: &SalesforceClient, records: &[(String, String)]) {
    for (sobject_type, id) in records.iter().rev() {
        match client.delete(sobject_type, id).await {
            Ok(()) => eprintln!("  Deleted {sobject_type}/{id}"),
            Err(e) => eprintln!("  Failed to delete {sobject_type}/{id}: {e}"),
        }
    }
}

fn chromedriver_url() -> String {
    std::env::var("CHROMEDRIVER_URL").unwrap_or_else(|_| "http://localhost:9515".to_string())
}

#[cfg(feature = "webdriver")]
async fn create_driver() -> Arc<dyn UtamDriver> {
    use thirtyfour::prelude::*;
    let url = chromedriver_url();
    let mut caps = DesiredCapabilities::chrome();
    let has_display = std::env::var("DISPLAY").is_ok();
    if !has_display {
        let _ = caps.set_headless();
    }
    let _ = caps.set_no_sandbox();
    let _ = caps.set_disable_gpu();
    let _ = caps.add_arg("--disable-dev-shm-usage");
    if has_display {
        let _ = caps.add_arg("--start-maximized");
    } else {
        let _ = caps.add_arg("--window-size=1920,1080");
    }
    let driver = WebDriver::new(&url, caps)
        .await
        .unwrap_or_else(|e| panic!("WebDriver connection to {url} failed: {e}"));
    Arc::new(ThirtyfourDriver::new(driver))
}

fn load_registry() -> PageObjectRegistry {
    let mut registry = PageObjectRegistry::new();
    let sf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../salesforce-pageobjects");
    if sf_path.exists() {
        registry.add_search_path(sf_path);
        let count = registry.scan().unwrap_or(0);
        eprintln!("Registry loaded: {count} page objects");
    }
    registry
}

/// Helper: load a page object by name from the registry, using the shared driver.
async fn load_page_object(
    driver: Arc<dyn UtamDriver>,
    registry: &Arc<PageObjectRegistry>,
    name: &str,
) -> Result<DynamicPageObject, String> {
    let matches = registry.search(name);
    if matches.is_empty() {
        return Err(format!("{name} not found in registry"));
    }
    let ast = registry.get(&matches[0]).map_err(|e| format!("{e}"))?;
    DynamicPageObject::load(driver, ast)
        .await
        .map(|po| po.with_registry(Arc::clone(registry)))
        .map_err(|e| format!("{e}"))
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_salesforce_live() {
    let Some(sf_client) = connect_salesforce().await else {
        eprintln!("SKIP: SF credentials not set");
        return;
    };

    let instance_url = sf_client.instance_url.clone();
    let frontdoor_url = sf_client.frontdoor_url();

    write_allure_environment(&instance_url);
    write_allure_categories();

    // ── Seed test data ──────────────────────────────────────────────
    eprintln!("\n=== Seed Test Data ===");
    let seeded_records = seed_test_data(&sf_client).await;

    let driver = create_driver().await;
    let registry = Arc::new(load_registry());

    // ── Authenticate ────────────────────────────────────────────────
    eprintln!("\n=== Authenticate ===");
    driver.navigate(&frontdoor_url).await.expect("Failed to navigate to frontdoor");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let url = driver.current_url().await.unwrap_or_default();
    if is_login_page(&url) {
        cleanup_test_data(&sf_client, &seeded_records).await;
        panic!("Frontdoor auth failed — landed on login page: {url}");
    }
    eprintln!("After frontdoor: {url}");

    let home_url = format!("{instance_url}/lightning/page/home");
    driver.navigate(&home_url).await.expect("Failed to navigate to home");
    assert!(wait_for_lightning(driver.as_ref()).await, "Lightning did not load after auth");
    eprintln!("Lightning loaded");

    // Signal to CI: safe to start video recording
    let _ = std::fs::write(allure_results_dir().join("browser-ready"), "1");

    // ── Test: Header page object ────────────────────────────────────
    eprintln!("\n=== Test: Header Page Object ===");
    {
        let mut allure = AllureReport::new("Header: notifications", "Page Objects");
        allure.begin_step();

        match load_page_object(Arc::clone(&driver), &registry, "global/header").await {
            Ok(header) => {
                eprintln!(
                    "  Methods: {:?}",
                    header.method_signatures().iter().map(|m| &m.name).collect::<Vec<_>>()
                );
                let mut att = Vec::new();
                if let Ok(png) = driver.screenshot_png().await {
                    if let Some(a) = allure.save_screenshot(&png, "header-loaded") {
                        att.push(a);
                    }
                }
                allure.end_step("Load global/header", "passed", att);

                // getNotificationCount
                allure.begin_step();
                let args = HashMap::new();
                match header.call_method("getNotificationCount", &args).await {
                    Ok(val) => {
                        eprintln!("  getNotificationCount = {val}");
                        allure.end_step(&format!("getNotificationCount → {val}"), "passed", vec![]);
                    }
                    Err(e) => {
                        eprintln!("  getNotificationCount FAILED: {e}");
                        let mut att = Vec::new();
                        if let Ok(png) = driver.screenshot_png().await {
                            if let Some(a) = allure.save_screenshot(&png, "notif-failed") {
                                att.push(a);
                            }
                        }
                        allure.end_step_failed("getNotificationCount", &format!("{e}"), att);
                    }
                }

                // showSetupMenu (click)
                allure.begin_step();
                match header.call_method("showSetupMenu", &args).await {
                    Ok(_) => {
                        eprintln!("  showSetupMenu clicked");
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        let mut att = Vec::new();
                        if let Ok(png) = driver.screenshot_png().await {
                            if let Some(a) = allure.save_screenshot(&png, "setup-menu-open") {
                                att.push(a);
                            }
                        }
                        allure.end_step("showSetupMenu (click)", "passed", att);
                    }
                    Err(e) => {
                        eprintln!("  showSetupMenu FAILED: {e}");
                        let mut att = Vec::new();
                        if let Ok(png) = driver.screenshot_png().await {
                            if let Some(a) = allure.save_screenshot(&png, "setup-failed") {
                                att.push(a);
                            }
                        }
                        allure.end_step_failed("showSetupMenu", &format!("{e}"), att);
                    }
                }
            }
            Err(e) => {
                eprintln!("  FAILED to load header: {e}");
                let mut att = Vec::new();
                if let Ok(png) = driver.screenshot_png().await {
                    if let Some(a) = allure.save_screenshot(&png, "header-load-failed") {
                        att.push(a);
                    }
                }
                allure.end_step_failed("Load global/header", &e, att);
            }
        }
        let status =
            if allure.steps.iter().any(|s| s.status == "failed") { "failed" } else { "passed" };
        allure.finish(status, None);
    }

    // ── Test: Navigate to Accounts list ─────────────────────────────
    eprintln!("\n=== Test: Accounts List ===");
    {
        let mut allure = AllureReport::new("Navigate: Accounts list", "Navigation");
        allure.begin_step();

        let accounts_url = format!("{instance_url}/lightning/o/Account/list");
        driver.navigate(&accounts_url).await.expect("Failed to navigate to Accounts");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        let url = driver.current_url().await.unwrap_or_default();
        eprintln!("  Accounts URL: {url}");
        let mut att = Vec::new();
        if let Ok(png) = driver.screenshot_png().await {
            if let Some(a) = allure.save_screenshot(&png, "accounts-list") {
                att.push(a);
            }
        }
        if is_login_page(&url) {
            allure.end_step_failed("Navigate to Accounts", "Redirected to login", att);
            allure.finish("failed", Some("Auth lost"));
        } else {
            allure.end_step("Navigate to Accounts", "passed", att);
            allure.finish("passed", None);
        }
    }

    // ── Test: Navigate to seeded Account detail ─────────────────────
    if let Some((_, account_id)) = seeded_records.iter().find(|(t, _)| t == "Account") {
        eprintln!("\n=== Test: Account Detail ===");
        let mut allure = AllureReport::new("Navigate: Account detail (Acme Corp)", "Navigation");
        allure.begin_step();

        let detail_url = format!("{instance_url}/lightning/r/Account/{account_id}/view");
        driver.navigate(&detail_url).await.expect("Failed to navigate to Account detail");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        let url = driver.current_url().await.unwrap_or_default();
        eprintln!("  Detail URL: {url}");
        let mut att = Vec::new();
        if let Ok(png) = driver.screenshot_png().await {
            if let Some(a) = allure.save_screenshot(&png, "account-detail") {
                att.push(a);
            }
        }
        allure.end_step("Navigate to Account detail", "passed", att);
        allure.finish("passed", None);
    }

    // ── Cleanup ─────────────────────────────────────────────────────
    driver.quit().await.expect("Failed to quit");

    if !seeded_records.is_empty() {
        eprintln!("\n=== Cleanup ===");
        cleanup_test_data(&sf_client, &seeded_records).await;
    }

    eprintln!("\n=== Complete ===");
}
