//! Integration tests for UTAM compiler
//!
//! Tests compilation of various UTAM JSON fixtures to Rust code.

mod common;

use common::*;

#[test]
fn test_compile_simple_element() {
    assert_compiles("basic/simple-element.utam.json");
}

#[test]
fn test_compile_clickable_button() {
    assert_compiles("basic/clickable-button.utam.json");
}

#[test]
fn test_compile_editable_input() {
    assert_compiles("basic/editable-input.utam.json");
}

#[test]
fn test_compile_shadow_root() {
    assert_compiles("shadow-dom/shadow-root.utam.json");
}

#[test]
fn test_compile_nested_shadow() {
    assert_compiles("shadow-dom/nested-shadow.utam.json");
}

#[test]
fn test_compile_simple_method() {
    assert_compiles("compose/simple-method.utam.json");
}

#[test]
fn test_compile_chained_method() {
    assert_compiles("compose/chained-method.utam.json");
}

#[test]
fn test_compile_filter_method() {
    assert_compiles("compose/filter-method.utam.json");
}

#[test]
fn test_compile_salesforce_app() {
    assert_compiles("salesforce/salesforceStudioApp.utam.json");
}

#[test]
fn test_invalid_missing_selector() {
    // Missing selector should still parse, but won't generate valid code
    // The validator should catch this before code generation
    assert_compiles("invalid/missing-selector.utam.json");
}

#[test]
fn test_invalid_type() {
    // Invalid types should still parse and generate code
    // Type checking happens at Rust compile time
    assert_compiles("invalid/invalid-type.utam.json");
}
