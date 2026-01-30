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
    
    let tf_rect = ElementRect {
        x: 5.0,
        y: 10.0,
        width: 200.0,
        height: 100.0,
    };
    
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
    let _ = || {
        #[allow(unreachable_code)]
        {
            let _element: BaseElement = panic!("not meant to run");
            let _ = _element.inner();
        }
    };
}
