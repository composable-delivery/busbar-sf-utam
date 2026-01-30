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
}
