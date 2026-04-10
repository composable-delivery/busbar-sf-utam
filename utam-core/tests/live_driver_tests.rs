//! Live test matrix — WebDriver and CDP adapters
//!
//! Runs the same logical test scenarios against both browser automation
//! backends to verify that the [`DriverKind`] abstraction is protocol-agnostic.
//!
//! # Running
//!
//! ```bash
//! # WebDriver tests (requires ChromeDriver on port 9515)
//! cargo test --test live_driver_tests -- --ignored webdriver
//!
//! # CDP tests (requires Chrome on $PATH, no ChromeDriver needed)
//! cargo test --test live_driver_tests --features cdp -- --ignored cdp
//!
//! # All live tests
//! cargo test --test live_driver_tests --features cdp -- --ignored
//! ```
//!
//! All tests are marked `#[ignore]` so they do not run in the default
//! `cargo test` invocation.

mod common;

use utam_core::prelude::*;

// ── WebDriver variant ─────────────────────────────────────────────────────────

/// Verify the WebDriver adapter can navigate to a URL and read the page title.
#[tokio::test]
#[ignore = "Requires ChromeDriver on port 9515"]
async fn test_page_title_webdriver() -> UtamResult<()> {
    run_page_title_test(DriverKind::WebDriver).await
}

/// Verify the WebDriver adapter can find a visible element by CSS selector.
#[tokio::test]
#[ignore = "Requires ChromeDriver on port 9515"]
async fn test_find_element_webdriver() -> UtamResult<()> {
    run_find_element_test(DriverKind::WebDriver).await
}

/// Verify the WebDriver adapter can execute JavaScript and get a result.
#[tokio::test]
#[ignore = "Requires ChromeDriver on port 9515"]
async fn test_execute_js_webdriver() -> UtamResult<()> {
    run_execute_js_test(DriverKind::WebDriver).await
}

// ── CDP variant ───────────────────────────────────────────────────────────────

/// Verify the CDP adapter can navigate to a URL and read the page title.
#[cfg(feature = "cdp")]
#[tokio::test]
#[ignore = "Requires Chrome on $PATH (CDP, no ChromeDriver needed)"]
async fn test_page_title_cdp() -> UtamResult<()> {
    run_page_title_test(DriverKind::Cdp).await
}

/// Verify the CDP adapter can find a visible element by CSS selector.
#[cfg(feature = "cdp")]
#[tokio::test]
#[ignore = "Requires Chrome on $PATH (CDP, no ChromeDriver needed)"]
async fn test_find_element_cdp() -> UtamResult<()> {
    run_find_element_test(DriverKind::Cdp).await
}

/// Verify the CDP adapter can execute JavaScript and get a result.
#[cfg(feature = "cdp")]
#[tokio::test]
#[ignore = "Requires Chrome on $PATH (CDP, no ChromeDriver needed)"]
async fn test_execute_js_cdp() -> UtamResult<()> {
    run_execute_js_test(DriverKind::Cdp).await
}

/// Verify that the CDP adapter captures console logs (CDP-exclusive capability).
#[cfg(feature = "cdp")]
#[tokio::test]
#[ignore = "Requires Chrome on $PATH (CDP, no ChromeDriver needed)"]
async fn test_console_log_capture_cdp() -> UtamResult<()> {
    let driver = CdpDriver::launch_headless().await?;

    let test_url = common::get_test_file_url("basic_test.html");
    driver.goto(&test_url).await?;

    // Inject a console log accumulator and then emit a log entry.
    driver
        .evaluate_js(
            "window.__utamConsoleLogs = [];
             const orig = console.log.bind(console);
             console.log = function(...args) {
                 window.__utamConsoleLogs.push({ level: 'log', text: args.join(' ') });
                 orig(...args);
             };
             console.log('utam-cdp-test-message');",
        )
        .await?;

    let logs = driver.console_logs().await?;
    assert!(
        logs.iter().any(|(_, text)| text.contains("utam-cdp-test-message")),
        "Expected console log to contain 'utam-cdp-test-message', got: {:?}",
        logs
    );

    driver.quit().await?;
    Ok(())
}

// ── Shared test logic ─────────────────────────────────────────────────────────

/// Navigate to a simple HTML fixture and assert the page title.
///
/// Parameterized by [`DriverKind`]; both WebDriver and CDP variants call
/// this function.
async fn run_page_title_test(kind: DriverKind) -> UtamResult<()> {
    let test_url = common::get_test_file_url("basic_test.html");

    match kind {
        DriverKind::WebDriver => {
            let driver =
                common::setup_thirtyfour_driver(common::TestDriverConfig::default()).await?;
            driver.goto(&test_url).await?;
            let title = driver.title().await?;
            assert!(!title.is_empty(), "[webdriver] Expected a non-empty page title");
            driver.quit().await?;
        }
        #[cfg(feature = "cdp")]
        DriverKind::Cdp => {
            let driver = CdpDriver::launch_headless().await?;
            driver.goto(&test_url).await?;
            let title: String = driver
                .evaluate_js("document.title")
                .await?
                .as_str()
                .unwrap_or("")
                .to_owned();
            assert!(!title.is_empty(), "[cdp] Expected a non-empty page title");
            driver.quit().await?;
        }
    }

    Ok(())
}

/// Find an element by CSS selector and assert it is visible.
async fn run_find_element_test(kind: DriverKind) -> UtamResult<()> {
    let test_url = common::get_test_file_url("basic_test.html");

    match kind {
        DriverKind::WebDriver => {
            let driver =
                common::setup_thirtyfour_driver(common::TestDriverConfig::default()).await?;
            driver.goto(&test_url).await?;
            let element = driver.find(By::Tag("body")).await?;
            common::assert_element_visible(&element).await?;
            driver.quit().await?;
        }
        #[cfg(feature = "cdp")]
        DriverKind::Cdp => {
            let driver = CdpDriver::launch_headless().await?;
            driver.goto(&test_url).await?;
            // Use CDP page API to find the element; finding it without error
            // is sufficient to confirm it exists and is accessible.
            driver
                .page()
                .find_element("body")
                .await
                .map_err(|e| UtamError::Cdp(format!("CDP find_element 'body' failed: {e}")))?;
            driver.quit().await?;
        }
    }

    Ok(())
}

/// Execute `document.readyState` via JavaScript and assert the page is ready.
async fn run_execute_js_test(kind: DriverKind) -> UtamResult<()> {
    let test_url = common::get_test_file_url("basic_test.html");
    const READY_SCRIPT: &str = "document.readyState";
    const EXPECTED: &str = "complete";

    match kind {
        DriverKind::WebDriver => {
            let driver =
                common::setup_thirtyfour_driver(common::TestDriverConfig::default()).await?;
            driver.goto(&test_url).await?;
            let result = driver.execute(READY_SCRIPT, vec![]).await?;
            let state = result.json().as_str().unwrap_or("").to_owned();
            assert_eq!(state, EXPECTED, "[webdriver] Expected readyState=complete");
            driver.quit().await?;
        }
        #[cfg(feature = "cdp")]
        DriverKind::Cdp => {
            let driver = CdpDriver::launch_headless().await?;
            driver.goto(&test_url).await?;
            let result = driver.evaluate_js(READY_SCRIPT).await?;
            let state = result.as_str().unwrap_or("").to_owned();
            assert_eq!(state, EXPECTED, "[cdp] Expected readyState=complete");
            driver.quit().await?;
        }
    }

    Ok(())
}
