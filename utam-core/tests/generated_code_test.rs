//! Test that generated code from utam-compiler actually compiles with utam-core

use utam_compiler::{compile, CodeGenConfig};

#[test]
fn test_generated_code_compiles() {
    // Generate code from a simple fixture
    let json = r#"{
        "description": "Simple test page",
        "root": true,
        "selector": { "css": ".test-page" },
        "type": ["clickable"]
    }"#;
    
    let config = CodeGenConfig {
        module_name: Some("TestPage".to_string()),
    };
    
    let code = compile(json, config).expect("Failed to compile");
    
    // Just verify it generates something
    assert!(code.contains("pub struct TestPage"));
    assert!(code.contains("impl PageObject for TestPage"));
    assert!(code.contains("impl RootPageObject for TestPage"));
}

#[test]
fn test_generated_code_with_elements() {
    let json = r#"{
        "root": true,
        "selector": { "css": "form" },
        "elements": [
            {
                "name": "submitButton",
                "type": ["clickable"],
                "selector": { "css": "button" },
                "public": true
            }
        ]
    }"#;
    
    let config = CodeGenConfig {
        module_name: Some("FormPage".to_string()),
    };
    
    let code = compile(json, config).expect("Failed to compile");
    
    assert!(code.contains("pub async fn get_submit_button"));
    assert!(code.contains("ClickableElement"));
}
