//! Integration tests for UTAM core runtime
//!
//! Tests runtime traits and element wrappers.

mod common;

use std::time::Duration;
use utam_core::prelude::*;

#[test]
fn test_error_types() {
    // Test that error types can be constructed and displayed
    let error1 = UtamError::ElementNotFound {
        name: "testButton".to_string(),
        selector: ".test".to_string(),
    };
    assert!(error1.to_string().contains("testButton"));
    assert!(error1.to_string().contains(".test"));

    let error2 = UtamError::Timeout { condition: "element to be visible".to_string() };
    assert!(error2.to_string().contains("Timeout"));

    let error3 = UtamError::ShadowRootNotFound { element: "myElement".to_string() };
    assert!(error3.to_string().contains("myElement"));

    let error4 = UtamError::InvalidSelector { selector: "invalid[".to_string() };
    assert!(error4.to_string().contains("invalid["));

    let error5 = UtamError::FrameNotFound { name: "myFrame".to_string() };
    assert!(error5.to_string().contains("myFrame"));

    let error6 =
        UtamError::AssertionFailed { expected: "foo".to_string(), actual: "bar".to_string() };
    assert!(error6.to_string().contains("foo"));
    assert!(error6.to_string().contains("bar"));
}

#[test]
fn test_prelude_exports() {
    // Test that all expected types are exported from prelude
    // This ensures the public API is stable
    let _result: UtamResult<()> = Ok(());
}

#[test]
fn test_element_wrappers_compile() {
    // Test that element wrapper types can be constructed
    // This ensures the API is stable and types are exported

    // These would normally be created from actual WebElements,
    // but we're just testing that the types compile and are accessible

    // Note: We can't actually instantiate these without a WebDriver,
    // but we can verify the types are exported and have the expected API
    fn _check_base_element(_elem: BaseElement) {}
    fn _check_clickable_element(_elem: ClickableElement) {}
    fn _check_editable_element(_elem: EditableElement) {}
    fn _check_draggable_element(_elem: DraggableElement) {}
}

#[test]
fn test_traits_compile() {
    // Test that trait bounds compile
    // This ensures traits are exported and have the correct signatures

    fn _check_actionable<T: Actionable>(_elem: T) {}
    fn _check_clickable<T: Clickable>(_elem: T) {}
    fn _check_editable<T: Editable>(_elem: T) {}
    fn _check_draggable<T: Draggable>(_elem: T) {}
}

#[test]
fn test_draggable_method_signatures() {
    // Verify that Draggable trait methods have the correct signatures
    // This is a compile-time test to ensure API stability

    async fn _test_drag_and_drop<T: Draggable>(elem: &T, target: &WebElement) -> UtamResult<()> {
        elem.drag_and_drop(target).await
    }

    async fn _test_drag_and_drop_with_duration<T: Draggable>(
        elem: &T,
        target: &WebElement,
        duration: Duration,
    ) -> UtamResult<()> {
        elem.drag_and_drop_with_duration(target, duration).await
    }

    async fn _test_drag_and_drop_by_offset<T: Draggable>(
        elem: &T,
        x: i64,
        y: i64,
    ) -> UtamResult<()> {
        elem.drag_and_drop_by_offset(x, y).await
    }
}

#[test]
fn test_actionable_method_signatures() {
    // Verify that Actionable trait methods have the correct signatures

    async fn _test_focus<T: Actionable>(elem: &T) -> UtamResult<()> {
        elem.focus().await
    }

    async fn _test_blur<T: Actionable>(elem: &T) -> UtamResult<()> {
        elem.blur().await
    }

    async fn _test_scroll_into_view<T: Actionable>(elem: &T) -> UtamResult<()> {
        elem.scroll_into_view().await
    }

    async fn _test_move_to<T: Actionable>(elem: &T) -> UtamResult<()> {
        elem.move_to().await
    }
}

#[test]
fn test_clickable_method_signatures() {
    // Verify that Clickable trait methods have the correct signatures

    async fn _test_click<T: Clickable>(elem: &T) -> UtamResult<()> {
        elem.click().await
    }

    async fn _test_double_click<T: Clickable>(elem: &T) -> UtamResult<()> {
        elem.double_click().await
    }

    async fn _test_right_click<T: Clickable>(elem: &T) -> UtamResult<()> {
        elem.right_click().await
    }

    async fn _test_click_and_hold<T: Clickable>(elem: &T) -> UtamResult<()> {
        elem.click_and_hold().await
    }
}

#[test]
fn test_editable_method_signatures() {
    // Verify that Editable trait methods have the correct signatures

    async fn _test_clear<T: Editable>(elem: &T) -> UtamResult<()> {
        elem.clear().await
    }

    async fn _test_clear_and_type<T: Editable>(elem: &T, text: &str) -> UtamResult<()> {
        elem.clear_and_type(text).await
    }

    async fn _test_set_text<T: Editable>(elem: &T, text: &str) -> UtamResult<()> {
        elem.set_text(text).await
    }

    async fn _test_press<T: Editable>(elem: &T, key: &str) -> UtamResult<()> {
        elem.press(key).await
fn test_error_messages_are_human_readable() {
    // Test ElementNotFound error message
    let error = UtamError::ElementNotFound {
        name: "submitButton".to_string(),
        selector: "button[type='submit']".to_string(),
    };
    let msg = error.to_string();
    assert!(msg.contains("submitButton"));
    assert!(msg.contains("button[type='submit']"));
    assert!(msg.contains("not found"));

    // Test Timeout error message
    let error = UtamError::Timeout { condition: "element to be visible".to_string() };
    let msg = error.to_string();
    assert!(msg.contains("Timeout"));
    assert!(msg.contains("element to be visible"));

    // Test ShadowRootNotFound error message
    let error = UtamError::ShadowRootNotFound { element: "customElement".to_string() };
    let msg = error.to_string();
    assert!(msg.contains("Shadow root not found"));
    assert!(msg.contains("customElement"));

    // Test InvalidSelector error message
    let error = UtamError::InvalidSelector { selector: "invalid>>selector".to_string() };
    let msg = error.to_string();
    assert!(msg.contains("Invalid selector"));
    assert!(msg.contains("invalid>>selector"));

    // Test FrameNotFound error message
    let error = UtamError::FrameNotFound { name: "paymentFrame".to_string() };
    let msg = error.to_string();
    assert!(msg.contains("Frame not found"));
    assert!(msg.contains("paymentFrame"));

    // Test AssertionFailed error message
    let error = UtamError::AssertionFailed {
        expected: "visible".to_string(),
        actual: "hidden".to_string(),
    };
    let msg = error.to_string();
    assert!(msg.contains("Assertion failed"));
    assert!(msg.contains("expected visible"));
    assert!(msg.contains("got hidden"));
}

#[test]
fn test_webdriver_error_conversion() {
    // Test that WebDriver errors can be converted to UtamError
    use thirtyfour::error::WebDriverError;

    // Create a WebDriver error (using ParseError variant which takes a String)
    let wd_error = WebDriverError::ParseError("test parse error".to_string());

    // Convert it to UtamError
    let utam_error: UtamError = wd_error.into();

    // Verify it's the WebDriver variant
    let msg = utam_error.to_string();
    assert!(msg.contains("WebDriver error"));
    assert!(msg.contains("parse error"));
}

#[test]
fn test_error_context_preserved() {
    // Test that error context (name, selector, etc.) is preserved
    let name = "loginButton".to_string();
    let selector = "#login-btn".to_string();

    let error = UtamError::ElementNotFound { name: name.clone(), selector: selector.clone() };

    // Verify context is accessible via display
    let msg = error.to_string();
    assert!(msg.contains(&name));
    assert!(msg.contains(&selector));

    // Test UtamResult type alias works correctly
    let result: UtamResult<String> = Err(error);
    assert!(result.is_err());

    if let Err(e) = result {
        let msg = e.to_string();
        assert!(msg.contains("loginButton"));
        assert!(msg.contains("#login-btn"));
    }
}
