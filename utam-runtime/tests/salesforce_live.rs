//! Salesforce live integration tests
//!
//! Runs against a real Salesforce org using a single browser session.
//! The frontdoor token can only establish one session, so all test steps
//! share the same WebDriver instance.
//!
//! Emits Allure-format JSON results to `allure-results/` for rich reporting.
//!
//! Skipped locally (no CHROMEDRIVER_URL), panics in CI if credentials missing.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use utam_runtime::prelude::*;

// ---------------------------------------------------------------------------
// Allure reporting helpers
// ---------------------------------------------------------------------------

fn now_millis() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

/// Generate a deterministic UUID-like string from a seed.
fn make_uuid(seed: &str) -> String {
    let mut h = DefaultHasher::new();
    seed.hash(&mut h);
    let n = h.finish();
    // Add timestamp for uniqueness across runs
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

/// A single Allure step with optional screenshot attachment.
struct AllureStep {
    name: String,
    status: String,
    start: u64,
    stop: u64,
    attachments: Vec<serde_json::Value>,
    details: Option<(String, String)>, // (message, trace)
}

/// Tracks the overall test result and its steps for Allure output.
struct AllureReport {
    test_uuid: String,
    test_name: String,
    start: u64,
    steps: Vec<AllureStep>,
    current_step_start: u64,
}

impl AllureReport {
    fn new(test_name: &str) -> Self {
        let start = now_millis();
        Self {
            test_uuid: make_uuid(&format!("{test_name}-{start}")),
            test_name: test_name.to_string(),
            start,
            steps: Vec::new(),
            current_step_start: start,
        }
    }

    fn begin_step(&mut self) {
        self.current_step_start = now_millis();
    }

    /// Save a screenshot to allure-results and return the attachment JSON.
    fn save_screenshot_attachment(&self, png_data: &[u8], name: &str) -> Option<serde_json::Value> {
        let att_uuid = make_uuid(&format!("{}-{name}", self.test_uuid));
        let filename = format!("{att_uuid}-attachment.png");
        let path = allure_results_dir().join(&filename);
        std::fs::write(&path, png_data).ok()?;

        // Also save to artifacts dir for the existing artifact upload
        let artifacts = artifacts_dir();
        let _ = std::fs::write(artifacts.join(format!("{name}.png")), png_data);

        Some(serde_json::json!({
            "name": name,
            "source": filename,
            "type": "image/png"
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
            details: Some((message.to_string(), String::new())),
        });
    }

    /// Write the Allure result JSON file.
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
                if let Some((ref msg, ref trace)) = s.details {
                    step["statusDetails"] = serde_json::json!({
                        "message": msg,
                        "trace": trace
                    });
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
                { "name": "suite", "value": "Salesforce Integration" },
                { "name": "parentSuite", "value": "Live Tests" },
                { "name": "framework", "value": "busbar-sf-utam" },
                { "name": "language", "value": "rust" },
                { "name": "feature", "value": "Salesforce Browser Testing" },
                { "name": "severity", "value": "critical" }
            ],
            "steps": steps_json,
            "attachments": [],
            "parameters": [],
            "links": []
        });

        if let Some(msg) = message {
            result["statusDetails"] = serde_json::json!({
                "message": msg,
                "trace": ""
            });
        }

        let path = allure_results_dir().join(format!("{}-result.json", self.test_uuid));
        let json = serde_json::to_string_pretty(&result).unwrap_or_default();
        let _ = std::fs::write(&path, &json);
        eprintln!("Allure result written to {}", path.display());
    }
}

/// Write Allure environment.properties file.
fn write_allure_environment(instance_url: &str) {
    let dir = allure_results_dir();
    let content = format!(
        "sf.instance={instance_url}\n\
         browser=Chrome (headless)\n\
         driver=chromedriver\n\
         framework=busbar-sf-utam\n\
         language=Rust\n"
    );
    let _ = std::fs::write(dir.join("environment.properties"), content);
}

/// Write Allure categories.json file.
fn write_allure_categories() {
    let categories = serde_json::json!([
        {
            "name": "Authentication failures",
            "messageRegex": ".*login.*|.*frontdoor.*|.*auth.*",
            "matchedStatuses": ["failed", "broken"]
        },
        {
            "name": "Lightning load failures",
            "messageRegex": ".*Lightning.*|.*timeout.*|.*load.*",
            "matchedStatuses": ["failed", "broken"]
        },
        {
            "name": "Element not found",
            "messageRegex": ".*not found.*|.*Unable to locate.*",
            "matchedStatuses": ["failed"]
        },
        {
            "name": "Infrastructure problems",
            "messageRegex": ".*WebDriver.*|.*connection.*|.*chromedriver.*",
            "matchedStatuses": ["broken"]
        }
    ]);
    let _ = std::fs::write(
        allure_results_dir().join("categories.json"),
        serde_json::to_string_pretty(&categories).unwrap_or_default(),
    );
}

// ---------------------------------------------------------------------------
// Original helpers
// ---------------------------------------------------------------------------

fn require_sf_credentials() -> Option<(String, String)> {
    let instance = std::env::var("SF_INSTANCE_URL").ok();
    let frontdoor = std::env::var("SF_FRONTDOOR_URL").ok();

    match (instance, frontdoor) {
        (Some(i), Some(f)) if !i.is_empty() && !f.is_empty() => Some((i, f)),
        _ => {
            if std::env::var("CHROMEDRIVER_URL").is_ok() {
                panic!(
                    "SF_INSTANCE_URL and SF_FRONTDOOR_URL must be set in CI!\n\
                     SF_INSTANCE_URL={}\n\
                     SF_FRONTDOOR_URL=({} chars)",
                    std::env::var("SF_INSTANCE_URL").unwrap_or_else(|_| "<not set>".into()),
                    std::env::var("SF_FRONTDOOR_URL").map(|s| s.len()).unwrap_or(0),
                );
            }
            None
        }
    }
}

fn chromedriver_url() -> String {
    std::env::var("CHROMEDRIVER_URL").unwrap_or_else(|_| "http://localhost:9515".to_string())
}

fn artifacts_dir() -> PathBuf {
    let dir = std::env::var("SF_ARTIFACTS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/sf-artifacts"));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Take a screenshot and return the PNG bytes (for Allure attachment).
async fn take_screenshot(driver: &dyn UtamDriver) -> Option<Vec<u8>> {
    driver.screenshot_png().await.ok()
}

#[cfg(feature = "webdriver")]
async fn create_driver() -> RuntimeResult<Box<dyn UtamDriver>> {
    use thirtyfour::prelude::*;

    let url = chromedriver_url();
    let mut caps = DesiredCapabilities::chrome();
    let _ = caps.set_headless();
    let _ = caps.set_no_sandbox();
    let _ = caps.set_disable_gpu();
    let _ = caps.add_arg("--window-size=1920,1080");
    let _ = caps.add_arg("--disable-dev-shm-usage");

    let driver = WebDriver::new(&url, caps).await.map_err(|e| RuntimeError::UnsupportedAction {
        action: "create_driver".into(),
        element_type: format!("WebDriver connection to {url} failed: {e}"),
    })?;

    Ok(Box::new(ThirtyfourDriver::new(driver)))
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

async fn wait_for_lightning_loaded(driver: &dyn UtamDriver) -> bool {
    for attempt in 1..=30 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let result = driver
            .execute_script(
                "return !!(document.querySelector('.oneHeader') || \
                 document.querySelector('.desktop.container.forceStyle') || \
                 document.querySelector('one-app-nav-bar'))",
                vec![],
            )
            .await;

        match result {
            Ok(serde_json::Value::Bool(true)) => {
                eprintln!("  Lightning detected on attempt {attempt}");
                return true;
            }
            _ => {
                if attempt % 5 == 0 {
                    let url = driver.current_url().await.unwrap_or_default();
                    eprintln!("  Attempt {attempt}/30 — still waiting... URL: {url}");
                }
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Single integration test — one browser session, sequential steps
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_salesforce_live() {
    let Some((instance_url, frontdoor_url)) = require_sf_credentials() else {
        eprintln!("SKIP: SF credentials not set");
        return;
    };

    // Initialize Allure reporting
    let mut allure = AllureReport::new("test_salesforce_live");
    write_allure_environment(&instance_url);
    write_allure_categories();

    let driver = create_driver().await.expect("Failed to create driver");

    // ── Step 1: Frontdoor authentication ─────────────────────────────
    allure.begin_step();
    eprintln!("\n=== Step 1: Frontdoor Authentication ===");
    eprintln!("Navigating to frontdoor URL ({} chars)", frontdoor_url.len());
    driver.navigate(&frontdoor_url).await.expect("Failed to navigate to frontdoor");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let url = driver.current_url().await.unwrap_or_default();
    eprintln!("After frontdoor: {url}");

    let mut attachments = Vec::new();
    if let Some(png) = take_screenshot(driver.as_ref()).await {
        if let Some(att) = allure.save_screenshot_attachment(&png, "01-after-frontdoor") {
            attachments.push(att);
        }
    }

    if url.contains("/login.") || url.contains("Login&") {
        allure.end_step_failed(
            "Frontdoor Authentication",
            &format!("Landed on login page: {url}"),
            attachments,
        );
        allure.finish("failed", Some(&format!("Auth failed — landed on login page: {url}")));
        panic!("Frontdoor auth failed — landed on login page: {url}");
    }
    allure.end_step("Frontdoor Authentication", "passed", attachments);

    // ── Step 2: Navigate to Lightning home ───────────────────────────
    allure.begin_step();
    eprintln!("\n=== Step 2: Lightning Home ===");
    let home_url = format!("{instance_url}/lightning/page/home");
    eprintln!("Navigating to: {home_url}");
    driver.navigate(&home_url).await.expect("Failed to navigate to home");

    eprintln!("Waiting for Lightning to load...");
    let lightning_loaded = wait_for_lightning_loaded(driver.as_ref()).await;

    let url = driver.current_url().await.unwrap_or_default();
    eprintln!("Lightning home: {url}");

    let mut attachments = Vec::new();
    if let Some(png) = take_screenshot(driver.as_ref()).await {
        if let Some(att) = allure.save_screenshot_attachment(&png, "02-lightning-home") {
            attachments.push(att);
        }
    }

    if !lightning_loaded {
        allure.end_step_failed(
            "Navigate to Lightning Home",
            &format!("Lightning did not load. URL: {url}"),
            attachments,
        );
        allure.finish("failed", Some("Lightning did not load within timeout"));
        panic!("Lightning did not load within timeout. URL: {url}");
    }
    assert!(
        url.contains("lightning") || url.contains("force.com"),
        "Should be on a Lightning page, got: {url}"
    );
    allure.end_step("Navigate to Lightning Home", "passed", attachments);

    // ── Step 3: Page object discovery ────────────────────────────────
    allure.begin_step();
    eprintln!("\n=== Step 3: Page Object Discovery ===");
    let registry = load_registry();
    let report = utam_runtime::discovery::discover(driver.as_ref(), &registry)
        .await
        .expect("Discovery failed");

    eprintln!("URL: {}", report.url);
    eprintln!("Known page objects matched: {}", report.matched.len());
    for m in &report.matched {
        eprintln!("  + {} ({} methods, {} elements)", m.name, m.method_count, m.element_count);
    }
    eprintln!("Unknown components discovered: {}", report.discovered.len());
    for d in report.discovered.iter().take(15) {
        eprintln!("  ? <{}> shadow={} children={}", d.tag_name, d.has_shadow, d.children.len());
    }

    let mut attachments = Vec::new();
    if let Some(png) = take_screenshot(driver.as_ref()).await {
        if let Some(att) = allure.save_screenshot_attachment(&png, "03-after-discovery") {
            attachments.push(att);
        }
    }

    // Attach discovery report as JSON
    let report_json = serde_json::to_string_pretty(&report).unwrap_or_default();
    let report_path = artifacts_dir().join("discovery-report.json");
    let _ = std::fs::write(&report_path, &report_json);
    eprintln!("Discovery report saved to {}", report_path.display());

    // Also save discovery report as an Allure attachment
    let disc_uuid = make_uuid("discovery-report");
    let disc_filename = format!("{disc_uuid}-attachment.json");
    let _ = std::fs::write(allure_results_dir().join(&disc_filename), &report_json);
    attachments.push(serde_json::json!({
        "name": "discovery-report.json",
        "source": disc_filename,
        "type": "application/json"
    }));

    let total = report.matched.len() + report.discovered.len();
    if total <= 5 {
        allure.end_step_failed(
            &format!("Page Object Discovery ({total} found)"),
            &format!("Expected >5 components, got {total}"),
            attachments,
        );
        allure.finish("failed", Some(&format!("Only {total} components found")));
        panic!("Expected >5 components on a Lightning page, got {total}");
    }
    allure.end_step(
        &format!(
            "Page Object Discovery ({} matched, {} discovered)",
            report.matched.len(),
            report.discovered.len()
        ),
        "passed",
        attachments,
    );

    // ── Step 4: Header page object ───────────────────────────────────
    allure.begin_step();
    eprintln!("\n=== Step 4: Header Page Object ===");
    let header_matches = registry.search("global/header");
    assert!(!header_matches.is_empty(), "global/header not found in registry");

    let header_ast = registry.get(&header_matches[0]).expect("Failed to get header AST");
    eprintln!("Header: {}", header_matches[0]);
    eprintln!("  Root selector: {:?}", header_ast.selector.as_ref().map(|s| &s.css));
    eprintln!("  Methods: {:?}", header_ast.methods.iter().map(|m| &m.name).collect::<Vec<_>>());

    let root_css = header_ast
        .selector
        .as_ref()
        .and_then(|s| s.css.as_ref())
        .expect("Header should have a root CSS selector")
        .clone();
    eprintln!("Looking for header root: {root_css}");
    let selector = Selector::Css(root_css.clone());
    let header_el = driver.find_element(&selector).await;

    let mut attachments = Vec::new();
    if let Some(png) = take_screenshot(driver.as_ref()).await {
        let name = if header_el.is_ok() { "04-header-found" } else { "04-header-not-found" };
        if let Some(att) = allure.save_screenshot_attachment(&png, name) {
            attachments.push(att);
        }
    }

    match header_el {
        Ok(_) => {
            eprintln!("Header element found!");
            allure.end_step("Header Page Object (.oneHeader)", "passed", attachments);
        }
        Err(e) => {
            let msg = format!("Header root element {root_css} not found: {e}");
            allure.end_step_failed("Header Page Object (.oneHeader)", &msg, attachments);
            allure.finish("failed", Some(&msg));
            panic!("{msg}");
        }
    }

    // ── Step 5: Navigate to Accounts and Contacts ────────────────────
    allure.begin_step();
    eprintln!("\n=== Step 5: Navigate Pages ===");

    let accounts_url = format!("{instance_url}/lightning/o/Account/list");
    eprintln!("Navigating to Accounts: {accounts_url}");
    driver.navigate(&accounts_url).await.expect("Accounts navigate failed");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    let url = driver.current_url().await.unwrap_or_default();
    eprintln!("Accounts: {url}");

    let mut attachments = Vec::new();
    if let Some(png) = take_screenshot(driver.as_ref()).await {
        if let Some(att) = allure.save_screenshot_attachment(&png, "05a-accounts") {
            attachments.push(att);
        }
    }

    let contacts_url = format!("{instance_url}/lightning/o/Contact/list");
    eprintln!("Navigating to Contacts: {contacts_url}");
    driver.navigate(&contacts_url).await.expect("Contacts navigate failed");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    let url = driver.current_url().await.unwrap_or_default();
    eprintln!("Contacts: {url}");

    if let Some(png) = take_screenshot(driver.as_ref()).await {
        if let Some(att) = allure.save_screenshot_attachment(&png, "05b-contacts") {
            attachments.push(att);
        }
    }

    allure.end_step("Navigate Pages (Accounts + Contacts)", "passed", attachments);

    driver.quit().await.expect("Failed to quit");

    // Write the final Allure result
    allure.finish("passed", None);
    eprintln!("\n=== All steps completed ===");
}
