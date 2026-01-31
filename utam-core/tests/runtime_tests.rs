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
fn test_key_enum_available() {
    // Test that Key enum is available and has expected variants
    let _enter = Key::Enter;
    let _tab = Key::Tab;
    let _escape = Key::Escape;
    let _backspace = Key::Backspace;
    let _delete = Key::Delete;
    let _arrow_up = Key::ArrowUp;
    let _arrow_down = Key::ArrowDown;
    let _arrow_left = Key::ArrowLeft;
    let _arrow_right = Key::ArrowRight;
    let _home = Key::Home;
    let _end = Key::End;
    let _page_up = Key::PageUp;
    let _page_down = Key::PageDown;
    let _space = Key::Space;
}

#[test]
fn test_traits_exported() {
    // Verify that traits are available in the prelude
    // This is a compile-time check
    fn _check_trait_bounds<T: Actionable>(_t: &T) {}
    fn _check_editable<T: Editable>(_t: &T) {}
}

#[test]
fn test_editable_element_type_available() {
    // Test that EditableElement type is available
    let _type_check: Option<EditableElement> = None;
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
