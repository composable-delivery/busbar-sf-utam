//! Salesforce live integration tests
//!
//! Runs against a real Salesforce org. Authenticates once via frontdoor.jsp,
//! clones session cookies into parallel browser sessions, and discovers page
//! objects across many Lightning pages simultaneously.
//!
//! Emits Allure-format JSON results (one per page) for rich reporting with
//! screenshots and video.
//!
//! Skipped locally (no CHROMEDRIVER_URL), panics in CI if credentials missing.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use utam_runtime::prelude::*;

// ---------------------------------------------------------------------------
// Page targets — grouped into parallel lanes
// ---------------------------------------------------------------------------

struct PageTarget {
    name: &'static str,
    url_suffix: &'static str,
}

const LANE_CORE: &[PageTarget] = &[
    PageTarget { name: "Home", url_suffix: "/lightning/page/home" },
    PageTarget { name: "Accounts", url_suffix: "/lightning/o/Account/list" },
    PageTarget { name: "Contacts", url_suffix: "/lightning/o/Contact/list" },
];

const LANE_SALES: &[PageTarget] = &[
    PageTarget { name: "Opportunities", url_suffix: "/lightning/o/Opportunity/list" },
    PageTarget { name: "Leads", url_suffix: "/lightning/o/Lead/list" },
    PageTarget { name: "Campaigns", url_suffix: "/lightning/o/Campaign/list" },
];

const LANE_SERVICE: &[PageTarget] = &[
    PageTarget { name: "Cases", url_suffix: "/lightning/o/Case/list" },
    PageTarget { name: "Tasks", url_suffix: "/lightning/o/Task/home" },
    PageTarget { name: "Events", url_suffix: "/lightning/o/Event/home" },
];

const LANE_ADMIN: &[PageTarget] = &[
    PageTarget { name: "Reports", url_suffix: "/lightning/o/Report/home" },
    PageTarget { name: "Dashboards", url_suffix: "/lightning/o/Dashboard/home" },
    PageTarget { name: "Setup", url_suffix: "/lightning/setup/SetupOneHome/home" },
];

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
        let path = allure_results_dir().join(&filename);
        std::fs::write(&path, png_data).ok()?;
        Some(serde_json::json!({
            "name": name,
            "source": filename,
            "type": "image/png"
        }))
    }

    fn save_json_attachment(&self, data: &str, name: &str) -> Option<serde_json::Value> {
        let att_uuid = make_uuid(&format!("{}-{name}", self.test_uuid));
        let filename = format!("{att_uuid}-attachment.json");
        let path = allure_results_dir().join(&filename);
        std::fs::write(&path, data).ok()?;
        Some(serde_json::json!({
            "name": name,
            "source": filename,
            "type": "application/json"
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
         parallel_lanes=4\n"
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
fn chrome_caps() -> thirtyfour::ChromeCapabilities {
    use thirtyfour::prelude::*;
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
    caps
}

#[cfg(feature = "webdriver")]
async fn create_driver() -> thirtyfour::prelude::WebDriver {
    use thirtyfour::prelude::*;
    let url = chromedriver_url();
    WebDriver::new(&url, chrome_caps())
        .await
        .unwrap_or_else(|e| panic!("WebDriver connection to {url} failed: {e}"))
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

async fn wait_for_lightning(driver: &dyn UtamDriver) -> bool {
    for attempt in 1..=20 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
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
            let url = driver.current_url().await.unwrap_or_default();
            eprintln!("  Attempt {attempt}/20 — waiting... URL: {url}");
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Phase 1: Authentication
// ---------------------------------------------------------------------------

#[cfg(feature = "webdriver")]
async fn authenticate(
    driver: &thirtyfour::prelude::WebDriver,
    frontdoor_url: &str,
    instance_url: &str,
) -> Vec<thirtyfour::Cookie> {
    let utam = ThirtyfourDriver::new(driver.clone());

    eprintln!("Navigating to frontdoor URL ({} chars)", frontdoor_url.len());
    utam.navigate(frontdoor_url).await.expect("Failed to navigate to frontdoor");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let url = utam.current_url().await.unwrap_or_default();
    eprintln!("After frontdoor: {url}");
    assert!(
        !url.contains("/login.") && !url.contains("Login&"),
        "Frontdoor auth failed — landed on login page: {url}"
    );

    // Navigate to Lightning home to ensure full session setup
    let home = format!("{instance_url}/lightning/page/home");
    utam.navigate(&home).await.expect("Failed to navigate home");
    assert!(wait_for_lightning(&utam).await, "Lightning did not load after auth");
    eprintln!("Authenticated and Lightning loaded");

    // Extract all cookies
    let cookies = driver.get_all_cookies().await.expect("Failed to get cookies");
    eprintln!("Extracted {} cookies", cookies.len());
    cookies
}

/// Create a new browser session and inject cookies for authentication.
#[cfg(feature = "webdriver")]
async fn create_authenticated_session(
    instance_url: &str,
    cookies: &[thirtyfour::Cookie],
) -> thirtyfour::prelude::WebDriver {
    let driver = create_driver().await;

    // Must be on the same domain before adding cookies
    driver.goto(instance_url).await.expect("Failed to navigate for cookie injection");
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Inject session cookies
    for cookie in cookies {
        let _ = driver.add_cookie(cookie.clone()).await;
    }
    eprintln!("Injected {} cookies into new session", cookies.len());

    driver
}

// ---------------------------------------------------------------------------
// Phase 2: Per-page discovery
// ---------------------------------------------------------------------------

struct PageResult {
    page_name: String,
    suite: String,
    matched: usize,
    discovered: usize,
    status: String,
}

#[cfg(feature = "webdriver")]
async fn visit_page(
    driver: &thirtyfour::prelude::WebDriver,
    instance_url: &str,
    page: &PageTarget,
    suite: &str,
    registry: &PageObjectRegistry,
) -> PageResult {
    let utam = ThirtyfourDriver::new(driver.clone());
    let test_name = format!("Discovery: {}", page.name);
    let mut allure = AllureReport::new(&test_name, suite);
    let page_url = format!("{}{}", instance_url, page.url_suffix);

    // Step: Navigate
    allure.begin_step();
    eprintln!("[{}] Navigating to {}", page.name, page_url);
    if let Err(e) = utam.navigate(&page_url).await {
        let msg = format!("Navigation failed: {e}");
        allure.end_step_failed("Navigate", &msg, vec![]);
        allure.finish("broken", Some(&msg));
        return PageResult {
            page_name: page.name.to_string(),
            suite: suite.to_string(),
            matched: 0,
            discovered: 0,
            status: "broken".to_string(),
        };
    }
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let current_url = utam.current_url().await.unwrap_or_default();
    eprintln!("[{}] Current URL: {current_url}", page.name);

    let mut nav_attachments = Vec::new();
    if let Ok(png) = utam.screenshot_png().await {
        if let Some(att) = allure.save_screenshot(&png, &format!("{}-loaded", page.name)) {
            nav_attachments.push(att);
        }
    }
    allure.end_step("Navigate", "passed", nav_attachments);

    // Step: Discovery
    allure.begin_step();
    let report = match utam_runtime::discovery::discover(&utam, registry).await {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("Discovery failed: {e}");
            allure.end_step_failed("Discovery", &msg, vec![]);
            allure.finish("failed", Some(&msg));
            return PageResult {
                page_name: page.name.to_string(),
                suite: suite.to_string(),
                matched: 0,
                discovered: 0,
                status: "failed".to_string(),
            };
        }
    };

    let matched = report.matched.len();
    let discovered = report.discovered.len();
    eprintln!("[{}] Found {} matched, {} discovered", page.name, matched, discovered);
    for m in &report.matched {
        eprintln!("  + {} ({} methods)", m.name, m.method_count);
    }

    let mut disc_attachments = Vec::new();
    let report_json = serde_json::to_string_pretty(&report).unwrap_or_default();
    if let Some(att) =
        allure.save_json_attachment(&report_json, &format!("{}-discovery.json", page.name))
    {
        disc_attachments.push(att);
    }
    if let Ok(png) = utam.screenshot_png().await {
        if let Some(att) = allure.save_screenshot(&png, &format!("{}-after-discovery", page.name)) {
            disc_attachments.push(att);
        }
    }

    allure.end_step(
        &format!("Discovery ({matched} matched, {discovered} new)"),
        "passed",
        disc_attachments,
    );
    allure.finish("passed", None);

    PageResult {
        page_name: page.name.to_string(),
        suite: suite.to_string(),
        matched,
        discovered,
        status: "passed".to_string(),
    }
}

/// Run a lane: visit all pages in sequence using one browser session.
#[cfg(feature = "webdriver")]
async fn run_lane(
    instance_url: &str,
    cookies: &[thirtyfour::Cookie],
    pages: &[PageTarget],
    suite: &str,
    registry: &PageObjectRegistry,
) -> Vec<PageResult> {
    let driver = create_authenticated_session(instance_url, cookies).await;
    let mut results = Vec::new();

    for page in pages {
        let result = visit_page(&driver, instance_url, page, suite, registry).await;
        results.push(result);
    }

    let _ = driver.quit().await;
    results
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

    // ── Phase 1: Authenticate ────────────────────────────────────────
    eprintln!("\n=== Phase 1: Authentication ===");
    let auth_driver = create_driver().await;
    let cookies = authenticate(&auth_driver, &frontdoor_url, &instance_url).await;

    // Auth Allure result
    let mut auth_allure = AllureReport::new("Authentication", "Setup");
    auth_allure.begin_step();
    let utam = ThirtyfourDriver::new(auth_driver.clone());
    let mut auth_att = Vec::new();
    if let Ok(png) = utam.screenshot_png().await {
        if let Some(att) = auth_allure.save_screenshot(&png, "authenticated") {
            auth_att.push(att);
        }
    }
    auth_allure.end_step(
        &format!("Frontdoor auth ({} cookies)", cookies.len()),
        "passed",
        auth_att,
    );
    auth_allure.finish("passed", None);

    let _ = auth_driver.quit().await;
    eprintln!("Auth session closed, launching parallel lanes\n");

    // ── Phase 2: Parallel discovery ──────────────────────────────────
    eprintln!("=== Phase 2: Parallel Discovery (4 lanes) ===");
    let registry = Arc::new(load_registry());

    let lanes: Vec<(&str, &[PageTarget])> = vec![
        ("Core Navigation", LANE_CORE),
        ("Sales", LANE_SALES),
        ("Service & Activities", LANE_SERVICE),
        ("Admin & Analytics", LANE_ADMIN),
    ];

    let mut handles = Vec::new();
    for (suite, pages) in &lanes {
        let instance_url = instance_url.clone();
        let cookies = cookies.clone();
        let registry = Arc::clone(&registry);
        let suite = suite.to_string();
        let pages: Vec<PageTarget> =
            pages.iter().map(|p| PageTarget { name: p.name, url_suffix: p.url_suffix }).collect();

        handles.push(tokio::spawn(async move {
            run_lane(&instance_url, &cookies, &pages, &suite, &registry).await
        }));
    }

    // Collect results
    let mut all_results: Vec<PageResult> = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(results) => all_results.extend(results),
            Err(e) => eprintln!("Lane panicked: {e}"),
        }
    }

    // ── Phase 3: Summary ─────────────────────────────────────────────
    eprintln!("\n=== Phase 3: Summary ===");
    let total_matched: usize = all_results.iter().map(|r| r.matched).sum();
    let total_discovered: usize = all_results.iter().map(|r| r.discovered).sum();
    let pages_passed = all_results.iter().filter(|r| r.status == "passed").count();
    let pages_total = all_results.len();

    eprintln!("Pages: {pages_passed}/{pages_total} passed");
    eprintln!("Total matched: {total_matched}");
    eprintln!("Total discovered: {total_discovered}");
    for r in &all_results {
        let icon = if r.status == "passed" { "+" } else { "!" };
        eprintln!(
            "  {icon} {} ({}) — {} matched, {} discovered",
            r.page_name, r.suite, r.matched, r.discovered
        );
    }

    // Assert at least some pages worked
    assert!(pages_passed > 0, "No pages passed! All {pages_total} pages failed.");
    assert!(total_matched > 10, "Expected >10 total matched page objects, got {total_matched}");

    eprintln!("\n=== All phases completed ===");
}
