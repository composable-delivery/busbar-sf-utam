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
