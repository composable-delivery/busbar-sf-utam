//! Example: Validating UTAM JSON files
//!
//! This example demonstrates how to use the SchemaValidator to validate
//! UTAM page object JSON files against the JSON schema.

use utam_compiler::validator::SchemaValidator;
use serde_json::json;

fn main() {
    println!("UTAM JSON Schema Validator Example\n");

    // Create a validator (this compiles the schema once)
    let validator = SchemaValidator::new().expect("Failed to create validator");
    println!("✓ Schema validator created successfully\n");

    // Example 1: Validate a minimal valid page object
    println!("Example 1: Minimal valid page object");
    let minimal_json = json!({
        "root": true,
        "selector": { "css": ".my-component" }
    });
    
    match validator.validate(&minimal_json) {
        Ok(_) => println!("✓ Valid: Minimal page object\n"),
        Err(e) => println!("✗ Invalid: {}\n", e),
    }

    // Example 2: Validate a page object with elements
    println!("Example 2: Page object with elements");
    let with_elements = json!({
        "root": true,
        "selector": { "css": "login-form" },
        "shadow": {
            "elements": [{
                "name": "usernameInput",
                "type": ["editable"],
                "selector": { "css": "input[name='username']" }
            }, {
                "name": "submitButton",
                "type": ["clickable"],
                "selector": { "css": "button[type='submit']" }
            }]
        }
    });
    
    match validator.validate(&with_elements) {
        Ok(_) => println!("✓ Valid: Page object with elements\n"),
        Err(e) => println!("✗ Invalid: {}\n", e),
    }

    // Example 3: Invalid - missing required field
    println!("Example 3: Invalid page object (missing selector)");
    let missing_selector = json!({
        "root": true,
        "type": ["clickable"]
    });
    
    match validator.validate(&missing_selector) {
        Ok(_) => println!("✗ This should have failed validation\n"),
        Err(e) => println!("✓ Correctly caught error:\n{}\n", e),
    }

    // Example 4: Invalid - bad element name
    println!("Example 4: Invalid element name pattern");
    let bad_name = json!({
        "root": true,
        "selector": { "css": ".root" },
        "elements": [{
            "name": "123invalid",  // Must start with letter or underscore
            "selector": { "css": ".elem" }
        }]
    });
    
    match validator.validate(&bad_name) {
        Ok(_) => println!("✗ This should have failed validation\n"),
        Err(e) => println!("✓ Correctly caught error:\n{}\n", e),
    }

    // Example 5: Validate from a JSON string
    println!("Example 5: Validating from a JSON string");
    let json_string = r#"{
        "root": true,
        "selector": { "css": "my-app" },
        "description": "Example application component",
        "type": ["clickable"]
    }"#;
    
    match validator.validate_str(json_string) {
        Ok(_) => println!("✓ Valid: JSON string parsed and validated successfully\n"),
        Err(e) => println!("✗ Invalid: {}\n", e),
    }

    println!("Examples completed!");
}
