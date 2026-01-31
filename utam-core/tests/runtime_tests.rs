//! Integration tests for UTAM core runtime
//!
//! Tests runtime traits and element wrappers.

mod common;

use utam_core::prelude::*;

#[test]
fn test_error_types() {
    // Test that error types can be constructed
    let _error = UtamError::ElementNotFound {
        name: "testButton".to_string(),
        selector: ".test".to_string(),
    };
}

#[test]
fn test_prelude_exports() {
    // Test that all expected types are exported from prelude
    // This ensures the public API is stable
    let _result: UtamResult<()> = Ok(());
}

#[test]
fn test_actionable_element_creation() {
    // Test that ActionableElement type exists and can be referenced
    // This validates the structure compiles correctly
    fn _takes_actionable<T: Actionable>(_element: T) {}
}

#[test]
fn test_base_element_exists() {
    // Test that BaseElement type is accessible
    // This validates exports are correct
    fn _takes_base(_element: BaseElement) {}
}

#[test]
fn test_actionable_trait_bounds() {
    // Test that Actionable trait has correct bounds
    // This ensures Send + Sync requirements
    fn _assert_send_sync<T: Actionable>() {
        fn _is_send<S: Send>() {}
        fn _is_sync<S: Sync>() {}
        _is_send::<T>();
        _is_sync::<T>();
    }
}

// Note: The following tests require a running WebDriver instance.
// They are marked with #[ignore] by default and can be run with:
// cargo test -- --ignored --test-threads=1
//
// To run these tests, start a WebDriver server first:
// chromedriver --port=4444
// or
// geckodriver --port=4444

#[cfg(test)]
mod actionable_tests {
    use super::*;

    /// Helper to create a test WebDriver instance
    /// This requires a running WebDriver server on localhost:4444
    async fn setup_driver() -> WebDriverResult<WebDriver> {
        let caps = DesiredCapabilities::chrome();
        WebDriver::new("http://localhost:4444", caps).await
    }

    /// Helper to create a test page with interactive elements
    async fn setup_test_page(driver: &WebDriver) -> WebDriverResult<()> {
        driver
            .goto(
                "data:text/html,<html><body>\
                <input id='testInput' type='text' value='test'>\
                <button id='testButton'>Click Me</button>\
                <div id='testDiv' style='height:2000px'>Content</div>\
                </body></html>",
            )
            .await
    }

    #[tokio::test]
    #[ignore]
    async fn test_actionable_focus() -> UtamResult<()> {
        let driver = setup_driver().await?;
        setup_test_page(&driver).await?;

        let element = driver.find(By::Id("testInput")).await?;
        let actionable = ActionableElement::new(element);

        // Focus the element
        actionable.focus().await?;

        // Verify element has focus by checking document.activeElement
        let result = driver.execute("return document.activeElement.id;", vec![]).await?;
        let active_id: String = result.convert()?;

        assert_eq!(active_id, "testInput");

        driver.quit().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_actionable_blur() -> UtamResult<()> {
        let driver = setup_driver().await?;
        setup_test_page(&driver).await?;

        let element = driver.find(By::Id("testInput")).await?;
        let actionable = ActionableElement::new(element);

        // Focus then blur the element
        actionable.focus().await?;
        actionable.blur().await?;

        // Verify element lost focus
        let result = driver.execute("return document.activeElement.id || 'body';", vec![]).await?;
        let active_id: String = result.convert()?;

        assert_ne!(active_id, "testInput");

        driver.quit().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_actionable_scroll_to_center() -> UtamResult<()> {
        let driver = setup_driver().await?;
        driver
            .goto(
                "data:text/html,<html><body style='height:3000px'>\
                <div id='top' style='height:1000px'>Top</div>\
                <div id='target' style='height:100px;background:red'>Target</div>\
                <div id='bottom' style='height:1900px'>Bottom</div>\
                </body></html>",
            )
            .await?;

        let element = driver.find(By::Id("target")).await?;
        let actionable = ActionableElement::new(element.clone());

        // Scroll to center
        actionable.scroll_to_center().await?;

        // Wait a bit for scroll to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify element is in viewport (scroll position > 0)
        let result = driver.execute("return window.scrollY;", vec![]).await?;
        let scroll_y: i64 = result.convert()?;

        assert!(scroll_y > 0, "Page should be scrolled");

        driver.quit().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_actionable_scroll_to_top() -> UtamResult<()> {
        let driver = setup_driver().await?;
        driver
            .goto(
                "data:text/html,<html><body style='height:3000px'>\
                <div id='top' style='height:1000px'>Top</div>\
                <div id='target' style='height:100px;background:red'>Target</div>\
                <div id='bottom' style='height:1900px'>Bottom</div>\
                </body></html>",
            )
            .await?;

        let element = driver.find(By::Id("target")).await?;
        let actionable = ActionableElement::new(element.clone());

        // First scroll down
        driver.execute("window.scrollTo(0, 2000);", vec![]).await?;

        // Then scroll element to top
        actionable.scroll_to_top().await?;

        // Wait a bit for scroll to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify element is near top of viewport
        let result = driver.execute("return window.scrollY;", vec![]).await?;
        let scroll_y: i64 = result.convert()?;

        // Should be scrolled to around where target element is (around 1000px)
        assert!(
            scroll_y >= 900 && scroll_y <= 1200,
            "Page should be scrolled to target element, got {}",
            scroll_y
        );

        driver.quit().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_actionable_move_to() -> UtamResult<()> {
        let driver = setup_driver().await?;
        setup_test_page(&driver).await?;

        let element = driver.find(By::Id("testButton")).await?;
        let actionable = ActionableElement::new(element.clone());

        // Move to element (hover)
        actionable.move_to().await?;

        // Note: Verifying hover state is tricky without actual hover styles
        // This test at least verifies the move_to method executes without error

        driver.quit().await?;
        Ok(())
    }
}
