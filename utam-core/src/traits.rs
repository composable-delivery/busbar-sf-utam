//! Traits for element interactions
//!
//! This module provides async traits for different types of element interactions.
//! All traits extend Send + Sync for thread safety and use async methods for
//! WebDriver operations.

use async_trait::async_trait;
use std::time::Duration;
use thirtyfour::{WebDriver, WebElement};

use crate::error::UtamResult;

/// Base trait for actionable elements
///
/// Provides methods for focus, blur, scroll, and move operations.
#[async_trait]
pub trait Actionable: Send + Sync {
    /// Get the underlying WebElement
    fn inner(&self) -> &WebElement;

    /// Get a WebDriver instance from the element's session handle
    ///
    /// This method constructs a WebDriver by cloning the element's session handle.
    /// The `handle` field is a public field of both WebDriver and WebElement in thirtyfour,
    /// making this a safe and stable pattern. This approach is necessary because thirtyfour
    /// doesn't provide a direct method to get a WebDriver from a WebElement.
    fn driver(&self) -> WebDriver {
        WebDriver { handle: self.inner().handle.clone() }
    }

    /// Focus on this element
    async fn focus(&self) -> UtamResult<()> {
        self.inner().focus().await?;
        Ok(())
    }

    /// Remove focus from this element
    async fn blur(&self) -> UtamResult<()> {
        // Blur by executing JavaScript since thirtyfour doesn't have a direct blur method
        let driver = self.driver();
        driver.execute("arguments[0].blur();", vec![self.inner().to_json()?]).await?;
        Ok(())
    }

    /// Scroll this element into view
    async fn scroll_into_view(&self) -> UtamResult<()> {
        let driver = self.driver();
        driver.execute("arguments[0].scrollIntoView();", vec![self.inner().to_json()?]).await?;
        Ok(())
    }

    /// Move the mouse to this element
    async fn move_to(&self) -> UtamResult<()> {
        let driver = self.driver();
        driver.action_chain().move_to_element_center(self.inner()).perform().await?;
        Ok(())
    }
}

/// Trait for clickable elements
///
/// Extends Actionable with click operations.
#[async_trait]
pub trait Clickable: Actionable {
    /// Click this element
    async fn click(&self) -> UtamResult<()> {
        self.inner().click().await?;
        Ok(())
    }

    /// Double-click this element
    async fn double_click(&self) -> UtamResult<()> {
        let driver = self.driver();
        driver.action_chain().double_click_element(self.inner()).perform().await?;
        Ok(())
    }

    /// Right-click (context click) this element
    async fn right_click(&self) -> UtamResult<()> {
        let driver = self.driver();
        driver.action_chain().context_click_element(self.inner()).perform().await?;
        Ok(())
    }

    /// Click and hold this element
    async fn click_and_hold(&self) -> UtamResult<()> {
        let driver = self.driver();
        driver.action_chain().click_and_hold_element(self.inner()).perform().await?;
        Ok(())
    }
}

/// Trait for editable elements
///
/// Extends Actionable with text input operations.
#[async_trait]
pub trait Editable: Actionable {
    /// Clear the text in this element
    async fn clear(&self) -> UtamResult<()> {
        self.inner().clear().await?;
        Ok(())
    }

    /// Set the text of this element (clears first, then types)
    async fn clear_and_type(&self, text: &str) -> UtamResult<()> {
        self.clear().await?;
        self.inner().send_keys(text).await?;
        Ok(())
    }

    /// Set text without clearing first
    async fn set_text(&self, text: &str) -> UtamResult<()> {
        self.inner().send_keys(text).await?;
        Ok(())
    }

    /// Press a single key
    async fn press(&self, key: &str) -> UtamResult<()> {
        self.inner().send_keys(key).await?;
        Ok(())
    }
}

/// Trait for draggable elements
///
/// Extends Actionable with drag-and-drop operations.
#[async_trait]
pub trait Draggable: Actionable {
    /// Drag this element to another element
    async fn drag_and_drop(&self, target: &WebElement) -> UtamResult<()> {
        let driver = self.driver();
        driver.action_chain().drag_and_drop_element(self.inner(), target).perform().await?;
        Ok(())
    }

    /// Drag this element to another element with a duration
    ///
    /// This performs a slower drag operation by:
    /// 1. Clicking and holding on the source element
    /// 2. Waiting for the specified duration (simulates human hesitation)
    /// 3. Moving to the target element and releasing
    ///
    /// Note: The duration represents a pause between clicking and moving, not the
    /// duration of the drag movement itself. This simulates human-like behavior where
    /// there's a delay between initiating a drag and completing it.
    ///
    /// # Arguments
    ///
    /// * `target` - The element to drag to
    /// * `duration` - How long to wait after clicking before moving to target
    async fn drag_and_drop_with_duration(
        &self,
        target: &WebElement,
        duration: Duration,
    ) -> UtamResult<()> {
        let driver = self.driver();

        // Click and hold on source element
        driver.action_chain().click_and_hold_element(self.inner()).perform().await?;

        // Wait for specified duration (simulates human hesitation)
        tokio::time::sleep(duration).await;

        // Move to target and release
        driver.action_chain().move_to_element_center(target).release().perform().await?;
        Ok(())
    }

    /// Drag this element by a pixel offset
    async fn drag_and_drop_by_offset(&self, x: i64, y: i64) -> UtamResult<()> {
        let driver = self.driver();
        driver.action_chain().drag_and_drop_element_by_offset(self.inner(), x, y).perform().await?;
        Ok(())
    }
}
