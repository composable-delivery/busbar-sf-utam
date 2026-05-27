//! Error types for the UTAM runtime interpreter

use thiserror::Error;
use utam_core::error::UtamError;

/// Errors that can occur during runtime interpretation
#[derive(Debug, Error)]
pub enum RuntimeError {
    /// Page object not found in registry
    #[error("Page object not found: {name}")]
    PageObjectNotFound { name: String },

    /// Method not found on page object
    #[error("Method '{method}' not found on page object '{page_object}'")]
    MethodNotFound { page_object: String, method: String },

    /// Element not defined in page object
    #[error("Element '{element}' not defined in page object '{page_object}'")]
    ElementNotDefined { page_object: String, element: String },

    /// Nullable element was not present in the DOM (expected absence).
    ///
    /// The UTAM `"nullable": true` flag on an element means "this element
    /// may or may not exist; the method should handle absence gracefully."
    /// Compose execution catches this error, sets the current result to
    /// Null, and continues with subsequent statements.
    #[error("Nullable element '{element}' is absent from the DOM")]
    NullableAbsent { element: String },

    /// A required element couldn't be found in the DOM.
    ///
    /// Produced when a scope's shadow root is missing but expected, or
    /// when a selector matches nothing in the search scope.  Reported as
    /// a "not found" failure (classified as StaleSelector by tests)
    /// rather than an architectural error.
    #[error("Element '{element}' not found in DOM: {reason}")]
    ElementNotFound { element: String, reason: String },

    /// Action not supported for this element type
    #[error("Action '{action}' not supported for element type '{element_type}'")]
    UnsupportedAction { action: String, element_type: String },

    /// Required argument missing
    #[error("Method '{method}' requires argument '{arg_name}'")]
    ArgumentMissing { method: String, arg_name: String },

    /// Argument type mismatch
    #[error("Argument type mismatch: expected {expected}, got {actual}")]
    ArgumentTypeMismatch { expected: String, actual: String },

    /// Underlying UTAM error
    #[error(transparent)]
    Utam(#[from] UtamError),

    /// JSON deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for runtime operations
pub type RuntimeResult<T> = Result<T, RuntimeError>;
