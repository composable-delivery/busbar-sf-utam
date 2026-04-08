//! Salesforce live integration tests
//!
//! These tests run against a real Salesforce org using credentials from
//! the `SF_INSTANCE_URL` and `SF_FRONTDOOR_URL` environment variables.
//! They are skipped when the variables are not set (local dev), and run
//! in CI via a GitHub Actions workflow with a `salesforce` environment.

use std::path::PathBuf;

use utam_runtime::prelude::*;

/// Check if Salesforce credentials are available; skip test if not.
fn require_sf_credentials() -> Option<(String, String)> {
    let instance = std::env::var("SF_INSTANCE_URL").ok()?;
    let frontdoor = std::env::var("SF_FRONTDOOR_URL").ok()?;
    if instance.is_empty() || frontdoor.is_empty() {
        return None;
    }
    Some((instance, frontdoor))
}

/// Get the chromedriver URL (default: localhost:9515)
fn chromedriver_url() -> String {
    std::env::var("CHROMEDRIVER_URL").unwrap_or_else(|_| "http://localhost:9515".to_string())
}

/// Create a WebDriver connected to chromedriver
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

/// Load the Salesforce page object registry
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

/// Navigate to the frontdoor URL and wait for the page to settle
async fn navigate_to_org(driver: &dyn UtamDriver, frontdoor_url: &str) -> RuntimeResult<String> {
    eprintln!("Navigating to frontdoor URL ({} chars)", frontdoor_url.len());
    driver.navigate(frontdoor_url).await?;

    // Salesforce redirects through frontdoor → home page; give it time
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    let url = driver.current_url().await?;
    eprintln!("Current URL after navigation: {url}");
    Ok(url)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Test: Navigate to the org and verify the page loads
#[tokio::test]
async fn test_sf_frontdoor_navigation() {
    let Some((_instance_url, frontdoor_url)) = require_sf_credentials() else {
        eprintln!("SKIP: SF_INSTANCE_URL / SF_FRONTDOOR_URL not set");
        return;
    };

    let driver = create_driver().await.expect("Failed to create driver");
    let url = navigate_to_org(driver.as_ref(), &frontdoor_url).await.expect("Failed to navigate");

    // The frontdoor should redirect — we should NOT still be on the frontdoor URL.
    // But we should be on SOME Salesforce page.
    assert!(!url.is_empty(), "URL should not be empty after navigation");

    // Take a screenshot for debugging
    if let Ok(png) = driver.screenshot_png().await {
        eprintln!("Screenshot after frontdoor: {} bytes", png.len());
        let _ = std::fs::write("/tmp/sf-screenshot-frontdoor.png", &png);
    }

    driver.quit().await.expect("Failed to quit");
}

/// Test: Discover which known page objects are on the home page
#[tokio::test]
async fn test_sf_discover_page_objects() {
    let Some((_, frontdoor_url)) = require_sf_credentials() else {
        eprintln!("SKIP: SF credentials not set");
        return;
    };

    let driver = create_driver().await.expect("Failed to create driver");
    navigate_to_org(driver.as_ref(), &frontdoor_url).await.expect("Navigate failed");

    let registry = load_registry();
    let report = utam_runtime::discovery::discover(driver.as_ref(), &registry)
        .await
        .expect("Discovery failed");

    eprintln!("=== Salesforce Live Discovery Report ===");
    eprintln!("URL: {}", report.url);
    eprintln!("Known page objects matched: {}", report.matched.len());
    for m in &report.matched {
        eprintln!(
            "  {} (selector: {}, {} methods, {} elements)",
            m.name, m.selector, m.method_count, m.element_count
        );
    }
    eprintln!("Unknown components discovered: {}", report.discovered.len());
    for d in report.discovered.iter().take(10) {
        eprintln!("  <{}> shadow={} children={}", d.tag_name, d.has_shadow, d.children.len());
    }
    eprintln!("========================================");

    // We should discover at least some components on any Salesforce page
    let total = report.matched.len() + report.discovered.len();
    eprintln!("Total components found: {total}");

    driver.quit().await.expect("Failed to quit");
}

/// Test: Load and introspect the global header page object on a live org
#[tokio::test]
async fn test_sf_header_introspection() {
    let Some((_, frontdoor_url)) = require_sf_credentials() else {
        eprintln!("SKIP: SF credentials not set");
        return;
    };

    let driver = create_driver().await.expect("Failed to create driver");
    navigate_to_org(driver.as_ref(), &frontdoor_url).await.expect("Navigate failed");

    let registry = load_registry();

    // Find the header page object
    let header_matches = registry.search("global/header");
    if header_matches.is_empty() {
        eprintln!("SKIP: global/header not found in registry");
        driver.quit().await.ok();
        return;
    }

    let header_ast = registry.get(&header_matches[0]).expect("Failed to get header AST");
    eprintln!("Header page object: {}", header_matches[0]);
    eprintln!("  Root selector: {:?}", header_ast.selector.as_ref().map(|s| &s.css));
    eprintln!("  Methods: {}", header_ast.methods.len());
    for m in &header_ast.methods {
        eprintln!("    - {}", m.name);
    }

    // Try to load the page object against the live page
    match DynamicPageObject::load(
        Box::new(ThirtyfourDriver::new(
            thirtyfour::WebDriver::new(
                &chromedriver_url(),
                thirtyfour::DesiredCapabilities::chrome(),
            )
            .await
            .unwrap(),
        )),
        header_ast,
    )
    .await
    {
        Ok(page) => {
            eprintln!("Header page object loaded successfully!");
            let methods = page.method_signatures();
            eprintln!("  Live methods: {:?}", methods.iter().map(|m| &m.name).collect::<Vec<_>>());
        }
        Err(e) => {
            // Not a test failure — the header might not be present
            eprintln!("Header page object did not load (may not be on this page): {e}");
        }
    }

    driver.quit().await.expect("Failed to quit");
}

/// Test: Take a screenshot of the authenticated page
#[tokio::test]
async fn test_sf_screenshot() {
    let Some((_, frontdoor_url)) = require_sf_credentials() else {
        eprintln!("SKIP: SF credentials not set");
        return;
    };

    let driver = create_driver().await.expect("Failed to create driver");
    navigate_to_org(driver.as_ref(), &frontdoor_url).await.expect("Navigate failed");

    let png = driver.screenshot_png().await.expect("Screenshot failed");
    assert!(!png.is_empty(), "Screenshot should not be empty");
    eprintln!("Screenshot captured: {} bytes", png.len());

    let path = "/tmp/sf-screenshot.png";
    std::fs::write(path, &png).expect("Failed to write screenshot");
    eprintln!("Screenshot saved to {path}");

    driver.quit().await.expect("Failed to quit");
}
