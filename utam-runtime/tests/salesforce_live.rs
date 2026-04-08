//! Salesforce live integration tests
//!
//! Runs against a real Salesforce org. Authenticates once via frontdoor.jsp,
//! then reuses the same browser session to visit multiple pages and run
//! page object discovery on each.
//!
//! The frontdoor token is single-use per browser session, so all pages
//! are visited sequentially in one session rather than parallel sessions
//! with cookie cloning (which proved unreliable).
//!
//! Emits Allure-format JSON results (one per page) for rich reporting.
//!
//! Skipped locally (no CHROMEDRIVER_URL), panics in CI if credentials missing.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use utam_runtime::prelude::*;

// ---------------------------------------------------------------------------
// Page targets
// ---------------------------------------------------------------------------

struct PageTarget {
    name: &'static str,
    url_suffix: &'static str,
    suite: &'static str,
}

const PAGES: &[PageTarget] = &[
    // Core Navigation
    PageTarget { name: "Home", url_suffix: "/lightning/page/home", suite: "Core" },
    PageTarget { name: "Accounts", url_suffix: "/lightning/o/Account/list", suite: "Core" },
    PageTarget { name: "Contacts", url_suffix: "/lightning/o/Contact/list", suite: "Core" },
    // Sales
    PageTarget {
        name: "Opportunities",
        url_suffix: "/lightning/o/Opportunity/list",
        suite: "Sales",
    },
    PageTarget { name: "Leads", url_suffix: "/lightning/o/Lead/list", suite: "Sales" },
    PageTarget { name: "Campaigns", url_suffix: "/lightning/o/Campaign/list", suite: "Sales" },
    // Service & Activities
    PageTarget { name: "Cases", url_suffix: "/lightning/o/Case/list", suite: "Service" },
    PageTarget { name: "Tasks", url_suffix: "/lightning/o/Task/home", suite: "Service" },
    PageTarget { name: "Events", url_suffix: "/lightning/o/Event/home", suite: "Service" },
    // Admin & Analytics
    PageTarget { name: "Reports", url_suffix: "/lightning/o/Report/home", suite: "Admin" },
    PageTarget { name: "Dashboards", url_suffix: "/lightning/o/Dashboard/home", suite: "Admin" },
    PageTarget { name: "Setup", url_suffix: "/lightning/setup/SetupOneHome/home", suite: "Admin" },
];

// ---------------------------------------------------------------------------
// Login detection
// ---------------------------------------------------------------------------

/// Check if the current URL indicates we're on a login page.
fn is_login_page(url: &str) -> bool {
    let dominated_by_login = url.contains("/login")
        || url.contains("Login&")
        || url.contains("login.salesforce.com")
        || url.contains("test.salesforce.com/login");
    // Lightning URLs that contain "/login" in a different context are OK
    let is_lightning = url.contains("/lightning/");
    dominated_by_login && !is_lightning
}

/// Verify the browser is authenticated. Returns current URL on success.
async fn assert_authenticated(driver: &dyn UtamDriver, context: &str) -> Result<String, String> {
    let url = driver.current_url().await.unwrap_or_default();
    if is_login_page(&url) {
        Err(format!("[{context}] On login page instead of app. URL: {url}"))
    } else if url.is_empty() {
        Err(format!("[{context}] Empty URL — browser may have crashed"))
    } else {
        Ok(url)
    }
}

// ---------------------------------------------------------------------------
// Allure reporting
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

    fn save_json_attachment(&self, data: &str, name: &str) -> Option<serde_json::Value> {
        let att_uuid = make_uuid(&format!("{}-{name}", self.test_uuid));
        let filename = format!("{att_uuid}-attachment.json");
        std::fs::write(allure_results_dir().join(&filename), data).ok()?;
        Some(serde_json::json!({
            "name": name, "source": filename, "type": "application/json"
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
         language=Rust\n\
         pages={}\n",
        PAGES.len()
    );
    let _ = std::fs::write(allure_results_dir().join("environment.properties"), content);
}

fn write_allure_categories() {
    let categories = serde_json::json!([
        { "name": "Auth failures", "messageRegex": ".*login.*|.*frontdoor.*|.*auth.*", "matchedStatuses": ["failed"] },
        { "name": "Lightning load failures", "messageRegex": ".*Lightning.*|.*timeout.*|.*load.*", "matchedStatuses": ["failed"] },
        { "name": "Element not found", "messageRegex": ".*not found.*|.*Unable to locate.*", "matchedStatuses": ["failed"] },
        { "name": "Infrastructure", "messageRegex": ".*WebDriver.*|.*connection.*", "matchedStatuses": ["broken"] }
    ]);
    let _ = std::fs::write(
        allure_results_dir().join("categories.json"),
        serde_json::to_string_pretty(&categories).unwrap_or_default(),
    );
}

// ---------------------------------------------------------------------------
// Test helpers
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

#[cfg(feature = "webdriver")]
async fn create_driver() -> Box<dyn UtamDriver> {
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

    Box::new(ThirtyfourDriver::new(driver))
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

/// Wait for Lightning to render, checking for key DOM elements.
/// Returns true if Lightning loaded, false on timeout.
async fn wait_for_lightning(driver: &dyn UtamDriver) -> bool {
    for attempt in 1..=20 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // First check: are we on the login page?
        let url = driver.current_url().await.unwrap_or_default();
        if is_login_page(&url) {
            eprintln!("  Attempt {attempt}/20 — stuck on login page: {url}");
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

/// Navigate to a URL and wait for Lightning to load.
/// Returns Ok(url) on success, Err(message) if auth fails or Lightning doesn't load.
async fn navigate_and_verify(
    driver: &dyn UtamDriver,
    url: &str,
    context: &str,
) -> Result<String, String> {
    driver.navigate(url).await.map_err(|e| format!("[{context}] Navigation failed: {e}"))?;

    // Quick check: did we get redirected to login?
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let current = driver.current_url().await.unwrap_or_default();
    if is_login_page(&current) {
        return Err(format!("[{context}] Redirected to login page: {current}"));
    }

    // Wait for Lightning to render
    if !wait_for_lightning(driver).await {
        let current = driver.current_url().await.unwrap_or_default();
        if is_login_page(&current) {
            return Err(format!("[{context}] Ended up on login page after waiting: {current}"));
        }
        // Lightning didn't fully render but we're not on login — continue anyway
        eprintln!(
            "[{context}] Lightning elements not detected, but not on login page. Continuing."
        );
    }

    let current = driver.current_url().await.unwrap_or_default();
    Ok(current)
}

// ---------------------------------------------------------------------------
// Per-page discovery
// ---------------------------------------------------------------------------

struct PageResult {
    page_name: String,
    suite: String,
    matched: usize,
    discovered: usize,
    status: String,
}

async fn visit_page(
    driver: &dyn UtamDriver,
    instance_url: &str,
    page: &PageTarget,
    registry: &PageObjectRegistry,
) -> PageResult {
    let test_name = format!("Discovery: {}", page.name);
    let mut allure = AllureReport::new(&test_name, page.suite);
    let page_url = format!("{}{}", instance_url, page.url_suffix);

    // Step 1: Navigate and verify auth
    allure.begin_step();
    eprintln!("[{}] Navigating to {}", page.name, page.url_suffix);

    let nav_result = navigate_and_verify(driver, &page_url, page.name).await;

    let mut nav_att = Vec::new();
    if let Ok(png) = driver.screenshot_png().await {
        if let Some(att) = allure.save_screenshot(&png, &format!("{}-loaded", page.name)) {
            nav_att.push(att);
        }
    }

    let current_url = match nav_result {
        Ok(url) => {
            allure.end_step("Navigate", "passed", nav_att);
            url
        }
        Err(msg) => {
            eprintln!("[{}] FAILED: {msg}", page.name);
            allure.end_step_failed("Navigate", &msg, nav_att);
            allure.finish("failed", Some(&msg));
            return PageResult {
                page_name: page.name.to_string(),
                suite: page.suite.to_string(),
                matched: 0,
                discovered: 0,
                status: "failed".to_string(),
            };
        }
    };

    eprintln!("[{}] Loaded: {current_url}", page.name);

    // Step 2: Discovery
    allure.begin_step();
    let report = match utam_runtime::discovery::discover(driver, registry).await {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("Discovery failed: {e}");
            eprintln!("[{}] {msg}", page.name);
            allure.end_step_failed("Discovery", &msg, vec![]);
            allure.finish("failed", Some(&msg));
            return PageResult {
                page_name: page.name.to_string(),
                suite: page.suite.to_string(),
                matched: 0,
                discovered: 0,
                status: "failed".to_string(),
            };
        }
    };

    let matched = report.matched.len();
    let discovered = report.discovered.len();
    eprintln!("[{}] {} matched, {} discovered", page.name, matched, discovered);
    for m in &report.matched {
        eprintln!("  + {} ({} methods)", m.name, m.method_count);
    }

    let mut disc_att = Vec::new();
    let report_json = serde_json::to_string_pretty(&report).unwrap_or_default();
    if let Some(att) =
        allure.save_json_attachment(&report_json, &format!("{}-discovery.json", page.name))
    {
        disc_att.push(att);
    }
    if let Ok(png) = driver.screenshot_png().await {
        if let Some(att) = allure.save_screenshot(&png, &format!("{}-after-discovery", page.name)) {
            disc_att.push(att);
        }
    }

    allure.end_step(
        &format!("Discovery ({matched} matched, {discovered} new)"),
        "passed",
        disc_att,
    );
    allure.finish("passed", None);

    PageResult {
        page_name: page.name.to_string(),
        suite: page.suite.to_string(),
        matched,
        discovered,
        status: "passed".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_salesforce_live() {
    let Some((instance_url, frontdoor_url)) = require_sf_credentials() else {
        eprintln!("SKIP: SF credentials not set");
        return;
    };

    write_allure_environment(&instance_url);
    write_allure_categories();

    let driver = create_driver().await;
    let registry = load_registry();

    // ── Phase 1: Authenticate via frontdoor ──────────────────────────
    eprintln!("\n=== Phase 1: Authentication ===");
    let mut auth_allure = AllureReport::new("Authentication", "Setup");
    auth_allure.begin_step();

    // Navigate through frontdoor (contains credentials — video not recording yet)
    driver.navigate(&frontdoor_url).await.expect("Failed to navigate to frontdoor");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    match assert_authenticated(driver.as_ref(), "frontdoor").await {
        Ok(url) => eprintln!("After frontdoor: {url}"),
        Err(msg) => {
            if let Ok(png) = driver.screenshot_png().await {
                auth_allure.save_screenshot(&png, "FAIL-auth");
            }
            auth_allure.end_step_failed("Frontdoor auth", &msg, vec![]);
            auth_allure.finish("failed", Some(&msg));
            panic!("{msg}");
        }
    }

    // Navigate to Lightning home to fully establish the session
    let home_url = format!("{instance_url}/lightning/page/home");
    match navigate_and_verify(driver.as_ref(), &home_url, "auth-home").await {
        Ok(url) => eprintln!("Lightning home: {url}"),
        Err(msg) => {
            if let Ok(png) = driver.screenshot_png().await {
                auth_allure.save_screenshot(&png, "FAIL-lightning");
            }
            auth_allure.end_step_failed("Lightning home", &msg, vec![]);
            auth_allure.finish("failed", Some(&msg));
            panic!("{msg}");
        }
    }

    let mut auth_att = Vec::new();
    if let Ok(png) = driver.screenshot_png().await {
        if let Some(att) = auth_allure.save_screenshot(&png, "authenticated") {
            auth_att.push(att);
        }
    }
    auth_allure.end_step("Frontdoor auth + Lightning", "passed", auth_att);
    auth_allure.finish("passed", None);

    // Signal to CI that auth is complete — safe to start video recording.
    // The frontdoor URL (with credentials) is no longer visible in the browser.
    let _ = std::fs::write(allure_results_dir().join("browser-ready"), "1");

    // ── Phase 2: Visit each page and run discovery ───────────────────
    eprintln!("\n=== Phase 2: Page Discovery ({} pages) ===", PAGES.len());

    let mut results = Vec::new();
    for page in PAGES {
        let result = visit_page(driver.as_ref(), &instance_url, page, &registry).await;

        // If we hit the login page, auth is gone — stop trying other pages
        if result.status == "failed" {
            let url = driver.current_url().await.unwrap_or_default();
            if is_login_page(&url) {
                eprintln!(
                    "[{}] Session expired or auth lost — aborting remaining pages",
                    page.name
                );
                results.push(result);
                break;
            }
        }

        results.push(result);
    }

    // ── Phase 3: Summary ─────────────────────────────────────────────
    eprintln!("\n=== Phase 3: Summary ===");
    let total_matched: usize = results.iter().map(|r| r.matched).sum();
    let total_discovered: usize = results.iter().map(|r| r.discovered).sum();
    let pages_passed = results.iter().filter(|r| r.status == "passed").count();
    let pages_total = results.len();

    eprintln!("Pages: {pages_passed}/{pages_total} passed");
    eprintln!("Total matched: {total_matched}");
    eprintln!("Total discovered: {total_discovered}");
    for r in &results {
        let icon = if r.status == "passed" { "+" } else { "!" };
        eprintln!(
            "  {icon} {} [{}] — {} matched, {} discovered",
            r.page_name, r.suite, r.matched, r.discovered
        );
    }

    driver.quit().await.expect("Failed to quit");

    assert!(pages_passed > 0, "No pages passed! All {pages_total} pages failed.");
    assert!(total_matched > 10, "Expected >10 total matched page objects, got {total_matched}");

    eprintln!("\n=== Complete ===");
}
