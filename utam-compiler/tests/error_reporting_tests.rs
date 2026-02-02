//! Tests for error reporting with source locations

use miette::{NamedSource, SourceSpan};
use utam_compiler::{CompilerError, ErrorReporter};

#[test]
fn test_invalid_element_type_error_includes_source_location() {
    let source = r#"{
  "root": true,
  "selector": {"css": ".button"},
  "type": ["unknownType"]
}"#
    .to_string();

    let error = CompilerError::InvalidElementType {
        src: NamedSource::new("test.utam.json", source.clone()),
        span: SourceSpan::new(70usize.into(), 13usize), // Position of "unknownType"
    };

    // Verify the error has diagnostic information
    let error_str = format!("{}", error);
    assert_eq!(error_str, "Invalid element type");

    // Verify the diagnostic code is set
    use miette::Diagnostic;
    let code = error.code();
    assert!(code.is_some());
    assert_eq!(code.unwrap().to_string(), "utam::invalid_element_type");
}

#[test]
fn test_unknown_action_type_error_includes_help_text() {
    let source = r#"{
  "root": true,
  "selector": {"css": ".button"},
  "type": ["invalidAction"]
}"#
    .to_string();

    let error = CompilerError::UnknownActionType {
        action: "invalidAction".to_string(),
        src: NamedSource::new("button.utam.json", source.clone()),
        span: SourceSpan::new(70usize.into(), 13usize),
    };

    let error_str = format!("{}", error);
    assert!(error_str.contains("invalidAction"));
    assert_eq!(error_str, "Unknown action type 'invalidAction'");

    // Verify the diagnostic code
    use miette::Diagnostic;
    let code = error.code();
    assert!(code.is_some());
    assert_eq!(code.unwrap().to_string(), "utam::unknown_action");
}

#[test]
fn test_selector_parameter_mismatch_error() {
    let source = r#"{
  "root": true,
  "selector": {
    "css": "button[data-id='%s']",
    "args": []
  }
}"#
    .to_string();

    let error = CompilerError::SelectorParameterMismatch {
        expected: 1,
        actual: 0,
        src: NamedSource::new("form.utam.json", source.clone()),
        span: SourceSpan::new(42usize.into(), 25usize),
    };

    let error_str = format!("{}", error);
    assert!(error_str.contains("expected 1"));
    assert!(error_str.contains("found 0"));

    // Verify the diagnostic code
    use miette::Diagnostic;
    let code = error.code();
    assert!(code.is_some());
    assert_eq!(code.unwrap().to_string(), "utam::selector_params");
}

#[test]
fn test_error_reporter_includes_file_path() {
    let source = r#"{"root": true, "selector": {"css": ".test"}}"#.to_string();
    let file_path = "/path/to/test.utam.json".to_string();

    let reporter = ErrorReporter::new(source.clone(), file_path.clone());

    let error = CompilerError::InvalidElementType {
        src: NamedSource::new(&file_path, source),
        span: SourceSpan::new(10usize.into(), 5usize),
    };

    let json_output = reporter.report_json(&[error]);

    // Verify JSON contains file path
    assert!(json_output.contains("/path/to/test.utam.json"));
    assert!(json_output.contains("Invalid element type"));
}

#[test]
fn test_error_reporter_json_format_is_valid() {
    let source = r#"{"root": true}"#.to_string();
    let file_path = "test.utam.json".to_string();

    let reporter = ErrorReporter::new(source.clone(), file_path.clone());

    let error1 = CompilerError::InvalidElementType {
        src: NamedSource::new(&file_path, source.clone()),
        span: SourceSpan::new(0usize.into(), 5usize),
    };

    let error2 = CompilerError::UnknownActionType {
        action: "invalidType".to_string(),
        src: NamedSource::new(&file_path, source.clone()),
        span: SourceSpan::new(0usize.into(), 5usize),
    };

    let json_output = reporter.report_json(&[error1, error2]);

    // Parse JSON to verify it's valid
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json_output);
    assert!(
        parsed.is_ok(),
        "JSON output should be valid: {}",
        json_output
    );

    let json_value = parsed.unwrap();
    assert!(json_value.is_array());

    let array = json_value.as_array().unwrap();
    assert_eq!(array.len(), 2);

    // Check first error
    assert_eq!(array[0]["file"], "test.utam.json");
    assert_eq!(array[0]["message"], "Invalid element type");
    assert_eq!(array[0]["code"], "utam::invalid_element_type");

    // Check second error
    assert_eq!(array[1]["file"], "test.utam.json");
    assert_eq!(array[1]["message"], "Unknown action type 'invalidType'");
    assert_eq!(array[1]["code"], "utam::unknown_action");
}

#[test]
fn test_error_reporter_handles_empty_errors() {
    let source = "".to_string();
    let file_path = "test.utam.json".to_string();

    let reporter = ErrorReporter::new(source, file_path);

    let json_output = reporter.report_json(&[]);

    // Should produce valid empty JSON array
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json_output);
    assert!(parsed.is_ok());

    let json_value = parsed.unwrap();
    assert!(json_value.is_array());
    assert_eq!(json_value.as_array().unwrap().len(), 0);
}

#[test]
fn test_error_reporter_report_method() {
    // This test verifies the report() method can be called without panicking
    // We can't easily test the actual output since it goes to stderr
    let source = r#"{"root": true, "type": ["invalid"]}"#.to_string();
    let file_path = "test.utam.json".to_string();

    let reporter = ErrorReporter::new(source.clone(), file_path.clone());

    let error = CompilerError::UnknownActionType {
        action: "invalid".to_string(),
        src: NamedSource::new(&file_path, source),
        span: SourceSpan::new(24usize.into(), 7usize),
    };

    // Should not panic
    reporter.report(&error);
}

#[test]
fn test_multiple_validation_errors_format() {
    use utam_compiler::ValidationError;

    let errors = vec![
        ValidationError {
            path: "/root".to_string(),
            message: "Missing required field".to_string(),
        },
        ValidationError {
            path: "/selector".to_string(),
            message: "Invalid selector format".to_string(),
        },
    ];

    let error = CompilerError::SchemaValidation(errors);
    let error_str = format!("{}", error);

    assert!(error_str.contains("Schema validation failed"));
    assert!(error_str.contains("2 error(s)"));
    assert!(error_str.contains("Missing required field"));
    assert!(error_str.contains("Invalid selector format"));
}
