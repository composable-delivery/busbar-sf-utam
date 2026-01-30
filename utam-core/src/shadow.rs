//! Shadow DOM support for UTAM
//!
//! This module provides types and functions for working with Shadow DOM,
//! including traversing nested shadow roots.

use crate::error::{UtamError, UtamResult};
use thirtyfour::prelude::*;

/// Wrapper around WebDriver's shadow root (represented as WebElement) providing UTAM-specific functionality
pub struct ShadowRoot {
    inner: WebElement,
}

impl ShadowRoot {
    /// Create a new ShadowRoot wrapper
    pub fn new(inner: WebElement) -> Self {
        Self { inner }
    }

    /// Find a single element within the shadow root
    ///
    /// # Arguments
    ///
    /// * `by` - The selector to use for finding the element
    ///
    /// # Returns
    ///
    /// Returns the first matching WebElement within the shadow root.
    ///
    /// # Errors
    ///
    /// * `UtamError::ElementNotFound` - When no element matches the selector
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let shadow = element.get_shadow_root().await?;
    /// let button = shadow.find(By::Css(".submit-button")).await?;
    /// button.click().await?;
    /// ```
    pub async fn find(&self, by: By) -> UtamResult<WebElement> {
        self.inner.find(by.clone()).await.map_err(|_| UtamError::ElementNotFound {
            name: "shadow element".to_string(),
            selector: format!("{:?}", by),
        })
    }

    /// Find all elements matching the selector within the shadow root
    ///
    /// # Arguments
    ///
    /// * `by` - The selector to use for finding elements
    ///
    /// # Returns
    ///
    /// Returns a vector of all matching WebElements within the shadow root.
    ///
    /// # Errors
    ///
    /// * `UtamError::WebDriver` - When the WebDriver operation fails
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let shadow = element.get_shadow_root().await?;
    /// let items = shadow.find_all(By::Css(".list-item")).await?;
    /// for item in items {
    ///     println!("{}", item.text().await?);
    /// }
    /// ```
    pub async fn find_all(&self, by: By) -> UtamResult<Vec<WebElement>> {
        Ok(self.inner.find_all(by).await?)
    }
}

/// Traverse a path through nested shadow DOMs to find an element
///
/// This helper function allows you to navigate through multiple levels
/// of shadow DOMs by providing a path of selectors. Each selector is used
/// to find an element, then access its shadow root, and continue to the
/// next selector until the final element is reached.
///
/// # Arguments
///
/// * `root` - The starting WebElement that has a shadow root
/// * `path` - Array of selectors to traverse through nested shadow roots
///
/// # Returns
///
/// Returns the final WebElement found by following the path.
///
/// # Errors
///
/// * `UtamError::ShadowRootNotFound` - When any element in the path doesn't have a shadow root
/// * `UtamError::ElementNotFound` - When an element can't be found at any step
///
/// # Examples
///
/// ```rust,ignore
/// // Find a button inside nested shadow DOMs:
/// // root -> shadow -> component -> shadow -> button
/// let button = traverse_shadow_path(
///     &root_element,
///     &[
///         By::Css(".component"),
///         By::Css(".inner-component"),
///         By::Css(".submit-button"),
///     ]
/// ).await?;
/// button.click().await?;
/// ```
pub async fn traverse_shadow_path(root: &WebElement, path: &[By]) -> UtamResult<WebElement> {
    let mut current = root.clone();

    for (i, selector) in path.iter().enumerate() {
        let shadow = current.get_shadow_root().await.map_err(|_| {
            UtamError::ShadowRootNotFound { element: format!("path element {}", i) }
        })?;

        current = shadow.find(selector.clone()).await.map_err(|_| UtamError::ElementNotFound {
            name: format!("element at path index {}", i),
            selector: format!("{:?}", selector),
        })?;
    }

    Ok(current)
}
