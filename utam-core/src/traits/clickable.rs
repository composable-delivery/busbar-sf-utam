//! Trait for clickable elements
//!
//! Extends Actionable with click operations.

use async_trait::async_trait;

use crate::error::UtamResult;
use crate::traits::Actionable;

/// Trait for clickable elements
///
/// Extends Actionable with click operations including single click,
/// double click, right click, and click-and-hold.
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
