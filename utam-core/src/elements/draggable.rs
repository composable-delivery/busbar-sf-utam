//! DraggableElement - wrapper implementing Draggable trait

use async_trait::async_trait;
use thirtyfour::WebElement;

use crate::elements::BaseElement;
use crate::traits::{Actionable, Draggable};

/// Element wrapper for draggable elements
#[derive(Debug, Clone)]
pub struct DraggableElement {
    base: BaseElement,
}

impl DraggableElement {
    /// Create a new DraggableElement from a WebElement
    pub fn new(element: WebElement) -> Self {
        Self { base: BaseElement::new(element) }
    }

    /// Get the underlying WebElement
    pub fn inner(&self) -> &WebElement {
        self.base.inner()
    }
}

#[async_trait]
impl Actionable for DraggableElement {
    fn inner(&self) -> &WebElement {
        self.base.inner()
    }
}

#[async_trait]
impl Draggable for DraggableElement {}
