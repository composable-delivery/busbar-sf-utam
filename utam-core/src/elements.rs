//! Elements module - Element wrapper types
//!
//! This module provides wrapper types around WebElement that implement
//! various traits for user interactions.

use crate::traits::Actionable;
use async_trait::async_trait;
use thirtyfour::prelude::*;

/// Base element wrapper around WebElement
#[derive(Debug, Clone)]
pub struct BaseElement {
    inner: WebElement,
}

impl BaseElement {
    /// Create a new BaseElement wrapping a WebElement
    pub fn new(element: WebElement) -> Self {
        Self { inner: element }
    }

    /// Get a reference to the inner WebElement
    pub fn inner(&self) -> &WebElement {
        &self.inner
    }
}

/// Element wrapper that implements Actionable trait
#[derive(Debug, Clone)]
pub struct ActionableElement {
    base: BaseElement,
}

impl ActionableElement {
    /// Create a new ActionableElement
    pub fn new(element: WebElement) -> Self {
        Self { base: BaseElement::new(element) }
    }

    /// Get a reference to the base element
    pub fn base(&self) -> &BaseElement {
        &self.base
    }
}

#[async_trait]
impl Actionable for ActionableElement {
    fn inner(&self) -> &WebElement {
        self.base.inner()
    }
}
