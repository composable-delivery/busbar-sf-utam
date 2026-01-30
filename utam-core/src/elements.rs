//! Element wrapper types for UTAM
//!
//! This module provides wrapper types for WebElements that implement
//! various traits for type-safe element interactions.

use crate::traits::{Actionable, Clickable};
use thirtyfour::WebElement;

/// A wrapper around WebElement that implements the Clickable trait.
///
/// This struct provides a type-safe way to interact with clickable elements,
/// ensuring that only click-related operations are available.
///
/// # Examples
///
/// ```rust,ignore
/// let element = driver.find(By::Css("button")).await?;
/// let clickable = ClickableElement::new(element);
/// clickable.click().await?;
/// ```
#[derive(Debug, Clone)]
pub struct ClickableElement {
    element: WebElement,
}

impl ClickableElement {
    /// Creates a new ClickableElement wrapping the given WebElement.
    ///
    /// # Arguments
    ///
    /// * `element` - The WebElement to wrap
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let element = driver.find(By::Css("button")).await?;
    /// let clickable = ClickableElement::new(element);
    /// ```
    pub fn new(element: WebElement) -> Self {
        Self { element }
    }

    /// Consumes the wrapper and returns the underlying WebElement.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let clickable = ClickableElement::new(element);
    /// let element = clickable.into_inner();
    /// ```
    pub fn into_inner(self) -> WebElement {
        self.element
    }
}

impl Actionable for ClickableElement {
    fn inner(&self) -> &WebElement {
        &self.element
    }
}

impl Clickable for ClickableElement {}
