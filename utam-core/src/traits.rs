//! Traits module for page objects
//!
//! This module defines the core traits that page objects implement:
//! - `PageObject` - Base trait for all page objects
//! - `RootPageObject` - Trait for page objects that can be loaded from a driver

use crate::error::UtamResult;
use async_trait::async_trait;
use std::time::Duration;
use thirtyfour::{WebDriver, WebElement};

/// Base trait for all page objects
///
/// All page objects implement this trait, providing access to the root element.
pub trait PageObject: Sized + Send + Sync {
    /// Get the root WebElement for this page object
    fn root(&self) -> &WebElement;
}

/// Trait for page objects that can be loaded from a WebDriver
///
/// Root page objects can be loaded directly from a WebDriver instance using
/// a CSS selector defined at compile time.
#[async_trait]
pub trait RootPageObject: PageObject {
    /// The CSS selector used to locate this page object's root element
    const ROOT_SELECTOR: &'static str;

    /// Load this page object from a WebDriver
    ///
    /// # Arguments
    ///
    /// * `driver` - The WebDriver instance to use
    ///
    /// # Returns
    ///
    /// The loaded page object instance
    ///
    /// # Errors
    ///
    /// Returns an error if the root element cannot be found
    async fn load(driver: &WebDriver) -> UtamResult<Self>;

    /// Load this page object with a timeout
    ///
    /// Waits for the element to be present before loading.
    ///
    /// # Arguments
    ///
    /// * `driver` - The WebDriver instance to use
    /// * `timeout` - Maximum time to wait for the element
    ///
    /// # Returns
    ///
    /// The loaded page object instance
    ///
    /// # Errors
    ///
    /// Returns an error if the root element cannot be found within the timeout
    async fn wait_for_load(driver: &WebDriver, timeout: Duration) -> UtamResult<Self>;

    /// Construct this page object from an existing WebElement
    ///
    /// This is used by Container elements to wrap found elements as page objects.
    ///
    /// # Arguments
    ///
    /// * `element` - The WebElement to use as the root
    ///
    /// # Returns
    ///
    /// The page object instance
    async fn from_element(element: WebElement) -> UtamResult<Self>;
}
