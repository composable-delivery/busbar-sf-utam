//! Base trait for actionable elements
//!
//! Provides methods for focus, blur, scroll, and move operations.
//! This is the foundation trait that other interaction traits extend.

use async_trait::async_trait;
use thirtyfour::{WebDriver, WebElement};

use crate::error::UtamResult;

/// Base trait for actionable elements
///
/// Provides methods for focus, blur, scroll, and move operations.
/// All other interaction traits (Clickable, Editable, Draggable) extend this.
#[async_trait]
pub trait Actionable: Send + Sync {
    /// Get the underlying WebElement
    fn inner(&self) -> &WebElement;

    /// Get a WebDriver instance from the element's session handle
    ///
    /// This method constructs a WebDriver by cloning the element's session handle.
    /// The `handle` field is a public field of both WebDriver and WebElement in thirtyfour,
    /// making this a safe and stable pattern.
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
