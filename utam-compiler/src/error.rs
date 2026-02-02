//! Error types for the UTAM compiler

use miette::{Diagnostic, NamedSource, SourceSpan};
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

    /// Invalid element type error
    #[error("Invalid element type")]
    #[diagnostic(
        code(utam::invalid_element_type),
        help("Element type must be an array of action types, a component path, 'container', or 'frame'")
    )]
    InvalidElementType {
        #[source_code]
        src: NamedSource<String>,
        #[label("this element type is invalid")]
        span: SourceSpan,
    },

    /// Unknown action type error
    #[error("Unknown action type '{action}'")]
    #[diagnostic(
        code(utam::unknown_action),
        help("Valid action types are: actionable, clickable, editable, draggable, touchable")
    )]
    UnknownActionType {
        action: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("unknown action type")]
        span: SourceSpan,
    },

    /// Selector parameter mismatch error
    #[error("Selector parameter mismatch: expected {expected}, found {actual}")]
    #[diagnostic(
        code(utam::selector_params),
        help("Ensure the number of args matches the number of %s/%d placeholders in the selector")
    )]
    SelectorParameterMismatch {
        expected: usize,
        actual: usize,
        #[source_code]
        src: NamedSource<String>,
        #[label("selector with {expected} placeholder(s)")]
        span: SourceSpan,
    },
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

/// Error reporter for formatting compiler errors
///
/// Provides both human-readable terminal output and machine-readable JSON format.
#[allow(dead_code)]
pub struct ErrorReporter {
    source: String,
    file_path: String,
}

impl ErrorReporter {
    /// Create a new error reporter
    ///
    /// # Arguments
    ///
    /// * `source` - The source code being compiled
    /// * `file_path` - Path to the source file
    pub fn new(source: String, file_path: String) -> Self {
        Self { source, file_path }
    }

    /// Report an error to stderr with colorized output
    ///
    /// Uses miette's fancy formatting for terminal output with colors,
    /// source snippets, and helpful diagnostic information.
    ///
    /// # Arguments
    ///
    /// * `error` - The compiler error to report
    pub fn report(&self, error: &CompilerError) {
        use miette::{GraphicalReportHandler, GraphicalTheme};

        // Create a graphical report handler with fancy theme
        let mut output = String::new();
        let handler =
            GraphicalReportHandler::new_themed(GraphicalTheme::unicode()).with_width(80);

        // Format the error using miette's fancy formatting
        if let Err(e) = handler.render_report(&mut output, error) {
            eprintln!("Error formatting diagnostic: {}", e);
            eprintln!("{:?}", error);
        } else {
            eprintln!("{}", output);
        }
    }

    /// Generate machine-readable JSON format for errors
    ///
    /// Produces a JSON array with error information suitable for
    /// programmatic consumption by tools and IDEs.
    ///
    /// # Arguments
    ///
    /// * `errors` - Slice of compiler errors to format
    ///
    /// # Returns
    ///
    /// A JSON string representing the errors
    pub fn report_json(&self, errors: &[CompilerError]) -> String {
        let error_objects: Vec<serde_json::Value> = errors
            .iter()
            .map(|e| {
                serde_json::json!({
                    "file": self.file_path,
                    "message": e.to_string(),
                    "code": e.code().map(|c| c.to_string()),
                })
            })
            .collect();

        serde_json::to_string_pretty(&error_objects)
            .unwrap_or_else(|_| "[]".to_string())
    }
}
