//! Salesforce live integration tests
//!
//! Runs against a real Salesforce org using a single browser session.
//! The frontdoor token can only establish one session, so all test steps
//! share the same WebDriver instance.
//!
//! Skipped locally (no CHROMEDRIVER_URL), panics in CI if credentials missing.

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

    // If DISPLAY is set (Xvfb), run non-headless so ffmpeg can record the display.
    // Otherwise fall back to headless mode for local dev.
    let has_display = std::env::var("DISPLAY").is_ok();
    if !has_display {
        let _ = caps.set_headless();
    }
    let _ = caps.set_no_sandbox();
    let _ = caps.set_disable_gpu();
    let _ = caps.add_arg("--window-size=1920,1080");
    let _ = caps.add_arg("--disable-dev-shm-usage");
    if has_display {
        let _ = caps.add_arg("--start-maximized");
    }

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

/// Poll until we detect Lightning has rendered (check for key DOM elements).
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

/// All Salesforce live tests run in a single browser session.
///
/// The frontdoor.jsp token establishes one authenticated session. Creating
/// multiple WebDriver sessions with the same token causes redirect loops
/// (ec=301/302) because the token is consumed on first use.
#[tokio::test]
async fn test_salesforce_live() {
    let Some((instance_url, frontdoor_url)) = require_sf_credentials() else {
        eprintln!("SKIP: SF credentials not set");
        return;
    };

    let driver = create_driver().await.expect("Failed to create driver");

    // ── Step 1: Frontdoor authentication ─────────────────────────────
    eprintln!("\n=== Step 1: Frontdoor Authentication ===");
    eprintln!("Navigating to frontdoor URL ({} chars)", frontdoor_url.len());
    driver.navigate(&frontdoor_url).await.expect("Failed to navigate to frontdoor");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let url = driver.current_url().await.unwrap_or_default();
    eprintln!("After frontdoor: {url}");
    save_screenshot(driver.as_ref(), "01-after-frontdoor").await;

    // Verify we're not stuck on the login page
    assert!(
        !url.contains("/login.") && !url.contains("Login&"),
        "Frontdoor auth failed — landed on login page: {url}"
    );

    // ── Step 2: Navigate to Lightning home ───────────────────────────
    eprintln!("\n=== Step 2: Lightning Home ===");
    let home_url = format!("{instance_url}/lightning/page/home");
    eprintln!("Navigating to: {home_url}");
    driver.navigate(&home_url).await.expect("Failed to navigate to home");

    eprintln!("Waiting for Lightning to load...");
    assert!(
        wait_for_lightning_loaded(driver.as_ref()).await,
        "Lightning did not load within timeout. URL: {}",
        driver.current_url().await.unwrap_or_default()
    );

    let url = driver.current_url().await.unwrap_or_default();
    eprintln!("Lightning home: {url}");
    save_screenshot(driver.as_ref(), "02-lightning-home").await;

    assert!(
        url.contains("lightning") || url.contains("force.com"),
        "Should be on a Lightning page, got: {url}"
    );

    // ── Step 3: Page object discovery ────────────────────────────────
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
    save_screenshot(driver.as_ref(), "03-after-discovery").await;

    let report_json = serde_json::to_string_pretty(&report).unwrap_or_default();
    let report_path = artifacts_dir().join("discovery-report.json");
    let _ = std::fs::write(&report_path, &report_json);
    eprintln!("Discovery report saved to {}", report_path.display());

    let total = report.matched.len() + report.discovered.len();
    assert!(total > 5, "Expected >5 components on a Lightning page, got {total}");

    // ── Step 4: Header page object ───────────────────────────────────
    eprintln!("\n=== Step 4: Header Page Object ===");
    let header_matches = registry.search("global/header");
    assert!(!header_matches.is_empty(), "global/header not found in registry");

    let header_ast = registry.get(&header_matches[0]).expect("Failed to get header AST");
    eprintln!("Header: {}", header_matches[0]);
    eprintln!("  Root selector: {:?}", header_ast.selector.as_ref().map(|s| &s.css));
    eprintln!("  Methods: {:?}", header_ast.methods.iter().map(|m| &m.name).collect::<Vec<_>>());

    // Verify the header root element exists on the current page.
    // We can't use DynamicPageObject::load() because it takes ownership
    // of the driver, and we only have one authenticated session.
    let root_css = header_ast
        .selector
        .as_ref()
        .and_then(|s| s.css.as_ref())
        .expect("Header should have a root CSS selector")
        .clone();
    eprintln!("Looking for header root: {root_css}");
    let selector = Selector::Css(root_css.clone());
    let header_el = driver.find_element(&selector).await;
    match header_el {
        Ok(_) => {
            eprintln!("Header element found!");
            save_screenshot(driver.as_ref(), "04-header-found").await;
        }
        Err(e) => {
            save_screenshot(driver.as_ref(), "04-header-not-found").await;
            panic!("Header root element {root_css} not found on Lightning page: {e}");
        }
    }

    // ── Step 5: Navigate to Accounts and Contacts ────────────────────
    eprintln!("\n=== Step 5: Navigate Pages ===");
    let accounts_url = format!("{instance_url}/lightning/o/Account/list");
    eprintln!("Navigating to Accounts: {accounts_url}");
    driver.navigate(&accounts_url).await.expect("Accounts navigate failed");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    save_screenshot(driver.as_ref(), "05a-accounts").await;
    let url = driver.current_url().await.unwrap_or_default();
    eprintln!("Accounts: {url}");

    let contacts_url = format!("{instance_url}/lightning/o/Contact/list");
    eprintln!("Navigating to Contacts: {contacts_url}");
    driver.navigate(&contacts_url).await.expect("Contacts navigate failed");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    save_screenshot(driver.as_ref(), "05b-contacts").await;
    let url = driver.current_url().await.unwrap_or_default();
    eprintln!("Contacts: {url}");

    driver.quit().await.expect("Failed to quit");
    eprintln!("\n=== All steps completed ===");
}
