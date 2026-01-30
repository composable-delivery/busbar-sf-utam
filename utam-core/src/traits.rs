//! Traits module for UTAM element behaviors
//!
//! This module defines the traits that elements can implement to provide
//! various interaction capabilities like clicking, typing, etc.

use crate::error::UtamResult;
use async_trait::async_trait;
use std::time::Duration;
use thirtyfour::WebElement;

/// Base trait for elements that can be interacted with.
///
/// This trait provides access to the underlying WebElement and serves
/// as the foundation for more specific interaction traits.
#[async_trait]
pub trait Actionable: Send + Sync {
    /// Get a reference to the underlying WebElement
    fn inner(&self) -> &WebElement;
}

/// Trait for elements that support click-related actions.
///
/// This trait extends `Actionable` and provides methods for clicking,
/// double-clicking, right-clicking, and click-and-hold operations.
///
/// # Examples
///
/// ```rust,ignore
/// let button = page.get_submit_button().await?;
/// button.click().await?;
/// button.double_click().await?;
/// button.right_click().await?;
/// ```
#[async_trait]
pub trait Clickable: Actionable {
    /// Performs a standard left click on the element.
    ///
    /// # Errors
    ///
    /// Returns an error if the WebDriver operation fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// button.click().await?;
    /// ```
    async fn click(&self) -> UtamResult<()> {
        self.inner().click().await?;
        Ok(())
    }

    /// Performs a double click on the element.
    ///
    /// Uses the WebDriver ActionChain API to perform a double click.
    ///
    /// # Errors
    ///
    /// Returns an error if the WebDriver operation fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// button.double_click().await?;
    /// ```
    async fn double_click(&self) -> UtamResult<()> {
        self.inner()
            .handle
            .action_chain()
            .double_click_element(self.inner())
            .perform()
            .await?;
        Ok(())
    }

    /// Performs a right click (context menu click) on the element.
    ///
    /// Uses the WebDriver ActionChain API to perform a right click.
    ///
    /// # Errors
    ///
    /// Returns an error if the WebDriver operation fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// button.right_click().await?;
    /// ```
    async fn right_click(&self) -> UtamResult<()> {
        self.inner()
            .handle
            .action_chain()
            .context_click_element(self.inner())
            .perform()
            .await?;
        Ok(())
    }

    /// Clicks and holds the element for the specified duration.
    ///
    /// This method clicks the element, holds for the given duration,
    /// then releases the mouse button.
    ///
    /// # Arguments
    ///
    /// * `duration` - How long to hold the click
    ///
    /// # Errors
    ///
    /// Returns an error if the WebDriver operation fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use std::time::Duration;
    /// button.click_and_hold(Duration::from_millis(500)).await?;
    /// ```
    async fn click_and_hold(&self, duration: Duration) -> UtamResult<()> {
        let handle = &self.inner().handle;
        handle
            .action_chain()
            .click_and_hold_element(self.inner())
            .perform()
            .await?;
        tokio::time::sleep(duration).await;
        handle.action_chain().release().perform().await?;
        Ok(())
    }
}
