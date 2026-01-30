//! Element wrappers for UTAM
//!
//! This module provides wrapper types around WebDriver elements
//! with UTAM-specific error handling and Shadow DOM support.

use crate::error::{UtamError, UtamResult};
use crate::shadow::ShadowRoot;
use thirtyfour::prelude::*;

/// Base element wrapper that provides common functionality
/// for all UTAM elements including Shadow DOM access
pub struct BaseElement {
    pub(crate) inner: WebElement,
}

impl BaseElement {
    /// Create a new BaseElement from a WebElement
    pub fn new(element: WebElement) -> Self {
        Self { inner: element }
    }

    /// Get the shadow root of this element
    ///
    /// # Returns
    ///
    /// Returns a `ShadowRoot` wrapper that can be used to find elements
    /// within the shadow DOM.
    ///
    /// # Errors
    ///
    /// * `UtamError::ShadowRootNotFound` - When the element doesn't have a shadow root
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let element = page.get_component().await?;
    /// let shadow = element.get_shadow_root().await?;
    /// let inner_element = shadow.find(By::Css(".inner-button")).await?;
    /// ```
    pub async fn get_shadow_root(&self) -> UtamResult<ShadowRoot> {
        let shadow = self
            .inner
            .get_shadow_root()
            .await
            .map_err(|_| UtamError::ShadowRootNotFound { element: "unknown".to_string() })?;
        Ok(ShadowRoot::new(shadow))
    }

    /// Get the underlying WebElement
    pub fn inner(&self) -> &WebElement {
        &self.inner
    }
}
