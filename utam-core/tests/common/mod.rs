//! Test utilities for UTAM core runtime tests
//!
//! Provides mock WebDriver setup and common assertions.

use utam_core::prelude::*;

/// Mock WebDriver configuration for testing
pub struct MockDriverConfig {
    pub headless: bool,
    pub implicit_wait_ms: u64,
}

impl Default for MockDriverConfig {
    fn default() -> Self {
        Self {
            headless: true,
            implicit_wait_ms: 5000,
        }
    }
}

/// Setup a mock WebDriver for testing
///
/// Note: This is a placeholder for actual mock implementation.
/// In real tests, you would either:
/// 1. Use a real WebDriver with a test browser
/// 2. Use a mock WebDriver implementation
/// 3. Use dependency injection to provide test doubles
#[allow(dead_code)]
pub async fn setup_mock_driver() -> UtamResult<()> {
    // TODO: Implement mock WebDriver or use test browser
    // For now, this is a placeholder that shows the API
    Ok(())
}

/// Assert that an element is visible
#[allow(dead_code)]
#[track_caller]
pub async fn assert_element_visible(element: &WebElement) -> UtamResult<()> {
    let is_displayed = element.is_displayed().await?;
    assert!(is_displayed, "Expected element to be visible");
    Ok(())
}

/// Assert that an element is not visible
#[allow(dead_code)]
#[track_caller]
pub async fn assert_element_not_visible(element: &WebElement) -> UtamResult<()> {
    let is_displayed = element.is_displayed().await?;
    assert!(!is_displayed, "Expected element to not be visible");
    Ok(())
}

/// Assert that an element has expected text
#[allow(dead_code)]
#[track_caller]
pub async fn assert_element_text(element: &WebElement, expected: &str) -> UtamResult<()> {
    let text = element.text().await?;
    assert_eq!(
        text, expected,
        "Expected element text to be '{}', but got '{}'",
        expected, text
    );
    Ok(())
}

/// Assert that an element has expected attribute value
#[allow(dead_code)]
#[track_caller]
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
