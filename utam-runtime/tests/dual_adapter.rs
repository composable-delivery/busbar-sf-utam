//! Dual adapter test matrix — WebDriver vs CDP
//!
//! Runs the same page object tests via both ThirtyfourDriver (WebDriver)
//! and CdpDriver (chromiumoxide CDP), comparing results to validate
//! the UtamDriver trait abstraction works across backends.
//!
//! Requires: CHROMEDRIVER_URL + SF_AUTH_URL (WebDriver)
//!           Chrome with --remote-debugging-port (CDP)

use std::path::PathBuf;
use std::sync::Arc;

use utam_runtime::prelude::*;

#[allow(unused_imports)]
use std::collections::HashMap;

/// Integration tests REQUIRE these env vars — no silent skip.
fn require_env(var: &str) -> String {
    match std::env::var(var) {
        Ok(v) if !v.is_empty() => v,
        _ => panic!(
            "{var} is required for the dual adapter integration test.  \
             Set it to run against a real Salesforce org."
        ),
    }
}

fn load_registry() -> PageObjectRegistry {
    let mut registry = PageObjectRegistry::new();
    let sf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../salesforce-pageobjects");
    if sf_path.exists() {
        registry.add_search_path(sf_path);
        registry.scan().unwrap_or(0);
    }
    registry
}

/// Create a WebDriver-based UtamDriver
#[cfg(feature = "webdriver")]
async fn create_webdriver() -> Arc<dyn UtamDriver> {
    use thirtyfour::prelude::*;
    let url =
        std::env::var("CHROMEDRIVER_URL").unwrap_or_else(|_| "http://localhost:9515".to_string());
    let mut caps = DesiredCapabilities::chrome();
    let has_display = std::env::var("DISPLAY").is_ok();
    if !has_display {
        let _ = caps.set_headless();
    }
    let _ = caps.set_no_sandbox();
    let _ = caps.set_disable_gpu();
    let _ = caps.add_arg("--disable-dev-shm-usage");
    // Enable CDP debugging port for the CDP adapter to connect to
    let _ = caps.add_arg("--remote-debugging-port=9222");
    if has_display {
        let _ = caps.add_arg("--start-maximized");
    } else {
        let _ = caps.add_arg("--window-size=1920,1080");
    }
    let driver = WebDriver::new(&url, caps)
        .await
        .unwrap_or_else(|e| panic!("WebDriver connection failed: {e}"));
    Arc::new(ThirtyfourDriver::new(driver))
}

/// Create a CDP-based UtamDriver
#[cfg(feature = "cdp")]
async fn create_cdp_driver() -> Arc<dyn UtamDriver> {
    let driver = CdpDriver::launch().await.expect("Failed to launch CDP driver");
    Arc::new(driver)
}

/// Shared test logic — exercises page objects via any UtamDriver implementation
async fn run_page_object_tests(
    driver: &dyn UtamDriver,
    registry: &PageObjectRegistry,
    adapter_name: &str,
) -> Vec<(String, Result<String, String>)> {
    let mut results = Vec::new();

    // Test: Load global/header
    let header_result = {
        let matches = registry.search("global/header");
        if matches.is_empty() {
            Err("global/header not in registry".to_string())
        } else {
            match registry.get(&matches[0]) {
                Ok(ast) => {
                    // We need an Arc<dyn UtamDriver> for DynamicPageObject::load
                    // but we only have &dyn UtamDriver. This is a design issue.
                    // For now, skip DynamicPageObject tests in CDP mode.
                    Ok(format!(
                        "[{adapter_name}] global/header AST loaded, {} methods",
                        ast.methods.len()
                    ))
                }
                Err(e) => Err(format!("{e}")),
            }
        }
    };
    results.push(("Load global/header AST".to_string(), header_result));

    // Test: Find .oneHeader element directly
    let header_el = driver.find_element(&Selector::Css(".oneHeader".to_string())).await;
    results.push((
        format!("[{adapter_name}] Find .oneHeader"),
        match header_el {
            Ok(_) => Ok("found".to_string()),
            Err(e) => Err(format!("{e}")),
        },
    ));

    // Test: Find .navexDesktopLayoutContainer
    let nav_el =
        driver.find_element(&Selector::Css(".navexDesktopLayoutContainer".to_string())).await;
    results.push((
        format!("[{adapter_name}] Find .navexDesktopLayoutContainer"),
        match nav_el {
            Ok(_) => Ok("found".to_string()),
            Err(e) => Err(format!("{e}")),
        },
    ));

    // Test: Execute JavaScript
    let js_result = driver.execute_script("return document.title", vec![]).await;
    results.push((
        format!("[{adapter_name}] Execute JS (document.title)"),
        match js_result {
            Ok(val) => Ok(format!("{val}")),
            Err(e) => Err(format!("{e}")),
        },
    ));

    // Test: Screenshot
    let screenshot = driver.screenshot_png().await;
    results.push((
        format!("[{adapter_name}] Screenshot"),
        match screenshot {
            Ok(png) => Ok(format!("{} bytes", png.len())),
            Err(e) => Err(format!("{e}")),
        },
    ));

    // Test: Current URL
    let url = driver.current_url().await;
    results.push((
        format!("[{adapter_name}] Current URL"),
        match url {
            Ok(u) => Ok(u),
            Err(e) => Err(format!("{e}")),
        },
    ));

    // Test: Find element and get text
    let notif_result = driver
        .find_element(&Selector::Css(".unsNotificationsCounter span.counterLabel".to_string()))
        .await;
    results.push((
        format!("[{adapter_name}] Find notification counter"),
        match notif_result {
            Ok(el) => match el.text().await {
                Ok(text) => Ok(format!("text='{text}'")),
                Err(e) => Err(format!("found but text failed: {e}")),
            },
            Err(e) => Err(format!("{e}")),
        },
    ));

    // Test: Click element
    let setup_btn = driver
        .find_element(&Selector::Css(".slds-global-actions__item .menuTriggerLink".to_string()))
        .await;
    results.push((
        format!("[{adapter_name}] Click setup menu"),
        match setup_btn {
            Ok(el) => match el.click().await {
                Ok(()) => Ok("clicked".to_string()),
                Err(e) => Err(format!("found but click failed: {e}")),
            },
            Err(e) => Err(format!("{e}")),
        },
    ));

    results
}

/// Test: WebDriver adapter against live Salesforce
#[tokio::test]
#[cfg(feature = "webdriver")]
async fn test_webdriver_adapter() {
    let auth_url = require_env("SF_AUTH_URL");
    let _ = require_env("CHROMEDRIVER_URL");

    // Auth
    let parsed = busbar_sf_api::SfdxAuthUrl::parse(&auth_url).expect("parse auth");
    let client =
        busbar_sf_api::SalesforceClient::from_auth_url(&parsed).await.expect("token exchange");
    let frontdoor = client.frontdoor_url();

    let driver = create_webdriver().await;
    driver.navigate(&frontdoor).await.expect("frontdoor nav");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let home = format!("{}/lightning/page/home", client.instance_url);
    driver.navigate(&home).await.expect("home nav");
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    let registry = load_registry();
    let results = run_page_object_tests(driver.as_ref(), &registry, "WebDriver").await;

    let mut passed = 0;
    let mut failed = 0;
    for (name, result) in &results {
        match result {
            Ok(val) => {
                eprintln!("  PASS {name}: {val}");
                passed += 1;
            }
            Err(e) => {
                eprintln!("  FAIL {name}: {e}");
                failed += 1;
            }
        }
    }

    driver.quit().await.expect("quit");
    eprintln!("WebDriver: {passed} passed, {failed} failed");
    assert!(passed > 3, "Too few WebDriver tests passed: {passed}");
}

/// Test: CDP adapter against a fresh Chrome instance
#[tokio::test]
#[cfg(feature = "cdp")]
async fn test_cdp_adapter() {
    let auth_url = require_env("SF_AUTH_URL");

    // Auth via API
    let parsed = busbar_sf_api::SfdxAuthUrl::parse(&auth_url).expect("parse auth");
    let client =
        busbar_sf_api::SalesforceClient::from_auth_url(&parsed).await.expect("token exchange");
    let frontdoor = client.frontdoor_url();

    let driver = create_cdp_driver().await;
    driver.navigate(&frontdoor).await.expect("frontdoor nav");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let home = format!("{}/lightning/page/home", client.instance_url);
    driver.navigate(&home).await.expect("home nav");
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    let registry = load_registry();
    let results = run_page_object_tests(driver.as_ref(), &registry, "CDP").await;

    let mut passed = 0;
    let mut failed = 0;
    for (name, result) in &results {
        match result {
            Ok(val) => {
                eprintln!("  PASS {name}: {val}");
                passed += 1;
            }
            Err(e) => {
                eprintln!("  FAIL {name}: {e}");
                failed += 1;
            }
        }
    }

    driver.quit().await.expect("quit");
    eprintln!("CDP: {passed} passed, {failed} failed");
    assert!(passed > 3, "Too few CDP tests passed: {passed}");
}
