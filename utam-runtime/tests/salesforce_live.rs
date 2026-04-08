//! Salesforce live integration tests
//!
//! These tests run against a real Salesforce org. They are skipped when
//! `SF_INSTANCE_URL` and `SF_FRONTDOOR_URL` are not set.
//!
//! Tests run sequentially (`--test-threads=1`) so they share a single
//! browser session via the frontdoor URL. Each test captures screenshots
//! at every meaningful stage.

use std::path::PathBuf;

use utam_runtime::prelude::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn require_sf_credentials() -> Option<(String, String)> {
    let instance = std::env::var("SF_INSTANCE_URL").ok()?;
    let frontdoor = std::env::var("SF_FRONTDOOR_URL").ok()?;
    if instance.is_empty() || frontdoor.is_empty() {
        return None;
    }
    Some((instance, frontdoor))
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

/// Save a screenshot with a descriptive name.
async fn save_screenshot(driver: &dyn UtamDriver, name: &str) {
    match driver.screenshot_png().await {
        Ok(png) => {
            let path = artifacts_dir().join(format!("{name}.png"));
            match std::fs::write(&path, &png) {
                Ok(_) => eprintln!("  screenshot: {} ({} bytes)", path.display(), png.len()),
                Err(e) => eprintln!("  screenshot write failed: {e}"),
            }
        }
        Err(e) => eprintln!("  screenshot failed: {e}"),
    }
}

#[cfg(feature = "webdriver")]
async fn create_driver() -> RuntimeResult<Box<dyn UtamDriver>> {
    use thirtyfour::prelude::*;

    let url = chromedriver_url();
    eprintln!("Connecting to chromedriver at {url}");

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

    eprintln!("WebDriver session created");
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

async fn navigate_to_org(driver: &dyn UtamDriver, frontdoor_url: &str) -> RuntimeResult<String> {
    eprintln!("Navigating to frontdoor URL ({} chars)", frontdoor_url.len());
    eprintln!("  Host: {}", frontdoor_url.split("/secur/").next().unwrap_or("unknown"));
    driver.navigate(frontdoor_url).await?;

    // Salesforce redirects through frontdoor; give it time to settle
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    let url = driver.current_url().await?;
    eprintln!("Current URL after navigation: {url}");
    Ok(url)
}

/// Assert that we are NOT on the login page (auth succeeded).
async fn assert_authenticated(driver: &dyn UtamDriver, context: &str) {
    let url = driver.current_url().await.unwrap_or_default();
    let is_login_page = url.contains("/login") && !url.contains("/lightning");

    if is_login_page {
        // Take a screenshot before panicking
        save_screenshot(driver, &format!("FAIL-auth-{context}")).await;

        // Check page content for more context
        let title = driver.title().await.unwrap_or_default();
        panic!(
            "Authentication failed in {context}! \
             Landed on login page instead of the app.\n\
             URL: {url}\n\
             Title: {title}\n\
             This usually means the frontdoor.jsp token was expired or \
             the URL host was incorrect."
        );
    }
    eprintln!("  Auth check passed: {url}");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Test 1: Navigate to the org via frontdoor, verify we land somewhere,
/// then navigate to the main app home page.
#[tokio::test]
async fn test_01_frontdoor_and_home() {
    let Some((_instance_url, frontdoor_url)) = require_sf_credentials() else {
        eprintln!("SKIP: SF credentials not set");
        return;
    };

    let driver = create_driver().await.expect("Failed to create driver");

    // Navigate through frontdoor
    let url = navigate_to_org(driver.as_ref(), &frontdoor_url).await.expect("Failed to navigate");
    save_screenshot(driver.as_ref(), "01a-after-frontdoor").await;
    assert_authenticated(driver.as_ref(), "test_01_frontdoor").await;

    eprintln!("Landed at: {url}");

    // If we landed on Setup, navigate to the app home
    if url.contains("/setup/") || url.contains("SetupOneHome") {
        eprintln!("Landed on Setup page, navigating to app home...");
        let instance = std::env::var("SF_INSTANCE_URL").unwrap_or_default();
        if !instance.is_empty() {
            driver
                .navigate(&format!("{instance}/lightning/page/home"))
                .await
                .expect("Navigate to home failed");
            tokio::time::sleep(std::time::Duration::from_secs(8)).await;
            save_screenshot(driver.as_ref(), "01b-app-home").await;
            let home_url = driver.current_url().await.unwrap_or_default();
            eprintln!("Home URL: {home_url}");
        }
    }

    driver.quit().await.expect("Failed to quit");
}

/// Test 2: Run page object discovery on the app home page.
#[tokio::test]
async fn test_02_discovery() {
    let Some((_instance, frontdoor_url)) = require_sf_credentials() else {
        eprintln!("SKIP: SF credentials not set");
        return;
    };

    let driver = create_driver().await.expect("Failed to create driver");
    navigate_to_org(driver.as_ref(), &frontdoor_url).await.expect("Navigate failed");
    assert_authenticated(driver.as_ref(), "test_02_discovery").await;

    // Try to get to the app home for better discovery results
    let instance = std::env::var("SF_INSTANCE_URL").unwrap_or_default();
    if !instance.is_empty() {
        let _ = driver.navigate(&format!("{instance}/lightning/page/home")).await;
        tokio::time::sleep(std::time::Duration::from_secs(8)).await;
    }
    save_screenshot(driver.as_ref(), "02a-before-discovery").await;

    let registry = load_registry();
    let report = utam_runtime::discovery::discover(driver.as_ref(), &registry)
        .await
        .expect("Discovery failed");

    eprintln!("=== Discovery Report ===");
    eprintln!("URL: {}", report.url);
    eprintln!("Known page objects matched: {}", report.matched.len());
    for m in &report.matched {
        eprintln!("  + {} ({} methods, {} elements)", m.name, m.method_count, m.element_count);
    }
    eprintln!("Unknown components discovered: {}", report.discovered.len());
    for d in report.discovered.iter().take(15) {
        eprintln!("  ? <{}> shadow={} children={}", d.tag_name, d.has_shadow, d.children.len());
    }
    eprintln!("========================");

    save_screenshot(driver.as_ref(), "02b-after-discovery").await;

    // Save the discovery report as JSON
    let report_json = serde_json::to_string_pretty(&report).unwrap_or_default();
    let report_path = artifacts_dir().join("discovery-report.json");
    let _ = std::fs::write(&report_path, &report_json);
    eprintln!("Discovery report saved to {}", report_path.display());

    driver.quit().await.expect("Failed to quit");
}

/// Test 3: Load the global header page object against the live org.
#[tokio::test]
async fn test_03_header_page_object() {
    let Some((_instance, frontdoor_url)) = require_sf_credentials() else {
        eprintln!("SKIP: SF credentials not set");
        return;
    };

    let driver = create_driver().await.expect("Failed to create driver");
    navigate_to_org(driver.as_ref(), &frontdoor_url).await.expect("Navigate failed");
    assert_authenticated(driver.as_ref(), "test_03_header").await;

    // Navigate to app home where the header is present
    let instance = std::env::var("SF_INSTANCE_URL").unwrap_or_default();
    if !instance.is_empty() {
        let _ = driver.navigate(&format!("{instance}/lightning/page/home")).await;
        tokio::time::sleep(std::time::Duration::from_secs(8)).await;
    }
    save_screenshot(driver.as_ref(), "03a-before-header-load").await;

    let registry = load_registry();
    let header_matches = registry.search("global/header");
    if header_matches.is_empty() {
        eprintln!("SKIP: global/header not found in registry");
        driver.quit().await.ok();
        return;
    }

    let header_ast = registry.get(&header_matches[0]).expect("Failed to get header AST");
    eprintln!("Header: {}", header_matches[0]);
    eprintln!("  Root selector: {:?}", header_ast.selector.as_ref().map(|s| &s.css));
    eprintln!("  Methods: {:?}", header_ast.methods.iter().map(|m| &m.name).collect::<Vec<_>>());

    // Load the page object — need a new driver since load() takes ownership
    let driver2 = create_driver().await.expect("Failed to create second driver");
    navigate_to_org(driver2.as_ref(), &frontdoor_url).await.expect("Navigate failed");
    if !instance.is_empty() {
        let _ = driver2.navigate(&format!("{instance}/lightning/page/home")).await;
        tokio::time::sleep(std::time::Duration::from_secs(8)).await;
    }

    match DynamicPageObject::load(driver2, header_ast).await {
        Ok(page) => {
            eprintln!("Header loaded successfully!");
            let methods = page.method_signatures();
            eprintln!("  Methods: {:?}", methods.iter().map(|m| &m.name).collect::<Vec<_>>());
            let elements = page.element_names();
            eprintln!("  Elements: {:?}", elements);
        }
        Err(e) => {
            eprintln!("Header did not load: {e}");
        }
    }

    save_screenshot(driver.as_ref(), "03b-after-header-load").await;
    driver.quit().await.expect("Failed to quit");
}

/// Test 4: Navigate to a record page and screenshot it.
#[tokio::test]
async fn test_04_navigate_pages() {
    let Some((_instance, frontdoor_url)) = require_sf_credentials() else {
        eprintln!("SKIP: SF credentials not set");
        return;
    };

    let driver = create_driver().await.expect("Failed to create driver");
    let instance = std::env::var("SF_INSTANCE_URL").unwrap_or_default();

    // Frontdoor
    navigate_to_org(driver.as_ref(), &frontdoor_url).await.expect("Navigate failed");
    save_screenshot(driver.as_ref(), "04a-frontdoor").await;
    assert_authenticated(driver.as_ref(), "test_04_navigate").await;

    if instance.is_empty() {
        driver.quit().await.expect("Failed to quit");
        return;
    }

    // App home
    driver
        .navigate(&format!("{instance}/lightning/page/home"))
        .await
        .expect("Home navigate failed");
    tokio::time::sleep(std::time::Duration::from_secs(8)).await;
    save_screenshot(driver.as_ref(), "04b-app-home").await;
    eprintln!("App home: {}", driver.current_url().await.unwrap_or_default());

    // Accounts list
    driver
        .navigate(&format!("{instance}/lightning/o/Account/list"))
        .await
        .expect("Accounts navigate failed");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    save_screenshot(driver.as_ref(), "04c-accounts-list").await;
    eprintln!("Accounts: {}", driver.current_url().await.unwrap_or_default());

    // Contacts list
    driver
        .navigate(&format!("{instance}/lightning/o/Contact/list"))
        .await
        .expect("Contacts navigate failed");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    save_screenshot(driver.as_ref(), "04d-contacts-list").await;
    eprintln!("Contacts: {}", driver.current_url().await.unwrap_or_default());

    driver.quit().await.expect("Failed to quit");
}
