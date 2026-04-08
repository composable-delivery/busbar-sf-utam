//! Salesforce live integration tests
//!
//! These tests run against a real Salesforce org using credentials from
//! the `SF_AUTH_URL` environment variable. They are skipped when the
//! variable is not set (local dev), and run in CI via a GitHub Actions
//! workflow with a `salesforce` environment.
//!
//! The tests use the thirtyfour (WebDriver) adapter by default,
//! connecting to chromedriver on localhost:9515.

use std::collections::HashMap;
use std::path::PathBuf;

use utam_runtime::element::ElementRuntime;
use utam_runtime::page_object::PageObjectRuntime;
use utam_runtime::prelude::*;

/// Check if Salesforce credentials are available; skip test if not.
fn require_sf_credentials() -> Option<(String, String)> {
    let auth_url = std::env::var("SF_INSTANCE_URL").ok()?;
    let frontdoor = std::env::var("SF_FRONTDOOR_URL").ok()?;
    Some((auth_url, frontdoor))
}

/// Get the chromedriver URL (default: localhost:9515)
fn chromedriver_url() -> String {
    std::env::var("CHROMEDRIVER_URL").unwrap_or_else(|_| "http://localhost:9515".to_string())
}

/// Create a WebDriver connected to chromedriver
#[cfg(feature = "webdriver")]
async fn create_driver() -> RuntimeResult<Box<dyn UtamDriver>> {
    use thirtyfour::prelude::*;

    let mut caps = DesiredCapabilities::chrome();
    let _ = caps.set_headless();
    let _ = caps.set_no_sandbox();
    let _ = caps.set_disable_gpu();
    // Larger window for Salesforce's responsive layout
    let _ = caps.add_arg("--window-size=1920,1080");
    let _ = caps.add_arg("--disable-dev-shm-usage");

    let driver = WebDriver::new(&chromedriver_url(), caps).await.map_err(|e| {
        RuntimeError::UnsupportedAction {
            action: "create_driver".into(),
            element_type: format!("WebDriver connection failed: {e}"),
        }
    })?;

    Ok(Box::new(ThirtyfourDriver::new(driver)))
}

/// Load the Salesforce page object registry
fn load_registry() -> PageObjectRegistry {
    let mut registry = PageObjectRegistry::new();
    let sf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../salesforce-pageobjects");
    if sf_path.exists() {
        registry.add_search_path(sf_path);
        let _ = registry.scan();
    }
    registry
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Test: Navigate to the org and verify the page loads
#[tokio::test]
async fn test_sf_frontdoor_navigation() {
    let Some((instance_url, frontdoor_url)) = require_sf_credentials() else {
        eprintln!("SKIP: SF_INSTANCE_URL / SF_FRONTDOOR_URL not set");
        return;
    };

    let driver = create_driver().await.expect("Failed to create driver");
    driver.navigate(&frontdoor_url).await.expect("Failed to navigate to frontdoor");

    // Wait for the page to settle — Salesforce redirects after frontdoor
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let url = driver.current_url().await.expect("Failed to get URL");
    eprintln!("Current URL after frontdoor: {url}");

    // Should have been redirected away from the frontdoor URL
    assert!(
        !url.contains("frontdoor"),
        "Should have been redirected past frontdoor, still at: {url}"
    );

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
    driver.navigate(&frontdoor_url).await.expect("Navigate failed");
    tokio::time::sleep(std::time::Duration::from_secs(8)).await;

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
    assert!(
        report.matched.len() + report.discovered.len() > 0,
        "Should discover at least one component"
    );

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
    driver.navigate(&frontdoor_url).await.expect("Navigate failed");
    tokio::time::sleep(std::time::Duration::from_secs(8)).await;

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
    eprintln!("  Elements: {}", header_ast.elements.len());
    for e in &header_ast.elements {
        eprintln!("    - {} ({:?})", e.name, e.element_type);
    }

    // Try to load the page object against the live page
    match DynamicPageObject::load(driver, header_ast).await {
        Ok(page) => {
            eprintln!("Header page object loaded successfully!");
            let methods = page.method_signatures();
            eprintln!("  Live methods: {:?}", methods.iter().map(|m| &m.name).collect::<Vec<_>>());
            let elements = page.element_names();
            eprintln!("  Live elements: {:?}", elements);
        }
        Err(e) => {
            eprintln!("Header page object failed to load (may not be on this page): {e}");
            // Not a test failure — the header might not be present on all pages
        }
    }
}

/// Test: Take a screenshot of the authenticated page
#[tokio::test]
async fn test_sf_screenshot() {
    let Some((_, frontdoor_url)) = require_sf_credentials() else {
        eprintln!("SKIP: SF credentials not set");
        return;
    };

    let driver = create_driver().await.expect("Failed to create driver");
    driver.navigate(&frontdoor_url).await.expect("Navigate failed");
    tokio::time::sleep(std::time::Duration::from_secs(8)).await;

    let png = driver.screenshot_png().await.expect("Screenshot failed");
    assert!(!png.is_empty(), "Screenshot should not be empty");
    eprintln!("Screenshot captured: {} bytes", png.len());

    // Save to disk if GITHUB_STEP_SUMMARY is set (CI)
    if std::env::var("GITHUB_STEP_SUMMARY").is_ok() {
        let path = "/tmp/sf-screenshot.png";
        std::fs::write(path, &png).expect("Failed to write screenshot");
        eprintln!("Screenshot saved to {path}");
    }

    driver.quit().await.expect("Failed to quit");
}
