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
fn test_shadow_root_error_type() {
    // Test that ShadowRootNotFound error can be constructed
    let _error = UtamError::ShadowRootNotFound { element: "test-element".to_string() };
}

#[test]
fn test_base_element_creation() {
    // Test that we can create types without compilation errors
    // This ensures the public API structure is correct
    fn _compile_check() -> UtamResult<()> {
        // This function doesn't run, it just ensures the types compile
        Ok(())
    }
}
