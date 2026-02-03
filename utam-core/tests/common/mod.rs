//! Test utilities for UTAM core runtime tests
//!
//! Provides WebDriver setup and common assertions for integration testing.

use std::path::PathBuf;
use utam_core::prelude::*;

/// WebDriver configuration for testing
#[allow(dead_code)]
pub struct TestDriverConfig {
    pub headless: bool,
    pub implicit_wait_ms: u64,
}

impl Default for TestDriverConfig {
    fn default() -> Self {
        Self { headless: true, implicit_wait_ms: 5000 }
    }
}

/// Setup a test WebDriver for integration tests
///
/// This requires a running WebDriver server (e.g., ChromeDriver).
/// Tests using this should be marked with `#[ignore]` by default
/// and run explicitly with `cargo test -- --ignored`.
#[allow(dead_code)]
pub async fn setup_test_driver(config: TestDriverConfig) -> UtamResult<WebDriver> {
    use thirtyfour::{ChromiumLikeCapabilities, DesiredCapabilities};

    let mut caps = DesiredCapabilities::chrome();
    if config.headless {
        caps.set_headless()?;
    }

    // Try to connect to ChromeDriver on default port
    let driver = WebDriver::new("http://localhost:9515", caps)
        .await
        .map_err(UtamError::WebDriver)?;

    // Set implicit wait
    driver
        .set_implicit_wait_timeout(std::time::Duration::from_millis(config.implicit_wait_ms))
        .await?;

    Ok(driver)
}

/// Get the file:// URL for a test HTML file
#[allow(dead_code)]
pub fn get_test_file_url(filename: &str) -> String {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("testdata");
    path.push(filename);

    format!("file://{}", path.display())
}

/// Assert that an element is visible
#[allow(dead_code)]
pub async fn assert_element_visible(element: &WebElement) -> UtamResult<()> {
    let is_displayed = element.is_displayed().await?;
    assert!(is_displayed, "Expected element to be visible");
    Ok(())
}

/// Assert that an element is not visible
#[allow(dead_code)]
pub async fn assert_element_not_visible(element: &WebElement) -> UtamResult<()> {
    let is_displayed = element.is_displayed().await?;
    assert!(!is_displayed, "Expected element to not be visible");
    Ok(())
}

/// Assert that an element has expected text
#[allow(dead_code)]
pub async fn assert_element_text(element: &WebElement, expected: &str) -> UtamResult<()> {
    let text = element.text().await?;
    assert_eq!(text, expected, "Expected element text to be '{}', but got '{}'", expected, text);
    Ok(())
}

/// Assert that an element has expected attribute value
#[allow(dead_code)]
pub async fn assert_element_attribute(
    element: &WebElement,
    attr: &str,
    expected: &str,
) -> UtamResult<()> {
    let value = element.attr(attr).await?.unwrap_or_default();
    assert_eq!(
        value, expected,
        "Expected element attribute '{}' to be '{}', but got '{}'",
        attr, expected, value
    );
    Ok(())
}
