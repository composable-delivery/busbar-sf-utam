//! Salesforce live integration tests
//!
//! Run against a real Salesforce org. Skipped locally (no CHROMEDRIVER_URL),
//! panics in CI if credentials are missing.

use std::path::PathBuf;

use utam_runtime::prelude::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns credentials or panics in CI. Only returns None for local dev.
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

/// Navigate to the org and wait for Lightning to fully load.
/// Panics if we end up on the login page or Lightning never renders.
async fn navigate_and_wait_for_lightning(
    driver: &dyn UtamDriver,
    frontdoor_url: &str,
    instance_url: &str,
) {
    eprintln!("Navigating to frontdoor URL ({} chars)", frontdoor_url.len());
    driver.navigate(frontdoor_url).await.expect("Failed to navigate to frontdoor");

    // Wait for the initial frontdoor redirect
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let url = driver.current_url().await.unwrap_or_default();
    eprintln!("After frontdoor: {url}");

    // Check we're not on the login page
    if url.contains("/login.") || url.contains("Login&") {
        save_screenshot(driver, "FAIL-on-login-page").await;
        panic!("Frontdoor auth failed — landed on login page: {url}");
    }

    // Navigate to Lightning app home explicitly
    let home_url = format!("{instance_url}/lightning/page/home");
    eprintln!("Navigating to Lightning home: {home_url}");
    driver.navigate(&home_url).await.expect("Failed to navigate to home");

    // Wait for Lightning to actually render — poll for the oneHeader element
    eprintln!("Waiting for Lightning to load...");
    let loaded = wait_for_lightning_loaded(driver).await;
    let final_url = driver.current_url().await.unwrap_or_default();
    eprintln!("Final URL: {final_url}");

    if !loaded {
        save_screenshot(driver, "FAIL-lightning-not-loaded").await;
        panic!(
            "Lightning did not load within timeout.\n\
             Final URL: {final_url}\n\
             Expected a fully rendered Lightning page."
        );
    }

    eprintln!("Lightning loaded successfully");
}

/// Poll until we detect Lightning has rendered (check for key DOM elements).
async fn wait_for_lightning_loaded(driver: &dyn UtamDriver) -> bool {
    for attempt in 1..=30 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // Check if Lightning runtime has rendered by looking for key elements
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
// Tests
// ---------------------------------------------------------------------------

/// Test 1: Authenticate via frontdoor, navigate to Lightning, verify it loads.
#[tokio::test]
async fn test_01_frontdoor_and_lightning() {
    let Some((instance_url, frontdoor_url)) = require_sf_credentials() else {
        eprintln!("SKIP: SF credentials not set");
        return;
    };

    let driver = create_driver().await.expect("Failed to create driver");
    navigate_and_wait_for_lightning(driver.as_ref(), &frontdoor_url, &instance_url).await;
    save_screenshot(driver.as_ref(), "01-lightning-home").await;

    let url = driver.current_url().await.unwrap_or_default();
    assert!(
        url.contains("lightning") || url.contains("force.com"),
        "Should be on a Lightning page, got: {url}"
    );

    driver.quit().await.expect("Failed to quit");
}

/// Test 2: Run page object discovery on a fully loaded Lightning page.
#[tokio::test]
async fn test_02_discovery() {
    let Some((instance_url, frontdoor_url)) = require_sf_credentials() else {
        eprintln!("SKIP: SF credentials not set");
        return;
    };

    let driver = create_driver().await.expect("Failed to create driver");
    navigate_and_wait_for_lightning(driver.as_ref(), &frontdoor_url, &instance_url).await;
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

    // Save the discovery report as JSON artifact
    let report_json = serde_json::to_string_pretty(&report).unwrap_or_default();
    let report_path = artifacts_dir().join("discovery-report.json");
    let _ = std::fs::write(&report_path, &report_json);

    // On a fully loaded Lightning page, we MUST find components
    let total = report.matched.len() + report.discovered.len();
    assert!(total > 5, "Expected to discover many components on a Lightning page, got {total}");

    driver.quit().await.expect("Failed to quit");
}

/// Test 3: Load the global header page object against a live Lightning page.
#[tokio::test]
async fn test_03_header_page_object() {
    let Some((instance_url, frontdoor_url)) = require_sf_credentials() else {
        eprintln!("SKIP: SF credentials not set");
        return;
    };

    let registry = load_registry();
    let header_matches = registry.search("global/header");
    if header_matches.is_empty() {
        panic!("global/header not found in registry — check salesforce-pageobjects/");
    }

    let header_ast = registry.get(&header_matches[0]).expect("Failed to get header AST");
    eprintln!("Header: {}", header_matches[0]);
    eprintln!("  Root selector: {:?}", header_ast.selector.as_ref().map(|s| &s.css));
    eprintln!("  Methods: {:?}", header_ast.methods.iter().map(|m| &m.name).collect::<Vec<_>>());

    // DynamicPageObject::load takes ownership of the driver
    let driver = create_driver().await.expect("Failed to create driver");
    navigate_and_wait_for_lightning(driver.as_ref(), &frontdoor_url, &instance_url).await;
    save_screenshot(driver.as_ref(), "03a-before-header-load").await;

    // Load the header page object
    let page = DynamicPageObject::load(driver, header_ast)
        .await
        .expect("Header page object should load on a Lightning page with .oneHeader");

    eprintln!("Header loaded successfully!");
    let methods = page.method_signatures();
    eprintln!("  Methods: {:?}", methods.iter().map(|m| &m.name).collect::<Vec<_>>());
    let elements = page.element_names();
    eprintln!("  Elements: {:?}", elements);

    assert!(!methods.is_empty(), "Header should have methods");
    assert!(!elements.is_empty(), "Header should have elements");
}

/// Test 4: Navigate across multiple Lightning pages, screenshot each.
#[tokio::test]
async fn test_04_navigate_pages() {
    let Some((instance_url, frontdoor_url)) = require_sf_credentials() else {
        eprintln!("SKIP: SF credentials not set");
        return;
    };

    let driver = create_driver().await.expect("Failed to create driver");
    navigate_and_wait_for_lightning(driver.as_ref(), &frontdoor_url, &instance_url).await;
    save_screenshot(driver.as_ref(), "04a-lightning-home").await;

    // Navigate to Accounts
    let accounts_url = format!("{instance_url}/lightning/o/Account/list");
    eprintln!("Navigating to Accounts: {accounts_url}");
    driver.navigate(&accounts_url).await.expect("Accounts navigate failed");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    save_screenshot(driver.as_ref(), "04b-accounts-list").await;
    let url = driver.current_url().await.unwrap_or_default();
    eprintln!("Accounts URL: {url}");
    assert!(
        url.contains("Account") || url.contains("lightning"),
        "Should be on Accounts page, got: {url}"
    );

    // Navigate to Contacts
    let contacts_url = format!("{instance_url}/lightning/o/Contact/list");
    eprintln!("Navigating to Contacts: {contacts_url}");
    driver.navigate(&contacts_url).await.expect("Contacts navigate failed");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    save_screenshot(driver.as_ref(), "04c-contacts-list").await;

    driver.quit().await.expect("Failed to quit");
}
