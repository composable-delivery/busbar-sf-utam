//! Integration tests for Frame element functionality
//!
//! These tests require a running WebDriver server (ChromeDriver on port 9515).
//! Run with: `cargo test --test frame_integration_tests -- --ignored`

mod common;

use common::*;
use utam_core::prelude::*;

/// Test that we can enter a simple iframe and find elements within it
#[tokio::test]
#[ignore = "Requires ChromeDriver running on port 9515"]
async fn test_enter_simple_frame() -> UtamResult<()> {
    let driver = setup_test_driver(TestDriverConfig::default()).await?;

    // Load test page
    driver.goto(get_test_file_url("frame_test.html")).await?;

    // Verify we're on the main page
    let main_button = driver.find(By::Id("main-button")).await?;
    assert_element_text(&main_button, "Main Page Button").await?;

    // Find the iframe element
    let iframe_element = driver.find(By::Id("simple-frame")).await?;
    let frame = FrameElement::new(iframe_element);

    // Enter the frame context
    let ctx = frame.enter().await?;

    // Find element within the frame
    let frame_button = ctx.find(By::Id("frame-button")).await?;
    assert_element_text(&frame_button, "Frame Button").await?;

    // Verify frame text
    let frame_text = ctx.find(By::Id("frame-text")).await?;
    assert_element_text(&frame_text, "This is content inside the iframe.").await?;

    // Exit the frame explicitly
    ctx.exit().await?;

    // Verify we're back on the main page
    let main_button_again = driver.find(By::Id("main-button")).await?;
    assert_element_visible(&main_button_again).await?;

    driver.quit().await?;
    Ok(())
}

/// Test that frame context switches back automatically on drop
#[tokio::test]
#[ignore = "Requires ChromeDriver running on port 9515"]
async fn test_frame_auto_switch_back_on_drop() -> UtamResult<()> {
    let driver = setup_test_driver(TestDriverConfig::default()).await?;
    driver.goto(get_test_file_url("frame_test.html")).await?;

    // Verify we're on the main page
    let main_button = driver.find(By::Id("main-button")).await?;
    assert_element_visible(&main_button).await?;

    // Enter frame in a scope
    {
        let iframe_element = driver.find(By::Id("simple-frame")).await?;
        let frame = FrameElement::new(iframe_element);
        let ctx = frame.enter().await?;

        // Find element within the frame
        let frame_button = ctx.find(By::Id("frame-button")).await?;
        assert_element_visible(&frame_button).await?;

        // ctx drops here, should switch back automatically
    }

    // Wait a bit for drop cleanup to complete
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Verify we're back on the main page
    let main_button_again = driver.find(By::Id("main-button")).await?;
    assert_element_visible(&main_button_again).await?;

    driver.quit().await?;
    Ok(())
}

/// Test finding multiple elements within a frame
#[tokio::test]
#[ignore = "Requires ChromeDriver running on port 9515"]
async fn test_find_multiple_elements_in_frame() -> UtamResult<()> {
    let driver = setup_test_driver(TestDriverConfig::default()).await?;
    driver.goto(get_test_file_url("frame_test.html")).await?;

    let iframe_element = driver.find(By::Id("simple-frame")).await?;
    let frame = FrameElement::new(iframe_element);
    let ctx = frame.enter().await?;

    // Find multiple elements
    let frame_text = ctx.find(By::Id("frame-text")).await?;
    let frame_button = ctx.find(By::Id("frame-button")).await?;
    let frame_input = ctx.find(By::Id("frame-input")).await?;

    // Verify all elements
    assert_element_visible(&frame_text).await?;
    assert_element_visible(&frame_button).await?;
    assert_element_visible(&frame_input).await?;

    ctx.exit().await?;
    driver.quit().await?;
    Ok(())
}

/// Test nested frame navigation
#[tokio::test]
#[ignore = "Requires ChromeDriver running on port 9515"]
async fn test_nested_frames() -> UtamResult<()> {
    let driver = setup_test_driver(TestDriverConfig::default()).await?;
    driver.goto(get_test_file_url("frame_test.html")).await?;

    // Verify we're on the main page
    let main_button = driver.find(By::Id("main-button")).await?;
    assert_element_visible(&main_button).await?;

    // Enter outer frame
    let outer_iframe = driver.find(By::Id("nested-frame")).await?;
    let outer_frame = FrameElement::new(outer_iframe);
    let outer_ctx = outer_frame.enter().await?;

    // Verify we're in the outer frame
    let outer_button = outer_ctx.find(By::Id("outer-button")).await?;
    assert_element_text(&outer_button, "Outer Frame Button").await?;

    // Enter inner frame
    let inner_iframe = outer_ctx.find(By::Id("inner-frame")).await?;
    let inner_frame = FrameElement::new(inner_iframe);
    let inner_ctx = inner_frame.enter().await?;

    // Verify we're in the inner frame
    let inner_button = inner_ctx.find(By::Id("inner-button")).await?;
    assert_element_text(&inner_button, "Inner Frame Button").await?;

    let inner_text = inner_ctx.find(By::Id("inner-frame-text")).await?;
    assert_element_text(&inner_text, "This is the innermost nested frame.").await?;

    // Exit inner frame
    inner_ctx.exit().await?;

    // Verify we're back in outer frame
    let outer_button_again = outer_ctx.find(By::Id("outer-button")).await?;
    assert_element_visible(&outer_button_again).await?;

    // Exit outer frame
    outer_ctx.exit().await?;

    // Verify we're back on main page
    let main_button_again = driver.find(By::Id("main-button")).await?;
    assert_element_visible(&main_button_again).await?;

    driver.quit().await?;
    Ok(())
}

/// Test interacting with elements in frame
#[tokio::test]
#[ignore = "Requires ChromeDriver running on port 9515"]
async fn test_interact_with_frame_elements() -> UtamResult<()> {
    let driver = setup_test_driver(TestDriverConfig::default()).await?;
    driver.goto(get_test_file_url("frame_test.html")).await?;

    let iframe_element = driver.find(By::Id("simple-frame")).await?;
    let frame = FrameElement::new(iframe_element);
    let ctx = frame.enter().await?;

    // Type into input field
    let input = ctx.find(By::Id("frame-input")).await?;
    input.send_keys("Test input").await?;

    // Verify the value was set
    let value = input.value().await?.unwrap_or_default();
    assert_eq!(value, "Test input", "Expected input value to be 'Test input'");

    // Click the button
    let button = ctx.find(By::Id("frame-button")).await?;
    button.click().await?;

    ctx.exit().await?;
    driver.quit().await?;
    Ok(())
}

/// Test error handling when trying to find non-existent element in frame
#[tokio::test]
#[ignore = "Requires ChromeDriver running on port 9515"]
async fn test_error_element_not_found_in_frame() -> UtamResult<()> {
    let driver = setup_test_driver(TestDriverConfig::default()).await?;
    driver.goto(get_test_file_url("frame_test.html")).await?;

    let iframe_element = driver.find(By::Id("simple-frame")).await?;
    let frame = FrameElement::new(iframe_element);
    let ctx = frame.enter().await?;

    // Try to find non-existent element
    let result = ctx.find(By::Id("non-existent-element")).await;
    assert!(result.is_err(), "Expected error when finding non-existent element");

    ctx.exit().await?;
    driver.quit().await?;
    Ok(())
}
