//! Integration test for error reporting in real usage scenarios

use utam_compiler::{ErrorReporter, SchemaValidator};

#[test]
fn test_validation_error_reporting() {
    // Test invalid JSON that fails schema validation
    let json_str = r#"{
  "root": true
}"#;

    let validator = SchemaValidator::new().unwrap();
    let result = validator.validate_str(json_str);

    assert!(result.is_err());

    if let Err(error) = result {
        // Demonstrate error reporting
        let reporter = ErrorReporter::new(json_str.to_string(), "test.utam.json".to_string());

        // Test JSON output
        let json_output = reporter.report_json(&[error]);
        assert!(json_output.contains("test.utam.json"));
        assert!(json_output.contains("validation"));
    }
}

#[test]
fn test_json_parse_error_reporting() {
    let invalid_json = r#"{ "root": true, invalid }"#;

    let validator = SchemaValidator::new().unwrap();
    let result = validator.validate_str(invalid_json);

    assert!(result.is_err());

    if let Err(error) = result {
        let reporter =
            ErrorReporter::new(invalid_json.to_string(), "invalid.utam.json".to_string());

        let json_output = reporter.report_json(&[error]);
        let parsed: serde_json::Value = serde_json::from_str(&json_output).unwrap();

        assert!(parsed.is_array());
        let array = parsed.as_array().unwrap();
        assert_eq!(array.len(), 1);
        assert_eq!(array[0]["file"], "invalid.utam.json");
        assert!(array[0]["message"]
            .as_str()
            .unwrap()
            .contains("Failed to parse JSON"));
    }
}
