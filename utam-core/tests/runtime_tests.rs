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
fn test_error_timeout() {
    let error = UtamError::Timeout { condition: "element to be visible".to_string() };
    assert!(format!("{}", error).contains("Timeout"));
    assert!(format!("{}", error).contains("element to be visible"));
}

#[test]
fn test_error_shadow_root_not_found() {
    let error = UtamError::ShadowRootNotFound { element: "custom-element".to_string() };
    assert!(format!("{}", error).contains("Shadow root not found"));
    assert!(format!("{}", error).contains("custom-element"));
}

#[test]
fn test_error_invalid_selector() {
    let error = UtamError::InvalidSelector { selector: ":::invalid".to_string() };
    assert!(format!("{}", error).contains("Invalid selector"));
    assert!(format!("{}", error).contains(":::invalid"));
}

#[test]
fn test_error_frame_not_found() {
    let error = UtamError::FrameNotFound { name: "myFrame".to_string() };
    assert!(format!("{}", error).contains("Frame not found"));
    assert!(format!("{}", error).contains("myFrame"));
}

#[test]
fn test_error_assertion_failed() {
    let error = UtamError::AssertionFailed {
        expected: "visible".to_string(),
        actual: "hidden".to_string(),
    };
    assert!(format!("{}", error).contains("Assertion failed"));
    assert!(format!("{}", error).contains("visible"));
    assert!(format!("{}", error).contains("hidden"));
}

#[test]
fn test_prelude_exports() {
    // Test that all expected types are exported from prelude
    // This ensures the public API is stable
    let _result: UtamResult<()> = Ok(());
}

#[test]
fn test_element_rectangle_creation() {
    // Test ElementRectangle can be created and accessed
    let rect = ElementRectangle::new(10.0, 20.0, 100.0, 50.0);
    assert_eq!(rect.x, 10.0);
    assert_eq!(rect.y, 20.0);
    assert_eq!(rect.width, 100.0);
    assert_eq!(rect.height, 50.0);
}

#[test]
fn test_element_rectangle_from_thirtyfour_rect() {
    // Test conversion from thirtyfour's ElementRect
    use thirtyfour::ElementRect;

    let tf_rect = ElementRect { x: 5.0, y: 10.0, width: 200.0, height: 100.0 };

    let rect = ElementRectangle::from(tf_rect);
    assert_eq!(rect.x, 5.0);
    assert_eq!(rect.y, 10.0);
    assert_eq!(rect.width, 200.0);
    assert_eq!(rect.height, 100.0);
}

#[test]
fn test_base_element_api_exists() {
    // Test that BaseElement type is available
    // This is a compile-time check to ensure the API is exported

    // We can't create an actual BaseElement without a WebDriver,
    // but we can verify the type exists and is properly exported
    fn _check_api_exists() {
        #[allow(unreachable_code)]
        #[allow(clippy::diverging_sub_expression)]
        {
            let _element: BaseElement = panic!("not meant to run");
            let _ = _element.inner();
        }
    }
}

#[test]
fn test_utam_result_ok() {
    let result: UtamResult<i32> = Ok(42);
    assert!(result.is_ok());
    if let Ok(value) = result {
        assert_eq!(value, 42);
    }
}

#[test]
fn test_utam_result_err() {
    let result: UtamResult<()> =
        Err(UtamError::ElementNotFound { name: "test".to_string(), selector: ".test".to_string() });
    assert!(result.is_err());
}

#[test]
fn test_element_rectangle_partial_eq() {
    let rect1 = ElementRectangle::new(1.0, 2.0, 3.0, 4.0);
    let rect2 = ElementRectangle::new(1.0, 2.0, 3.0, 4.0);
    let rect3 = ElementRectangle::new(1.0, 2.0, 3.0, 5.0);

    assert_eq!(rect1, rect2);
    assert_ne!(rect1, rect3);
}

#[test]
fn test_element_rectangle_debug_impl() {
    let rect = ElementRectangle::new(10.0, 20.0, 30.0, 40.0);
    let debug = format!("{:?}", rect);
    assert!(debug.contains("ElementRectangle"));
}

#[test]
fn test_error_display_element_not_found() {
    let error = UtamError::ElementNotFound {
        name: "submitBtn".to_string(),
        selector: "#submit".to_string(),
    };
    let display = format!("{}", error);
    assert!(display.contains("submitBtn"));
    assert!(display.contains("#submit"));
    assert!(display.contains("not found"));
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
