//! Traits module for user interaction patterns
//!
//! This module defines the core traits for interacting with page elements.

use crate::error::UtamResult;
use async_trait::async_trait;
use thirtyfour::prelude::*;

/// Trait for basic user interaction actions
///
/// Provides methods for focus management, scrolling, and mouse hovering.
/// Elements implementing this trait can be focused, blurred, scrolled into view,
/// and hovered over.
#[async_trait]
pub trait Actionable: Send + Sync {
    /// Get reference to the underlying WebElement
    fn inner(&self) -> &WebElement;

    /// Remove focus from the element using JavaScript
    ///
    /// # Errors
    ///
    /// Returns an error if JavaScript execution fails or the element is not focusable.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// element.blur().await?;
    /// ```
    async fn blur(&self) -> UtamResult<()> {
        let element = self.inner();
        element.handle.execute("arguments[0].blur()", vec![element.to_json()?]).await?;
        Ok(())
    }

    /// Set focus on the element using JavaScript
    ///
    /// # Errors
    ///
    /// Returns an error if JavaScript execution fails or the element is not focusable.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// element.focus().await?;
    /// ```
    async fn focus(&self) -> UtamResult<()> {
        let element = self.inner();
        element.handle.execute("arguments[0].focus()", vec![element.to_json()?]).await?;
        Ok(())
    }

    /// Move mouse pointer to the element (hover)
    ///
    /// Uses the WebDriver Actions API to perform mouse hover.
    ///
    /// # Errors
    ///
    /// Returns an error if the mouse move action fails.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// element.move_to().await?;
    /// ```
    async fn move_to(&self) -> UtamResult<()> {
        let element = self.inner();
        element.handle.action_chain().move_to_element_center(element).perform().await?;
        Ok(())
    }

    /// Scroll the element to the center of the viewport
    ///
    /// Uses JavaScript scrollIntoView with center alignment for both
    /// block (vertical) and inline (horizontal) directions.
    ///
    /// # Errors
    ///
    /// Returns an error if JavaScript execution fails.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// element.scroll_to_center().await?;
    /// ```
    async fn scroll_to_center(&self) -> UtamResult<()> {
        let element = self.inner();
        element
            .handle
            .execute(
                "arguments[0].scrollIntoView({block: 'center', inline: 'center'})",
                vec![element.to_json()?],
            )
            .await?;
        Ok(())
    }

    /// Scroll the element to the top of the viewport
    ///
    /// Uses JavaScript scrollIntoView with start alignment for both
    /// block (vertical) and inline (horizontal) directions.
    ///
    /// # Errors
    ///
    /// Returns an error if JavaScript execution fails.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// element.scroll_to_top().await?;
    /// ```
    async fn scroll_to_top(&self) -> UtamResult<()> {
        let element = self.inner();
        element
            .handle
            .execute(
                "arguments[0].scrollIntoView({block: 'start', inline: 'start'})",
                vec![element.to_json()?],
            )
            .await?;
        Ok(())
    }
}
