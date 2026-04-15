//! Salesforce browser session — auth, driver, registry, and test lifecycle.
//!
//! [`SalesforceSession`] owns the browser, the page object registry, and the
//! Allure writer.  Each test module receives a `&SalesforceSession` and returns
//! one or more `AllureTestResult`s that the orchestrator writes to disk.
//!
//! Test data uses a **session-scoped tag** embedded in record names so
//! concurrent test runs (e.g. the webdriver and cdp matrix cells hitting
//! the same org in parallel) can seed and clean up without colliding.
//! See [`RunTag`] for the format.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use busbar_sf_api::{SObjectRecord, SalesforceClient, SfdxAuthUrl};
use utam_runtime::prelude::*;
use utam_test::allure::{AllureCategory, AllureStatus, AllureWriter};

/// Identifies the test records created by this process so concurrent
/// runs don't delete each other's data.
///
/// Format: `{driver}-{unix-millis-hex}` e.g. `webdriver-19a4c7b8f12`.
///
/// - `{driver}` comes from `UTAM_DRIVER` (or `"local"`) so the matrix
///   cells are guaranteed to disagree.
/// - `{unix-millis-hex}` disambiguates consecutive runs with the same
///   driver — each run gets its own tag; old runs' data can still be
///   queried and cleaned via a LIKE pattern.
///
/// Example record names:
///   Account.Name     = "Acme Corp [webdriver-19a4c7b8f12]"
///   Contact.LastName = "Doe-webdriver-19a4c7b8f12"
///   Opportunity.Name = "Acme Deal [cdp-19a4c7b900a]"
#[derive(Debug, Clone)]
pub struct RunTag {
    pub driver: String,
    pub stamp: String,
}

impl RunTag {
    pub fn generate(driver: &str) -> Self {
        let stamp = format!(
            "{:x}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        );
        Self { driver: driver.to_string(), stamp }
    }

    /// Full tag — unique per (driver, process start time).
    pub fn full(&self) -> String {
        format!("{}-{}", self.driver, self.stamp)
    }

    /// Pattern matching THIS driver's records across runs (including
    /// orphans from crashed runs).  Used for age-based cleanup.
    pub fn driver_like(&self) -> String {
        format!("%[{}-%]", self.driver)
    }

    /// Pattern matching exactly THIS run's records.
    #[allow(dead_code)]
    pub fn exact_like(&self) -> String {
        format!("%[{}]", self.full())
    }
}

/// Shared state for the entire Salesforce live test run.
pub struct SalesforceSession {
    pub driver: Arc<dyn UtamDriver>,
    pub registry: Arc<PageObjectRegistry>,
    pub allure: AllureWriter,
    pub sf_client: SalesforceClient,
    pub instance_url: String,
    pub seeded_records: Vec<(String, String)>,
    /// Held so tests that want to reference the run tag (for ad-hoc
    /// Allure parameters or custom cleanup) can read it.
    #[allow(dead_code)]
    pub run_tag: RunTag,
    driver_name: String,
}

impl SalesforceSession {
    /// Set up the full session: authenticate, seed data, launch browser.
    ///
    /// Integration tests REQUIRE a Salesforce org — this panics loudly
    /// if credentials are missing, if auth fails, or if the browser
    /// can't reach Lightning.  There is no silent skip path.  Tests
    /// that "pass" without a real org give false confidence.
    pub async fn setup() -> Self {
        // ── Salesforce auth ────────────────────────────────────────────
        let auth_url = std::env::var("SF_AUTH_URL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                panic!(
                    "SF_AUTH_URL is required.  Integration tests need a real \
                     Salesforce org.  Set SF_AUTH_URL to an sfdx auth URL \
                     (force://<clientId>:<secret>:<refreshToken>@<instance>)."
                )
            });
        eprintln!("[setup] SF_AUTH_URL present (len={})", auth_url.len());

        let parsed =
            SfdxAuthUrl::parse(&auth_url).expect("Failed to parse SF_AUTH_URL");
        let sf_client = SalesforceClient::from_auth_url(&parsed)
            .await
            .expect("Failed to exchange refresh token — check SF_AUTH_URL is valid");
        let instance_url = sf_client.instance_url.clone();
        eprintln!("Authenticated to {instance_url}");

        // Generate a per-run tag so concurrent matrix cells don't collide
        // on seed/cleanup.  Must happen BEFORE seeding so records can be
        // tagged as we create them.
        let driver_name = if use_cdp() { "cdp" } else { "webdriver" };
        let run_tag = RunTag::generate(driver_name);
        eprintln!("[setup] run_tag = {}", run_tag.full());

        // ── Seed test data ─────────────────────────────────────────────
        eprintln!("\n=== Seed Test Data ===");
        let seeded_records = seed_test_data(&sf_client, &run_tag).await;

        // ── Browser + registry ─────────────────────────────────────────
        let driver = create_driver().await;
        let registry = Arc::new(load_registry());

        // ── Authenticate browser via frontdoor ─────────────────────────
        eprintln!("\n=== Authenticate Browser ===");
        let frontdoor_url = sf_client.frontdoor_url();
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
        assert!(
            wait_for_lightning(driver.as_ref()).await,
            "Lightning did not load after auth"
        );
        eprintln!("Lightning loaded");

        // ── Allure setup ───────────────────────────────────────────────
        let allure = AllureWriter::from_env();
        allure
            .write_environment(&[
                ("Driver", driver_name),
                ("Browser", "Chrome"),
                ("Platform", "Linux"),
                ("Instance", &instance_url),
            ])
            .unwrap_or_else(|e| eprintln!("WARNING: failed to write environment.properties: {e}"));

        allure
            .write_categories(&[
                AllureCategory {
                    name: "Auth failures".into(),
                    description: Some("Session expired or access denied".into()),
                    message_regex: Some(".*login.*|.*auth.*|.*frontdoor.*".into()),
                    trace_regex: None,
                    matched_statuses: vec![AllureStatus::Failed],
                },
                AllureCategory {
                    name: "Element not found".into(),
                    description: Some("Page object element missing from DOM".into()),
                    message_regex: Some(".*ElementNotDefined.*|.*not found.*".into()),
                    trace_regex: None,
                    matched_statuses: vec![AllureStatus::Broken],
                },
                AllureCategory {
                    name: "Timeout".into(),
                    description: Some("Wait condition not met".into()),
                    message_regex: Some(".*timed out.*|.*Timeout.*".into()),
                    trace_regex: None,
                    matched_statuses: vec![AllureStatus::Broken],
                },
            ])
            .unwrap_or_else(|e| eprintln!("WARNING: failed to write categories.json: {e}"));

        // Signal to CI: safe to start video recording
        let _ = std::fs::create_dir_all("/tmp/allure-results");
        let _ = std::fs::write("/tmp/allure-results/browser-ready", "1");

        Self {
            driver,
            registry,
            allure,
            sf_client,
            instance_url,
            seeded_records,
            run_tag,
            driver_name: driver_name.to_string(),
        }
    }

    /// Which driver adapter is in use.
    pub fn driver_name(&self) -> &str {
        &self.driver_name
    }

    /// Load a page object by registry path (e.g. `"global/header"`).
    pub async fn load_page_object(&self, name: &str) -> Result<DynamicPageObject, String> {
        let matches = self.registry.search(name);
        if matches.is_empty() {
            return Err(format!("{name} not found in registry"));
        }
        let ast = self.registry.get(&matches[0]).map_err(|e| format!("{e}"))?;
        DynamicPageObject::load(Arc::clone(&self.driver), ast)
            .await
            .map(|po| po.with_registry(Arc::clone(&self.registry)))
            .map_err(|e| format!("{e}"))
    }

    /// Navigate the browser to a URL and wait briefly for load.
    pub async fn navigate(&self, url: &str) {
        self.driver.navigate(url).await.expect("navigation failed");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

}

// ---------------------------------------------------------------------------
// Infrastructure helpers (private)
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

fn use_cdp() -> bool {
    std::env::var("UTAM_DRIVER")
        .map(|v| v.eq_ignore_ascii_case("cdp"))
        .unwrap_or(false)
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
    let driver =
        CdpDriver::launch_with_config(config).await.expect("Failed to launch CDP driver");
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

async fn seed_test_data(
    client: &SalesforceClient,
    tag: &RunTag,
) -> Vec<(String, String)> {
    eprintln!("Seeding test data (tag={})", tag.full());
    cleanup_old_test_data(client, tag).await;

    // Each record embeds the run tag in a queryable field so cleanup
    // can find it unambiguously.  Names are `base [tag]`; tag-on-Name
    // fields work on any object.
    let t = tag.full();
    let account_name = format!("Acme Corp [{t}]");
    let contact_last = format!("Doe [{t}]");
    let contact_email = format!("jane.doe+{t}@example.com");
    let opp_name = format!("Acme Deal [{t}]");
    let lead_last = format!("Smith [{t}]");
    let lead_company = format!("Smith Industries [{t}]");
    let case_subject = format!("Test Support Case [{t}]");

    let records = vec![
        (
            "account1",
            "Account",
            SObjectRecord::new()
                .field("Name", account_name.as_str())
                .field("Industry", "Technology")
                .field("Phone", "(555) 123-4567"),
        ),
        (
            "contact1",
            "Contact",
            SObjectRecord::new()
                .field("FirstName", "Jane")
                .field("LastName", contact_last.as_str())
                .field("Email", contact_email.as_str())
                .field("AccountId", "@{account1.id}"),
        ),
        (
            "opportunity1",
            "Opportunity",
            SObjectRecord::new()
                .field("Name", opp_name.as_str())
                .field("StageName", "Prospecting")
                .field("CloseDate", "2026-12-31")
                .field("AccountId", "@{account1.id}"),
        ),
        (
            "lead1",
            "Lead",
            SObjectRecord::new()
                .field("FirstName", "John")
                .field("LastName", lead_last.as_str())
                .field("Company", lead_company.as_str()),
        ),
        (
            "case1",
            "Case",
            SObjectRecord::new()
                .field("Subject", case_subject.as_str())
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
            for sobject_type in &types {
                let soql = format!(
                    "SELECT Id FROM {sobject_type} ORDER BY CreatedDate DESC LIMIT 200 FOR VIEW"
                );
                match client.query(&soql).await {
                    Ok(rows) => {
                        eprintln!("  Marked {} {sobject_type} recently viewed", rows.len())
                    }
                    Err(e) => eprintln!("  WARNING: FOR VIEW failed for {sobject_type}: {e}"),
                }
            }
            pairs
        }
        Err(e) => {
            panic!(
                "Failed to seed test data: {e}.  The Salesforce org \
                 rejected record creation — check permissions, object-level \
                 access, or required fields on Account/Contact/Opportunity/Lead/Case."
            );
        }
    }
}

/// Delete orphaned records from *this driver's* previous crashed runs.
///
/// Matches on `Name LIKE '...[%driver%]'` so we only touch records this
/// driver created — never records another driver's concurrent run just
/// seeded.  Child records (Case/Opportunity/Contact) are deleted before
/// Account to avoid FK-cascade surprises.
async fn cleanup_old_test_data(client: &SalesforceClient, tag: &RunTag) {
    let dl = tag.driver_like();
    let queries: [(&str, String); 5] = [
        ("Case", format!("SELECT Id FROM Case WHERE Subject LIKE '{dl}'")),
        ("Opportunity", format!("SELECT Id FROM Opportunity WHERE Name LIKE '{dl}'")),
        (
            "Contact",
            format!("SELECT Id FROM Contact WHERE LastName LIKE '{dl}'"),
        ),
        ("Lead", format!("SELECT Id FROM Lead WHERE Company LIKE '{dl}'")),
        ("Account", format!("SELECT Id FROM Account WHERE Name LIKE '{dl}'")),
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
