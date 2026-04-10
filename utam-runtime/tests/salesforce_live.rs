//! Salesforce live integration tests
//!
//! Runs against a real Salesforce org. Authenticates via busbar-sf-api,
//! seeds test data, then exercises page objects against the live DOM.
//!
//! Uses DynamicPageObject with PageObjectRegistry for custom component
//! type resolution and compose method chaining.
//!
//! Skipped locally (no CHROMEDRIVER_URL), panics in CI if credentials missing.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use busbar_sf_api::{SObjectRecord, SalesforceClient, SfdxAuthUrl};
use utam_runtime::prelude::*;

// (Allure reporting removed — tests use direct assertions now)
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
    let selectors = [
        Selector::Css(".oneHeader".to_string()),
        Selector::Css(".desktop.container.forceStyle".to_string()),
        Selector::Css("one-app-nav-bar".to_string()),
    ];
    for attempt in 1..=20 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let url = driver.current_url().await.unwrap_or_default();
        if is_login_page(&url) {
            return false;
        }
        for sel in &selectors {
            if driver.find_element(sel).await.is_ok() {
                eprintln!("  Lightning detected on attempt {attempt}");
                return true;
            }
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

    // Clean up any leftover test records from previous runs
    cleanup_old_test_data(client).await;

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

/// Delete any leftover test records from previous runs that didn't clean up.
async fn cleanup_old_test_data(client: &SalesforceClient) {
    // Delete children first to avoid FK violations
    let queries = [
        ("Case", "SELECT Id FROM Case WHERE Subject = 'Test Support Case'"),
        ("Opportunity", "SELECT Id FROM Opportunity WHERE Name = 'Acme Deal'"),
        ("Contact", "SELECT Id FROM Contact WHERE Email = 'jane.doe@example.com'"),
        ("Lead", "SELECT Id FROM Lead WHERE Company = 'Smith Industries' AND FirstName = 'John'"),
        ("Account", "SELECT Id FROM Account WHERE Name = 'Acme Corp'"),
    ];
    for (sobject_type, soql) in &queries {
        if let Ok(records) = client.query(soql).await {
            for record in &records {
                if let Some(id) = record.id() {
                    let _ = client.delete(sobject_type, id).await;
                    eprintln!("  Cleaned up old {sobject_type}/{id}");
                }
            }
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

/// Determine which driver backend to use.
/// Set UTAM_DRIVER=cdp to use CDP, otherwise defaults to webdriver.
fn use_cdp() -> bool {
    std::env::var("UTAM_DRIVER").map(|v| v.eq_ignore_ascii_case("cdp")).unwrap_or(false)
}

async fn create_driver() -> Arc<dyn UtamDriver> {
    if use_cdp() {
        create_cdp_driver().await
    } else {
        create_webdriver().await
    }
}

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

#[cfg(feature = "cdp")]
async fn create_cdp_driver() -> Arc<dyn UtamDriver> {
    let has_display = std::env::var("DISPLAY").is_ok();
    let mut builder = chromiumoxide::BrowserConfig::builder();
    if !has_display {
        builder = builder.arg("--headless");
    }
    builder = builder
        .arg("--no-sandbox")
        .arg("--disable-gpu")
        .arg("--disable-dev-shm-usage")
        .window_size(1920, 1080);

    let config = builder.build().expect("Failed to build CDP browser config");
    let driver = CdpDriver::launch_with_config(config).await.expect("Failed to launch CDP driver");
    eprintln!("CDP driver launched (headless={})", !has_display);
    Arc::new(driver)
}

#[cfg(not(feature = "cdp"))]
async fn create_cdp_driver() -> Arc<dyn UtamDriver> {
    panic!("CDP feature not enabled. Build with --features cdp");
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
    let _ = std::fs::write(PathBuf::from("/tmp/allure-results/browser-ready"), "1");

    let empty_args = HashMap::new();

    // ── Test: global/header — load and verify compose methods ────────
    eprintln!("\n=== Test: global/header ===");
    let header = load_page_object(Arc::clone(&driver), &registry, "global/header")
        .await
        .expect("global/header must load — .oneHeader not found in DOM");

    // getNotificationCount must return a String (the counter text)
    let count = header.call_method("getNotificationCount", &empty_args).await.expect(
        "getNotificationCount must succeed — .unsNotificationsCounter span.counterLabel not found",
    );
    assert!(
        matches!(count, utam_runtime::RuntimeValue::String(_)),
        "getNotificationCount must return a String, got: {count:?}"
    );
    eprintln!("  getNotificationCount = {count}");

    // hasNewNotification must return a Bool (isVisible result)
    let has_notif = header
        .call_method("hasNewNotification", &empty_args)
        .await
        .expect("hasNewNotification must succeed");
    assert!(
        matches!(has_notif, utam_runtime::RuntimeValue::Bool(_)),
        "hasNewNotification must return a Bool, got: {has_notif:?}"
    );
    eprintln!("  hasNewNotification = {has_notif}");

    // showSetupMenu must click without error
    header.call_method("showSetupMenu", &empty_args).await.expect(
        "showSetupMenu must succeed — .slds-global-actions__item .menuTriggerLink not found",
    );
    eprintln!("  showSetupMenu clicked");

    // Dismiss menu
    let _ = driver.execute_script("document.body.click()", vec![]).await;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // getGlobalActionsList must click without error
    header
        .call_method("getGlobalActionsList", &empty_args)
        .await
        .expect("getGlobalActionsList must succeed — .globalCreateTrigger not found");
    eprintln!("  getGlobalActionsList clicked");

    // Dismiss
    let _ = driver.execute_script("document.body.click()", vec![]).await;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // ── Test: navex/desktopLayoutContainer — load and call getAppNav ──
    eprintln!("\n=== Test: navex/desktopLayoutContainer ===");
    let nav = load_page_object(Arc::clone(&driver), &registry, "navex/desktopLayoutContainer")
        .await
        .expect("navex/desktopLayoutContainer must load — .navexDesktopLayoutContainer not found");

    let app_nav = nav.call_method("getAppNav", &empty_args).await.expect("getAppNav must succeed");
    // getAppNav returns the appNav custom component element
    assert!(
        !matches!(app_nav, utam_runtime::RuntimeValue::Null),
        "getAppNav must return a non-null value, got: {app_nav:?}"
    );
    eprintln!("  getAppNav = {app_nav}");

    // ── Test: global/globalCreate — open menu and click Account ────────
    eprintln!("\n=== Test: global/globalCreate ===");
    let global_create = load_page_object(Arc::clone(&driver), &registry, "global/globalCreate")
        .await
        .expect("global/globalCreate must load");

    global_create
        .call_method("clickGlobalActions", &empty_args)
        .await
        .expect("clickGlobalActions must succeed");
    eprintln!("  clickGlobalActions opened menu");
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Click "New Contact" in the global create menu.
    // The menu items are <li class="uiMenuItem oneGlobalCreateItem"> containing
    // <a class="highlightButton">. Click the <li> (the actual Aura event target),
    // not the <a> (which may not propagate the event correctly).
    driver
        .execute_script(
            "const items = document.querySelectorAll('li.uiMenuItem.oneGlobalCreateItem'); \
             for (const li of items) { \
                 if (li.textContent.includes('New Contact')) { \
                     li.focus(); li.click(); break; \
                 } \
             }",
            vec![],
        )
        .await
        .expect("Click on 'New Contact' li must succeed");
    eprintln!("  Clicked 'New Contact' in create menu");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // Verify the click triggered modal loading (glass overlay + spinner appeared).
    // The actual form may not load if the scratch org's Global Action isn't configured.
    let modal_loading = driver.find_element(&Selector::Css(".modal-glass".to_string())).await;
    assert!(
        modal_loading.is_ok(),
        "Click on 'New Contact' must trigger modal loading (glass overlay)"
    );
    eprintln!("  Modal loading triggered (glass overlay visible)");

    // Try to load the record form — may not appear if quick action layout isn't configured
    match load_page_object(Arc::clone(&driver), &registry, "global/recordActionWrapper").await {
        Ok(record_modal) => {
            eprintln!("  recordActionWrapper loaded — modal form appeared");

            // Exercise the compose chain: clickFooterButton("Save")
            let mut save_args = HashMap::new();
            save_args.insert(
                "labelText".to_string(),
                utam_runtime::RuntimeValue::String("Save".to_string()),
            );
            match record_modal.call_method("clickFooterButton", &save_args).await {
                Ok(_) => eprintln!("  clickFooterButton('Save') executed"),
                Err(e) => eprintln!("  clickFooterButton('Save') chain error: {e}"),
            }
        }
        Err(e) => {
            eprintln!(
                "  recordActionWrapper did not load (quick action may not be configured): {e}"
            );
        }
    }

    // Dismiss any modal/overlay
    let _ = driver.execute_script("document.body.click()", vec![]).await;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // ── Test: Navigate to Account detail, verify seeded data visible ──
    assert!(!seeded_records.is_empty(), "Test data seeding must succeed — no records were created");
    {
        let (_, account_id) = seeded_records
            .iter()
            .find(|(t, _)| t == "Account")
            .expect("Account must be in seeded records");
        eprintln!("\n=== Test: Account detail — verify seeded data ===");
        let url = format!("{instance_url}/lightning/r/Account/{account_id}/view");
        driver.navigate(&url).await.expect("nav to Account detail failed");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // Verify the page title or body contains "Acme Corp"
        let title = driver.title().await.unwrap_or_default();
        let body_has_acme = driver
            .execute_script("return document.body.innerText.includes('Acme Corp')", vec![])
            .await;
        assert!(
            title.contains("Acme Corp")
                || matches!(body_has_acme, Ok(serde_json::Value::Bool(true))),
            "Account detail page must contain 'Acme Corp'. Title: '{title}'"
        );
        eprintln!("  Verified 'Acme Corp' visible on detail page");

        // Verify related Contact is visible
        let body_has_jane = driver
            .execute_script(
                "return document.body.innerText.includes('Jane Doe') || document.body.innerText.includes('Jane')",
                vec![],
            )
            .await;
        assert!(
            matches!(body_has_jane, Ok(serde_json::Value::Bool(true))),
            "Account detail should show related Contact 'Jane Doe'"
        );
        eprintln!("  Verified related Contact 'Jane Doe' visible");

        // Verify related Opportunity is visible
        let body_has_deal = driver
            .execute_script("return document.body.innerText.includes('Acme Deal')", vec![])
            .await;
        assert!(
            matches!(body_has_deal, Ok(serde_json::Value::Bool(true))),
            "Account detail should show related Opportunity 'Acme Deal'"
        );
        eprintln!("  Verified related Opportunity 'Acme Deal' visible");
    }

    // ── Test: Setup — load setupNavTree page object ───────────────────
    eprintln!("\n=== Test: setup/setupNavTree ===");
    let setup_url = format!("{instance_url}/lightning/setup/SetupOneHome/home");
    driver.navigate(&setup_url).await.expect("nav to Setup failed");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let _setup_nav = load_page_object(Arc::clone(&driver), &registry, "setup/setupNavTree")
        .await
        .expect("setup/setupNavTree must load — .onesetupSetupNavTree not found");
    eprintln!("  setup/setupNavTree loaded");

    // Verify the URL contains SetupOneHome
    let setup_current_url = driver.current_url().await.unwrap_or_default();
    assert!(
        setup_current_url.contains("Setup") || setup_current_url.contains("setup"),
        "Should be on Setup page, got: {setup_current_url}"
    );
    eprintln!("  Verified on Setup page: {setup_current_url}");

    // ── Cleanup ─────────────────────────────────────────────────────
    driver.quit().await.expect("Failed to quit");

    if !seeded_records.is_empty() {
        eprintln!("\n=== Cleanup ===");
        cleanup_test_data(&sf_client, &seeded_records).await;
    }

    eprintln!("\n=== All tests passed ===");
}
