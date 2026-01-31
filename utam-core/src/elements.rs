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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_element_new() {
        // This test verifies BaseElement::new is callable
        // Actual construction requires WebElement which needs WebDriver
        // So we just verify the function signature compiles
        fn _test_signature(_element: WebElement) -> BaseElement {
            BaseElement::new(_element)
        }
    }

    #[test]
    fn test_base_element_is_debug() {
        // Verify Debug trait is implemented
        fn _assert_debug<T: std::fmt::Debug>() {}
        _assert_debug::<BaseElement>();
    }

    #[test]
    fn test_base_element_is_clone() {
        // Verify Clone trait is implemented
        fn _assert_clone<T: Clone>() {}
        _assert_clone::<BaseElement>();
    }

    #[test]
    fn test_actionable_element_new() {
        // This test verifies ActionableElement::new is callable
        fn _test_signature(_element: WebElement) -> ActionableElement {
            ActionableElement::new(_element)
        }
    }

    #[test]
    fn test_actionable_element_is_debug() {
        // Verify Debug trait is implemented
        fn _assert_debug<T: std::fmt::Debug>() {}
        _assert_debug::<ActionableElement>();
    }

    #[test]
    fn test_actionable_element_is_clone() {
        // Verify Clone trait is implemented
        fn _assert_clone<T: Clone>() {}
        _assert_clone::<ActionableElement>();
    }

    #[test]
    fn test_actionable_element_implements_actionable() {
        // Verify ActionableElement implements Actionable trait
        fn _assert_actionable<T: Actionable>() {}
        _assert_actionable::<ActionableElement>();
    }

    #[test]
    fn test_actionable_element_is_send() {
        // Verify Send bound
        fn _assert_send<T: Send>() {}
        _assert_send::<ActionableElement>();
    }

    #[test]
    fn test_actionable_element_is_sync() {
        // Verify Sync bound
        fn _assert_sync<T: Sync>() {}
        _assert_sync::<ActionableElement>();
    }
}
