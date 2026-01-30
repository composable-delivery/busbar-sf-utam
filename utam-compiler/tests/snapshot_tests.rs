//! Snapshot tests for UTAM compiler code generation
//!
//! Uses insta for snapshot testing of generated Rust code.

mod common;

use common::*;

#[test]
fn snapshot_simple_element() {
    let code = compile_fixture("basic/simple-element.utam.json")
        .expect("Failed to compile simple-element");
    insta::assert_snapshot!("simple_element", code);
}

#[test]
fn snapshot_clickable_button() {
    let code = compile_fixture("basic/clickable-button.utam.json")
        .expect("Failed to compile clickable-button");
    insta::assert_snapshot!("clickable_button", code);
}

#[test]
fn snapshot_editable_input() {
    let code = compile_fixture("basic/editable-input.utam.json")
        .expect("Failed to compile editable-input");
    insta::assert_snapshot!("editable_input", code);
}

#[test]
fn snapshot_shadow_root() {
    let code = compile_fixture("shadow-dom/shadow-root.utam.json")
        .expect("Failed to compile shadow-root");
    insta::assert_snapshot!("shadow_root", code);
}

#[test]
fn snapshot_nested_shadow() {
    let code = compile_fixture("shadow-dom/nested-shadow.utam.json")
        .expect("Failed to compile nested-shadow");
    insta::assert_snapshot!("nested_shadow", code);
}

#[test]
fn snapshot_simple_method() {
    let code = compile_fixture("compose/simple-method.utam.json")
        .expect("Failed to compile simple-method");
    insta::assert_snapshot!("simple_method", code);
}

#[test]
fn snapshot_chained_method() {
    let code = compile_fixture("compose/chained-method.utam.json")
        .expect("Failed to compile chained-method");
    insta::assert_snapshot!("chained_method", code);
}

#[test]
fn snapshot_filter_method() {
    let code = compile_fixture("compose/filter-method.utam.json")
        .expect("Failed to compile filter-method");
    insta::assert_snapshot!("filter_method", code);
}
