//! Element wrappers for different interaction types
//!
//! This module provides wrapper structs that implement the various traits
//! for element interactions.

use async_trait::async_trait;
use thirtyfour::WebElement;

use crate::traits::{Actionable, Clickable, Draggable, Editable};

/// Base element wrapper that holds a WebElement
#[derive(Debug, Clone)]
pub struct BaseElement {
    element: WebElement,
}

impl BaseElement {
    /// Create a new BaseElement from a WebElement
    pub fn new(element: WebElement) -> Self {
        Self { element }
    }

    /// Get the underlying WebElement
    pub fn inner(&self) -> &WebElement {
        &self.element
    }
}

#[async_trait]
impl Actionable for BaseElement {
    fn inner(&self) -> &WebElement {
        &self.element
    }
}

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

/// Element wrapper for editable elements
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
