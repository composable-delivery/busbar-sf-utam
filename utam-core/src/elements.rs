//! Element wrapper types
//!
//! This module provides wrapper types for WebElements that implement the
//! various traits defined in the traits module.

use crate::traits::{Actionable, Editable};
use async_trait::async_trait;
use thirtyfour::WebElement;

/// Wrapper for editable elements (text inputs, textareas, etc.)
///
/// This type implements the `Editable` trait, providing methods for
/// clearing, typing, and keyboard interactions.
///
/// # Examples
///
/// ```rust,ignore
/// let element = EditableElement::new(web_element);
/// element.clear_and_type("hello").await?;
/// element.press(Key::Enter).await?;
/// ```
pub struct EditableElement {
    element: WebElement,
}

impl EditableElement {
    /// Create a new EditableElement wrapper
    ///
    /// # Arguments
    ///
    /// * `element` - The WebElement to wrap
    pub fn new(element: WebElement) -> Self {
        Self { element }
    }

    /// Get a reference to the underlying WebElement
    pub fn element(&self) -> &WebElement {
        &self.element
    }
}

#[async_trait]
impl Actionable for EditableElement {
    fn inner(&self) -> &WebElement {
        &self.element
    }
}

#[async_trait]
impl Editable for EditableElement {
    // All methods are provided by the trait with default implementations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editable_element_construction() {
        // Test that we can construct an EditableElement
        // Note: We can't easily test with a real WebElement without a WebDriver,
        // but we can verify the type signature is correct
        // This would require a WebElement, so we'll just test the type exists
        let _type_check: Option<EditableElement> = None;
    }
}
