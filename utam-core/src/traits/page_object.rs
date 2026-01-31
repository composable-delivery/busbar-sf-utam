//! Core traits for UTAM page objects
//!
//! Defines the traits that all generated page objects implement.

use std::time::Duration;

use async_trait::async_trait;
use thirtyfour::{WebDriver, WebElement};

use crate::error::UtamResult;
use crate::wait::{wait_for, WaitConfig};

/// Trait implemented by all page objects
///
/// This trait provides access to the root element for any page object,
/// whether it's a root page object or a child component.
pub trait PageObject: Sized + Send + Sync {
    /// Get the root element of this page object
    ///
    /// The root element is the WebElement that represents this page object
    /// in the DOM. All other elements in this page object are descendants
    /// of the root element.
    fn root(&self) -> &WebElement;
}

/// Trait for page objects that can be loaded directly (root=true)
///
/// Root page objects have a root selector and can be loaded from the page
/// without requiring a parent element. They can also be constructed from
/// an existing element.
#[async_trait]
pub trait RootPageObject: PageObject {
    /// The CSS selector for the root element
    ///
    /// This selector is used by `load()` to find the page object's root
    /// element in the current page.
    const ROOT_SELECTOR: &'static str;

    /// Load the page object from the current page
    ///
    /// Finds the root element using `ROOT_SELECTOR` and constructs
    /// the page object.
    async fn load(driver: &WebDriver) -> UtamResult<Self>;

    /// Load with timeout for beforeLoad conditions
    ///
    /// Repeatedly attempts to load the page object until it succeeds
    /// or the timeout is reached.
    async fn wait_for_load(driver: &WebDriver, timeout: Duration) -> UtamResult<Self> {
        let config = WaitConfig { timeout, ..Default::default() };

        wait_for(
            || async {
                match Self::load(driver).await {
                    Ok(po) => Ok(Some(po)),
                    Err(_) => Ok(None),
                }
            },
            &config,
            &format!("page object with selector '{}' to load", Self::ROOT_SELECTOR),
        )
        .await
    }

    /// Construct from an existing element
    ///
    /// Creates a page object instance wrapping the provided element.
    /// This is useful when you already have a reference to the element
    /// from a parent page object.
    async fn from_element(element: WebElement) -> UtamResult<Self>;
}
