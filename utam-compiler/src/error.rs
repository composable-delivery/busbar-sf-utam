//! Error types for the UTAM compiler

use miette::Diagnostic;
use thiserror::Error;

/// Result type for compiler operations
pub type CompilerResult<T> = Result<T, CompilerError>;

/// Main error type for the UTAM compiler
#[derive(Error, Debug, Diagnostic)]
pub enum CompilerError {
    /// JSON parsing error
    #[error("Failed to parse JSON: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// Schema compilation error
    #[error("Failed to compile JSON schema: {0}")]
    SchemaCompilation(String),

    /// Schema validation errors
    #[error("Schema validation failed with {} error(s):\n{}", .0.len(), format_validation_errors(.0))]
    #[diagnostic(help("Check the validation errors for details"))]
    SchemaValidation(Vec<ValidationError>),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic compilation error
    #[error("Compilation error: {0}")]
    Compilation(String),
}

/// Detailed validation error with path and message
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// JSON path where the error occurred
    pub path: String,
    /// Human-readable error message
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "at '{}': {}", self.path, self.message)
        }
    }
}

/// Format multiple validation errors for display
fn format_validation_errors(errors: &[ValidationError]) -> String {
    errors
        .iter()
        .enumerate()
        .map(|(i, e)| format!("  {}. {}", i + 1, e))
        .collect::<Vec<_>>()
        .join("\n")
}
