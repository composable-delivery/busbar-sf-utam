//! Element wrappers for UTAM operations
//!
//! This module provides wrappers around WebDriver elements with
//! additional functionality like wait utilities.

use crate::error::{UtamError, UtamResult};
use crate::wait::{wait_for, WaitConfig};
use std::time::Duration;
use thirtyfour::WebElement;

/// Base element wrapper providing UTAM-specific functionality
///
/// Wraps a `WebElement` from thirtyfour and adds wait utilities
/// and other UTAM operations.
#[derive(Debug, Clone)]
pub struct BaseElement {
    inner: WebElement,
}

impl BaseElement {
    /// Create a new BaseElement from a WebElement
    pub fn new(element: WebElement) -> Self {
        Self { inner: element }
    }

    /// Get a reference to the underlying WebElement
    pub fn inner(&self) -> &WebElement {
        &self.inner
    }

    /// Check if the element is currently present in the DOM
    ///
    /// # Errors
    ///
    /// Returns error if WebDriver operation fails
    ///
    /// # Implementation Notes
    ///
    /// This method attempts to call `is_enabled()` on the element to check if it's
    /// still attached to the DOM. If the element has become stale (no longer in DOM),
    /// the WebDriver will return an error containing "stale" or "no such element".
    ///
    /// **Note**: This error detection relies on substring matching in error messages,
    /// which may not work with all WebDriver implementations or localized error messages.
    /// This is a limitation of the current thirtyfour API which doesn't provide
    /// specific error types for stale elements.
    pub async fn is_present(&self) -> UtamResult<bool> {
        // Try to get a property to check if element is still attached to DOM
        // If it throws a stale element exception, it's not present
        match self.inner.is_enabled().await {
            Ok(_) => Ok(true),
            Err(e) => {
                // Check if it's a stale element error
                // This is fragile but necessary given current thirtyfour API
                let err_str = e.to_string().to_lowercase();
                if err_str.contains("stale") || err_str.contains("no such element") {
                    Ok(false)
                } else {
                    Err(UtamError::WebDriver(e))
                }
            }
        }
    }

    /// Check if the element is currently visible
    ///
    /// # Errors
    ///
    /// Returns error if WebDriver operation fails
    pub async fn is_visible(&self) -> UtamResult<bool> {
        Ok(self.inner.is_displayed().await?)
    }

    /// Check if the element is currently enabled
    ///
    /// # Errors
    ///
    /// Returns error if WebDriver operation fails
    pub async fn is_enabled(&self) -> UtamResult<bool> {
        Ok(self.inner.is_enabled().await?)
    }

    /// Wait for the element to become visible
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time to wait for visibility
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the element becomes visible within the timeout period.
    ///
    /// # Errors
    ///
    /// * `UtamError::Timeout` - When element doesn't become visible within timeout
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use std::time::Duration;
    ///
    /// element.wait_for_visible(Duration::from_secs(10)).await?;
    /// ```
    pub async fn wait_for_visible(&self, timeout: Duration) -> UtamResult<()> {
        let element = self.clone();
        wait_for(
            || async {
                if element.is_visible().await? {
                    Ok(Some(()))
                } else {
                    Ok(None)
                }
            },
            &WaitConfig {
                timeout,
                ..Default::default()
            },
            "element to become visible",
        )
        .await
    }

    /// Wait for the element to become invisible
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time to wait for invisibility
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the element becomes invisible within the timeout period.
    /// A stale element (removed from DOM) is also considered invisible.
    ///
    /// # Errors
    ///
    /// * `UtamError::Timeout` - When element doesn't become invisible within timeout
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use std::time::Duration;
    ///
    /// element.wait_for_invisible(Duration::from_secs(10)).await?;
    /// ```
    pub async fn wait_for_invisible(&self, timeout: Duration) -> UtamResult<()> {
        let element = self.clone();
        wait_for(
            || async {
                // If element is not present (stale), consider it invisible
                if !element.is_present().await? {
                    return Ok(Some(()));
                }
                // Otherwise check if it's invisible
                if !element.is_visible().await? {
                    Ok(Some(()))
                } else {
                    Ok(None)
                }
            },
            &WaitConfig {
                timeout,
                ..Default::default()
            },
            "element to become invisible",
        )
        .await
    }

    /// Wait for the element to be removed from the DOM
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time to wait for removal
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the element is removed from DOM within the timeout period.
    ///
    /// # Errors
    ///
    /// * `UtamError::Timeout` - When element is not removed within timeout
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use std::time::Duration;
    ///
    /// element.wait_for_absence(Duration::from_secs(10)).await?;
    /// ```
    pub async fn wait_for_absence(&self, timeout: Duration) -> UtamResult<()> {
        let element = self.clone();
        wait_for(
            || async {
                if !element.is_present().await? {
                    Ok(Some(()))
                } else {
                    Ok(None)
                }
            },
            &WaitConfig {
                timeout,
                ..Default::default()
            },
            "element to be removed from DOM",
        )
        .await
    }

    /// Wait for the element to become enabled
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time to wait for element to be enabled
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the element becomes enabled within the timeout period.
    ///
    /// # Errors
    ///
    /// * `UtamError::Timeout` - When element doesn't become enabled within timeout
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use std::time::Duration;
    ///
    /// element.wait_for_enabled(Duration::from_secs(10)).await?;
    /// ```
    pub async fn wait_for_enabled(&self, timeout: Duration) -> UtamResult<()> {
        let element = self.clone();
        wait_for(
            || async {
                if element.is_enabled().await? {
                    Ok(Some(()))
                } else {
                    Ok(None)
                }
            },
            &WaitConfig {
                timeout,
                ..Default::default()
            },
            "element to become enabled",
        )
        .await
    }
}
