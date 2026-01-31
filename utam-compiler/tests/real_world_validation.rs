//! Test validation against real-world UTAM files

use utam_compiler::validator::SchemaValidator;
use std::fs;

#[test]
fn test_org_details_validation() {
    let validator = SchemaValidator::new().unwrap();
    let json_str = fs::read_to_string("../applications/orgDetails.utam.json")
        .expect("Failed to read orgDetails.utam.json");
    
    let result = validator.validate_str(&json_str);
    match &result {
        Ok(_) => println!("✓ orgDetails.utam.json is valid"),
        Err(e) => println!("✗ orgDetails.utam.json validation failed: {}", e),
    }
    
    assert!(result.is_ok(), "orgDetails.utam.json should be valid");
}

#[test]
fn test_salesforce_studio_app_validation() {
    let validator = SchemaValidator::new().unwrap();
    let json_str = fs::read_to_string("../applications/salesforceStudioApp.utam.json")
        .expect("Failed to read salesforceStudioApp.utam.json");
    
    let result = validator.validate_str(&json_str);
    match &result {
        Ok(_) => println!("✓ salesforceStudioApp.utam.json is valid"),
        Err(e) => println!("✗ salesforceStudioApp.utam.json validation failed: {}", e),
    }
    
    assert!(result.is_ok(), "salesforceStudioApp.utam.json should be valid");
}
