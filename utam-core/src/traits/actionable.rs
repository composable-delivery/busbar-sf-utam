//! Base trait for actionable elements
//!
//! Provides methods for focus, blur, scroll, and move operations.
//! This is the foundation trait that other interaction traits extend.

use std::sync::Arc;

use async_trait::async_trait;
use thirtyfour::session::handle::SessionHandle;
use thirtyfour::WebElement;

use crate::error::UtamResult;

/// Base trait for actionable elements
///
/// Provides methods for focus, blur, scroll, and move operations.
/// All other interaction traits (Clickable, Editable, Draggable) extend this.
#[async_trait]
pub trait Actionable: Send + Sync {
    /// Get the underlying WebElement
    fn inner(&self) -> &WebElement;

    /// Get the element's session handle.
    ///
    /// thirtyfour 0.37 made `WebElement::handle` a private field exposed via a
    /// `handle()` accessor, and `WebDriver` can no longer be constructed from a
    /// handle outside the crate. `SessionHandle` carries every operation these
    /// traits use (`execute`, `action_chain`), and `WebDriver` itself just
    /// derefs to `Arc<SessionHandle>`, so we work with the handle directly.
    fn driver(&self) -> Arc<SessionHandle> {
        self.inner().handle().clone()
    }

    /// Focus on this element
    async fn focus(&self) -> UtamResult<()> {
        self.inner().focus().await?;
        Ok(())
    }

    /// Remove focus from this element
    async fn blur(&self) -> UtamResult<()> {
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

    /// Scroll the element to the center of the viewport
    async fn scroll_to_center(&self) -> UtamResult<()> {
        let driver = self.driver();
        driver
            .execute(
                "arguments[0].scrollIntoView({block: 'center', inline: 'center'})",
                vec![self.inner().to_json()?],
            )
            .await?;
        Ok(())
    }

    /// Scroll the element to the top of the viewport
    async fn scroll_to_top(&self) -> UtamResult<()> {
        let driver = self.driver();
        driver
            .execute(
                "arguments[0].scrollIntoView({block: 'start', inline: 'start'})",
                vec![self.inner().to_json()?],
            )
            .await?;
        Ok(())
    }

    /// Move the mouse to this element
    async fn move_to(&self) -> UtamResult<()> {
        let driver = self.driver();
        driver.action_chain().move_to_element_center(self.inner()).perform().await?;
        Ok(())
    }
}
