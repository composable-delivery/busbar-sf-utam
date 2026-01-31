//! JSON schema validation for UTAM page objects

use crate::error::{CompilerError, CompilerResult, ValidationError};
use jsonschema::Validator;
use serde_json::Value;

/// Schema validator for UTAM page objects
///
/// Validates UTAM JSON files against the official JSON schema before parsing.
/// The schema is embedded at compile time for performance and reliability.
///
/// # Note on Default Implementation
///
/// The `Default` implementation will panic if the embedded schema cannot be compiled.
/// This should never happen in practice since the schema is validated at compile time,
/// but if it does occur, it indicates a bug in the schema definition itself.
pub struct SchemaValidator {
    validator: Validator,
}

impl SchemaValidator {
    /// Create a new schema validator
    ///
    /// Compiles the embedded UTAM JSON schema. This is relatively expensive,
    /// so the validator instance should be reused when validating multiple files.
    ///
    /// # Errors
    ///
    /// Returns `CompilerError::SchemaCompilation` if the embedded schema is invalid
    /// or cannot be compiled.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use utam_compiler::validator::SchemaValidator;
    ///
    /// let validator = SchemaValidator::new().expect("Failed to create validator");
    /// ```
    pub fn new() -> CompilerResult<Self> {
        let schema_json: Value = serde_json::from_str(include_str!("schema/utam-page-object.json"))
            .map_err(|e| {
                CompilerError::SchemaCompilation(format!("Failed to parse embedded schema: {}", e))
            })?;

        let validator = jsonschema::draft7::new(&schema_json)
            .map_err(|e| CompilerError::SchemaCompilation(e.to_string()))?;

        Ok(Self { validator })
    }

    /// Validate a JSON value against the UTAM schema
    ///
    /// # Arguments
    ///
    /// * `json` - The JSON value to validate
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if validation succeeds, or an error containing all
    /// validation failures if the JSON does not conform to the schema.
    ///
    /// # Errors
    ///
    /// Returns `CompilerError::SchemaValidation` with detailed error information
    /// if the JSON does not match the schema.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use utam_compiler::validator::SchemaValidator;
    /// use serde_json::json;
    ///
    /// let validator = SchemaValidator::new().unwrap();
    /// let json = json!({
    ///     "root": true,
    ///     "selector": { "css": ".button" }
    /// });
    ///
    /// validator.validate(&json).expect("Validation failed");
    /// ```
    pub fn validate(&self, json: &Value) -> CompilerResult<()> {
        if self.validator.is_valid(json) {
            return Ok(());
        }

        let validation_errors: Vec<ValidationError> = self.validator
            .iter_errors(json)
            .map(|e| ValidationError {
                path: e.instance_path().to_string(),
                message: e.to_string(),
            })
            .collect();

        if validation_errors.is_empty() {
            // This should never happen: if is_valid() returns false, iter_errors() should yield errors.
            // If we reach here, it indicates a bug in the jsonschema library or our usage of it.
            unreachable!(
                "Schema validation indicated errors but iter_errors() yielded none. \
                 This is a bug in the validation logic."
            );
        }

        Err(CompilerError::SchemaValidation(validation_errors))
    }

    /// Validate a JSON string against the UTAM schema
    ///
    /// This is a convenience method that parses the JSON string and then validates it.
    ///
    /// # Arguments
    ///
    /// * `json_str` - The JSON string to validate
    ///
    /// # Returns
    ///
    /// Returns the parsed JSON value if validation succeeds.
    ///
    /// # Errors
    ///
    /// Returns `CompilerError::JsonParse` if the string is not valid JSON,
    /// or `CompilerError::SchemaValidation` if the JSON does not match the schema.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use utam_compiler::validator::SchemaValidator;
    ///
    /// let validator = SchemaValidator::new().unwrap();
    /// let json_str = r#"{"root": true, "selector": {"css": ".button"}}"#;
    ///
    /// let value = validator.validate_str(json_str).expect("Validation failed");
    /// ```
    pub fn validate_str(&self, json_str: &str) -> CompilerResult<Value> {
        let json: Value = serde_json::from_str(json_str)?;
        self.validate(&json)?;
        Ok(json)
    }
}

// Note: Default implementation is intentionally not provided because schema
// compilation can fail (though it shouldn't with an embedded schema).
// Users should explicitly call SchemaValidator::new() and handle potential errors.

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validator_creation() {
        let result = SchemaValidator::new();
        assert!(result.is_ok(), "Should create validator successfully");
    }

    #[test]
    fn test_valid_minimal_page_object() {
        let validator = SchemaValidator::new().unwrap();
        let json = json!({
            "root": true,
            "selector": { "css": ".button" }
        });

        let result = validator.validate(&json);
        assert!(result.is_ok(), "Should validate minimal page object");
    }

    #[test]
    fn test_valid_page_object_with_type() {
        let validator = SchemaValidator::new().unwrap();
        let json = json!({
            "root": true,
            "selector": { "css": ".button" },
            "type": ["clickable"]
        });

        let result = validator.validate(&json);
        assert!(result.is_ok(), "Should validate page object with type");
    }

    #[test]
    fn test_missing_root_field() {
        let validator = SchemaValidator::new().unwrap();
        let json = json!({
            "selector": { "css": ".button" }
        });

        let result = validator.validate(&json);
        assert!(result.is_err(), "Should fail validation without root field");

        if let Err(CompilerError::SchemaValidation(errors)) = result {
            assert!(!errors.is_empty(), "Should have validation errors");
        } else {
            panic!("Expected SchemaValidation error");
        }
    }

    #[test]
    fn test_missing_selector_field() {
        let validator = SchemaValidator::new().unwrap();
        let json = json!({
            "root": true
        });

        let result = validator.validate(&json);
        assert!(result.is_err(), "Should fail validation without selector field");

        if let Err(CompilerError::SchemaValidation(errors)) = result {
            assert!(!errors.is_empty(), "Should have validation errors");
        } else {
            panic!("Expected SchemaValidation error");
        }
    }

    #[test]
    fn test_validate_str_valid() {
        let validator = SchemaValidator::new().unwrap();
        let json_str = r#"{"root": true, "selector": {"css": ".button"}}"#;

        let result = validator.validate_str(json_str);
        assert!(result.is_ok(), "Should validate valid JSON string");
    }

    #[test]
    fn test_validate_str_invalid_json() {
        let validator = SchemaValidator::new().unwrap();
        let json_str = r#"{"root": true, invalid json}"#;

        let result = validator.validate_str(json_str);
        assert!(result.is_err(), "Should fail with invalid JSON");

        match result {
            Err(CompilerError::JsonParse(_)) => {},
            _ => panic!("Expected JsonParse error"),
        }
    }

    #[test]
    fn test_validate_str_schema_violation() {
        let validator = SchemaValidator::new().unwrap();
        let json_str = r#"{"root": true}"#;

        let result = validator.validate_str(json_str);
        assert!(result.is_err(), "Should fail schema validation");

        match result {
            Err(CompilerError::SchemaValidation(_)) => {},
            _ => panic!("Expected SchemaValidation error"),
        }
    }

    #[test]
    fn test_element_with_invalid_name_pattern() {
        let validator = SchemaValidator::new().unwrap();
        let json = json!({
            "root": true,
            "selector": { "css": ".root" },
            "elements": [{
                "name": "123invalid",  // Names must start with letter or underscore
                "selector": { "css": ".elem" }
            }]
        });

        let result = validator.validate(&json);
        assert!(result.is_err(), "Should fail validation with invalid element name");
    }

    #[test]
    fn test_valid_shadow_dom_structure() {
        let validator = SchemaValidator::new().unwrap();
        let json = json!({
            "root": true,
            "selector": { "css": "my-component" },
            "shadow": {
                "elements": [{
                    "name": "innerButton",
                    "type": ["clickable"],
                    "selector": { "css": ".inner-btn" },
                    "public": true
                }]
            }
        });

        let result = validator.validate(&json);
        assert!(result.is_ok(), "Should validate shadow DOM structure");
    }

    #[test]
    fn test_valid_methods() {
        let validator = SchemaValidator::new().unwrap();
        let json = json!({
            "root": true,
            "selector": { "css": "login-form" },
            "shadow": {
                "elements": [{
                    "name": "submitButton",
                    "type": ["clickable"],
                    "selector": { "css": "button" }
                }]
            },
            "methods": [{
                "name": "clickSubmit",
                "compose": [{
                    "element": "submitButton",
                    "apply": "click"
                }]
            }]
        });

        let result = validator.validate(&json);
        assert!(result.is_ok(), "Should validate methods");
    }
}
