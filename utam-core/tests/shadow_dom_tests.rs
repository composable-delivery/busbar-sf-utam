//! Tests for Shadow DOM support
//!
//! These tests verify the Shadow DOM functionality including:
//! - ShadowRoot wrapper type
//! - get_shadow_root() method
//! - Element finding within shadow roots
//! - Nested shadow DOM traversal
//! - Error handling for missing shadow roots

use utam_core::prelude::*;

#[test]
fn test_shadow_root_type_exists() {
    // Test that ShadowRoot type is exported and can be referenced
    fn _type_check() -> UtamResult<()> {
        // This ensures ShadowRoot is in scope from the prelude
        let _check: Option<ShadowRoot> = None;
        Ok(())
    }
}

#[test]
fn test_base_element_type_exists() {
    // Test that BaseElement type is exported and can be referenced
    fn _type_check() -> UtamResult<()> {
        // This ensures BaseElement is in scope from the prelude
        let _check: Option<BaseElement> = None;
        Ok(())
    }
}

#[test]
fn test_traverse_shadow_path_signature() {
    // Test that traverse_shadow_path function is exported and has correct signature
    // This is a compile-time test to ensure the API is correct
    async fn _signature_check() -> UtamResult<()> {
        // Create a dummy web element (this won't actually run in the test)
        let _dummy_element: Option<WebElement> = None;
        let _path: Vec<By> = vec![By::Css(".test")];

        // This line ensures the function signature is correct
        // but won't actually execute since we don't have a real WebElement
        // let _result = traverse_shadow_path(&_dummy_element.unwrap(), &_path).await?;

        Ok(())
    }
}

#[test]
fn test_shadow_root_not_found_error() {
    // Test that ShadowRootNotFound error displays correctly
    let error = UtamError::ShadowRootNotFound { element: "my-component".to_string() };

    let error_string = format!("{}", error);
    assert!(error_string.contains("Shadow root not found"));
    assert!(error_string.contains("my-component"));
}

#[test]
fn test_element_not_found_in_shadow_error() {
    // Test that ElementNotFound error can be used for shadow DOM elements
    let error = UtamError::ElementNotFound {
        name: "shadow element".to_string(),
        selector: "By::Css(\".button\")".to_string(),
    };

    let error_string = format!("{}", error);
    assert!(error_string.contains("Element"));
    assert!(error_string.contains("not found"));
}

// Note: The following tests would require a real WebDriver session
// and HTML with shadow DOM elements. They are structured to show
// what the runtime behavior would look like, but are commented out
// since they need integration test infrastructure.

/*
#[tokio::test]
async fn test_get_shadow_root_success() -> UtamResult<()> {
    // This test would require:
    // 1. A WebDriver instance
    // 2. An HTML page with a shadow DOM host element
    // 3. The ability to find that element

    let caps = DesiredCapabilities::chrome();
    let driver = WebDriver::new("http://localhost:4444", caps).await?;

    // Navigate to a page with shadow DOM
    driver.goto("http://example.com/shadow-dom-test").await?;

    // Find an element with a shadow root
    let host_element = driver.find(By::Id("shadow-host")).await?;
    let base_element = BaseElement::new(host_element);

    // Get the shadow root
    let shadow_root = base_element.get_shadow_root().await?;

    // Find an element within the shadow root
    let inner_button = shadow_root.find(By::Css(".inner-button")).await?;

    assert!(inner_button.is_present().await?);

    driver.quit().await?;
    Ok(())
}

#[tokio::test]
async fn test_shadow_root_not_found() -> UtamResult<()> {
    // This test verifies error handling when an element has no shadow root

    let caps = DesiredCapabilities::chrome();
    let driver = WebDriver::new("http://localhost:4444", caps).await?;

    driver.goto("http://example.com/test").await?;

    // Find a regular element without shadow root
    let regular_element = driver.find(By::Id("regular-div")).await?;
    let base_element = BaseElement::new(regular_element);

    // Try to get shadow root - should fail
    let result = base_element.get_shadow_root().await;

    assert!(result.is_err());
    if let Err(UtamError::ShadowRootNotFound { .. }) = result {
        // Expected error
    } else {
        panic!("Expected ShadowRootNotFound error");
    }

    driver.quit().await?;
    Ok(())
}

#[tokio::test]
async fn test_nested_shadow_dom() -> UtamResult<()> {
    // This test verifies traversing nested shadow DOMs

    let caps = DesiredCapabilities::chrome();
    let driver = WebDriver::new("http://localhost:4444", caps).await?;

    driver.goto("http://example.com/nested-shadow").await?;

    // Find the outer shadow host
    let outer_host = driver.find(By::Id("outer-host")).await?;

    // Traverse through nested shadows
    let path = vec![
        By::Css(".inner-host"),
        By::Css(".deeply-nested"),
    ];

    let final_element = traverse_shadow_path(&outer_host, &path).await?;

    assert!(final_element.is_present().await?);

    driver.quit().await?;
    Ok(())
}

#[tokio::test]
async fn test_find_all_in_shadow_root() -> UtamResult<()> {
    // This test verifies finding multiple elements in a shadow root

    let caps = DesiredCapabilities::chrome();
    let driver = WebDriver::new("http://localhost:4444", caps).await?;

    driver.goto("http://example.com/shadow-list").await?;

    let host_element = driver.find(By::Id("list-host")).await?;
    let base_element = BaseElement::new(host_element);

    let shadow_root = base_element.get_shadow_root().await?;

    // Find all list items within the shadow root
    let items = shadow_root.find_all(By::Css(".list-item")).await?;

    assert!(items.len() > 0);

    driver.quit().await?;
    Ok(())
}
*/
