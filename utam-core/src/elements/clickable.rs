//! ClickableElement - wrapper implementing Clickable trait

use async_trait::async_trait;
use thirtyfour::WebElement;

use crate::elements::BaseElement;
use crate::traits::{Actionable, Clickable};

/// Element wrapper for clickable elements
#[derive(Debug, Clone)]
pub struct ClickableElement {
    base: BaseElement,
}

impl ClickableElement {
    /// Create a new ClickableElement from a WebElement
    pub fn new(element: WebElement) -> Self {
        Self { base: BaseElement::new(element) }
    }

    /// Get the underlying WebElement
    pub fn inner(&self) -> &WebElement {
        self.base.inner()
    }
}

#[async_trait]
impl Actionable for ClickableElement {
    fn inner(&self) -> &WebElement {
        self.base.inner()
    }
}

#[async_trait]
impl Clickable for ClickableElement {}
