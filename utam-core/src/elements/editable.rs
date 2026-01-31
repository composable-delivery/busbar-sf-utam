//! EditableElement - wrapper implementing Editable trait

use async_trait::async_trait;
use thirtyfour::WebElement;

use crate::elements::BaseElement;
use crate::traits::{Actionable, Editable};

/// Element wrapper for editable elements (text inputs, textareas, etc.)
#[derive(Debug, Clone)]
pub struct EditableElement {
    base: BaseElement,
}

impl EditableElement {
    /// Create a new EditableElement from a WebElement
    pub fn new(element: WebElement) -> Self {
        Self { base: BaseElement::new(element) }
    }

    /// Get the underlying WebElement
    pub fn inner(&self) -> &WebElement {
        self.base.inner()
    }
}

#[async_trait]
impl Actionable for EditableElement {
    fn inner(&self) -> &WebElement {
        self.base.inner()
    }
}

#[async_trait]
impl Editable for EditableElement {}
