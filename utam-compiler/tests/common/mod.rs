//! Test utilities for UTAM compiler integration tests
//!
//! Provides common helpers for testing UTAM JSON compilation.

use std::path::Path;
use utam_compiler::{compile, CodeGenConfig, CompilerResult};

/// Load a test fixture from the testdata directory
pub fn load_fixture(path: &str) -> String {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("testdata")
        .join(path);
    std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to load fixture {}: {}", fixture_path.display(), e))
}

/// Compile a test fixture and return the result
pub fn compile_fixture(path: &str) -> CompilerResult<String> {
    let json = load_fixture(path);
    // Extract module name from path (e.g., "basic/simple-element.utam.json" -> "SimpleElement")
    let module_name = extract_module_name(path);
    let config = CodeGenConfig {
        module_name: Some(module_name),
    };
    compile(&json, config)
}

/// Extract module name from fixture path
fn extract_module_name(path: &str) -> String {
    let filename = Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("PageObject");
    
    // Remove .utam suffix if present
    let name = filename.strip_suffix(".utam").unwrap_or(filename);
    
    // Convert to PascalCase
    to_pascal_case(name)
}

/// Convert string to PascalCase
fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;
    
    for ch in s.chars() {
        if ch == '_' || ch == '-' || ch == '/' || ch == '.' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    
    result
}

/// Assert that compilation succeeds for a given fixture
#[track_caller]
pub fn assert_compiles(path: &str) {
    match compile_fixture(path) {
        Ok(_code) => {},
        Err(e) => panic!("Expected fixture {} to compile successfully, but got error: {}", path, e),
    }
}

/// Assert that compilation fails for a given fixture
#[track_caller]
pub fn assert_fails_to_compile(path: &str) {
    match compile_fixture(path) {
        Ok(_) => panic!("Expected fixture {} to fail compilation, but it succeeded", path),
        Err(_) => {},
    }
}

/// Assert that compilation produces specific error
#[track_caller]
pub fn assert_compile_error_contains(path: &str, expected_msg: &str) {
    match compile_fixture(path) {
        Ok(_) => panic!("Expected fixture {} to fail compilation, but it succeeded", path),
        Err(e) => {
            let error_msg = format!("{}", e);
            assert!(
                error_msg.contains(expected_msg),
                "Expected error message to contain '{}', but got: {}",
                expected_msg,
                error_msg
            );
        }
    }
}
