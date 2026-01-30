//! Error types for UTAM operations

use thiserror::Error;

/// Errors that can occur during UTAM operations
#[derive(Debug, Error)]
pub enum UtamError {
    /// Element was not found with the given selector
    #[error("Element '{name}' not found with selector: {selector}")]
    ElementNotFound { name: String, selector: String },

    /// Operation timed out
    #[error("Timeout waiting for condition: {condition}")]
    Timeout { condition: String },

    /// WebDriver operation failed
    #[error("WebDriver error: {0}")]
    WebDriver(#[from] thirtyfour::error::WebDriverError),

    /// Shadow root not found
    #[error("Shadow root not found for element: {element}")]
    ShadowRootNotFound { element: String },

    /// Invalid selector
    #[error("Invalid selector: {selector}")]
    InvalidSelector { selector: String },

    /// Frame not found
    #[error("Frame not found: {name}")]
    FrameNotFound { name: String },

    /// Assertion failed
    #[error("Assertion failed: expected {expected}, got {actual}")]
    AssertionFailed { expected: String, actual: String },
}

/// Result type for UTAM operations
pub type UtamResult<T> = Result<T, UtamError>;
