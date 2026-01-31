//! Integration tests for schema validation

use utam_compiler::validator::SchemaValidator;
use std::fs;

#[test]
fn test_valid_simple_element() {
    let validator = SchemaValidator::new().unwrap();
    let json_str = fs::read_to_string("../testdata/basic/simple-element.utam.json")
        .expect("Failed to read test file");
    
    let result = validator.validate_str(&json_str);
    assert!(result.is_ok(), "simple-element.utam.json should be valid");
}

#[test]
fn test_valid_clickable_button() {
    let validator = SchemaValidator::new().unwrap();
    let json_str = fs::read_to_string("../testdata/basic/clickable-button.utam.json")
        .expect("Failed to read test file");
    
    let result = validator.validate_str(&json_str);
    assert!(result.is_ok(), "clickable-button.utam.json should be valid");
}

#[test]
fn test_valid_editable_input() {
    let validator = SchemaValidator::new().unwrap();
    let json_str = fs::read_to_string("../testdata/basic/editable-input.utam.json")
        .expect("Failed to read test file");
    
    let result = validator.validate_str(&json_str);
    assert!(result.is_ok(), "editable-input.utam.json should be valid");
}

#[test]
fn test_invalid_missing_selector() {
    let validator = SchemaValidator::new().unwrap();
    let json_str = fs::read_to_string("../testdata/invalid/missing-selector.utam.json")
        .expect("Failed to read test file");
    
    let result = validator.validate_str(&json_str);
    assert!(result.is_err(), "missing-selector.utam.json should be invalid");
    
    // Verify error message contains useful information
    if let Err(e) = result {
        let error_msg = format!("{}", e);
        assert!(error_msg.contains("selector") || error_msg.contains("required"), 
                "Error should mention missing selector: {}", error_msg);
    }
}

#[test]
fn test_custom_type_allowed() {
    let validator = SchemaValidator::new().unwrap();
    let json_str = fs::read_to_string("../testdata/valid-custom-types/custom-type.utam.json")
        .expect("Failed to read test file");
    
    // The schema allows any string values in the type array, including custom page object types
    // like "myCustomType" which could reference another UTAM page object.
    // Known basic types (actionable, clickable, editable, etc.) are documented but not enforced
    // by the schema since custom types are valid and can't be distinguished at schema level.
    let result = validator.validate_str(&json_str);
    assert!(result.is_ok(), 
            "Custom type names should be allowed as they can reference other page objects");
}

#[test]
fn test_valid_shadow_dom() {
    let validator = SchemaValidator::new().unwrap();
    let json_str = fs::read_to_string("../testdata/shadow-dom/shadow-root.utam.json")
        .expect("Failed to read test file");
    
    let result = validator.validate_str(&json_str);
    assert!(result.is_ok(), "shadow-root.utam.json should be valid");
}

#[test]
fn test_valid_compose_methods() {
    let validator = SchemaValidator::new().unwrap();
    let json_str = fs::read_to_string("../testdata/compose/simple-method.utam.json")
        .expect("Failed to read test file");
    
    let result = validator.validate_str(&json_str);
    assert!(result.is_ok(), "simple-method.utam.json should be valid");
}
