//! Shadow DOM support for UTAM
//!
//! This module provides types and functions for working with Shadow DOM,
//! including traversing nested shadow roots.

use thirtyfour::prelude::*;

use crate::error::{UtamError, UtamResult};

/// Wrapper around a shadow root element providing UTAM-specific functionality
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
    /// # Errors
    ///
    /// * `UtamError::ElementNotFound` - When no element matches the selector
    pub async fn find(&self, by: By) -> UtamResult<WebElement> {
        self.inner.find(by.clone()).await.map_err(|_| UtamError::ElementNotFound {
            name: "shadow element".to_string(),
            selector: format!("{:?}", by),
        })
    }

    /// Find all elements matching the selector within the shadow root
    pub async fn find_all(&self, by: By) -> UtamResult<Vec<WebElement>> {
        Ok(self.inner.find_all(by).await?)
    }
}

/// Traverse a path through nested shadow DOMs to find an element
///
/// This helper function allows navigating through multiple levels
/// of shadow DOMs by providing a path of selectors. Each selector is used
/// to find an element, then access its shadow root, continuing until
/// the final element is reached.
///
/// # Arguments
///
/// * `root` - The starting WebElement that has a shadow root
/// * `path` - Array of selectors to traverse through nested shadow roots
///
/// # Examples
///
/// ```rust,ignore
/// let button = traverse_shadow_path(
///     &root_element,
///     &[
///         By::Css(".component"),
///         By::Css(".inner-component"),
///         By::Css(".submit-button"),
///     ]
/// ).await?;
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
