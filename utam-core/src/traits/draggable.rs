//! Trait for draggable elements
//!
//! Extends Actionable with drag-and-drop operations.

use std::time::Duration;

use async_trait::async_trait;
use thirtyfour::WebElement;

use crate::error::UtamResult;
use crate::traits::Actionable;

/// Trait for draggable elements
///
/// Extends Actionable with drag-and-drop operations including
/// element-to-element drag, timed drag, and offset-based drag.
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
    /// The duration represents a pause between clicking and moving, not the
    /// duration of the drag movement itself.
    async fn drag_and_drop_with_duration(
        &self,
        target: &WebElement,
        duration: Duration,
    ) -> UtamResult<()> {
        let driver = self.driver();
        driver.action_chain().click_and_hold_element(self.inner()).perform().await?;
        tokio::time::sleep(duration).await;
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
