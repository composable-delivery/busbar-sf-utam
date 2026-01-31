//! Frame element and context for iframe handling
//!
//! This module provides support for switching into iframe contexts and back.
//! Uses RAII pattern for automatic context cleanup.

use thirtyfour::prelude::*;

use crate::error::UtamResult;

/// Element wrapper for iframe elements
///
/// Provides methods to enter the iframe context for interaction with
/// elements inside the frame.
#[derive(Debug, Clone)]
pub struct FrameElement {
    inner: WebElement,
}

impl FrameElement {
    /// Create a new FrameElement from a WebElement
    pub fn new(element: WebElement) -> Self {
        Self { inner: element }
    }

    /// Get the underlying WebElement
    pub fn inner(&self) -> &WebElement {
        &self.inner
    }

    /// Enter the frame context
    ///
    /// This switches the WebDriver context to this iframe, allowing
    /// queries and actions on elements within the frame.
    ///
    /// # Returns
    ///
    /// A `FrameContext` guard that will automatically switch back to
    /// the parent frame when dropped.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let frame = page.get_content_frame().await?;
    /// let ctx = frame.enter().await?;
    /// let btn = ctx.find(By::Css(".btn")).await?;
    /// btn.click().await?;
    /// ctx.exit().await?;  // Or let it auto-exit on drop
    /// ```
    pub async fn enter(&self) -> UtamResult<FrameContext> {
        // Clone the element to enter frame (enter_frame consumes self)
        let element = self.inner.clone();
        let driver = WebDriver { handle: element.handle.clone() };
        
        // Switch to the frame context
        element.enter_frame().await?;
        
        Ok(FrameContext { driver })
    }
}

/// RAII guard for frame context - switches back to parent on drop
///
/// This guard ensures that when you're done working within a frame,
/// the WebDriver context automatically switches back to the parent frame.
///
/// # Safety
///
/// The drop implementation spawns a tokio task to perform the async
/// operation of switching back to parent frame. For more reliable cleanup,
/// prefer explicitly calling `exit()` when possible.
pub struct FrameContext {
    driver: WebDriver,
}

impl FrameContext {
    /// Find element within frame
    ///
    /// Queries for an element within the current frame context.
    ///
    /// # Arguments
    ///
    /// * `by` - The selector to use for finding the element
    ///
    /// # Returns
    ///
    /// The WebElement if found
    ///
    /// # Errors
    ///
    /// Returns a WebDriver error if the element is not found
    pub async fn find(&self, by: By) -> UtamResult<WebElement> {
        Ok(self.driver.find(by).await?)
    }

    /// Explicitly exit frame (or let it auto-exit on drop)
    ///
    /// This method switches back to the parent frame context.
    /// It consumes self to prevent double-exit.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let ctx = frame.enter().await?;
    /// // ... work with frame elements ...
    /// ctx.exit().await?;  // Explicit exit
    /// ```
    pub async fn exit(self) -> UtamResult<()> {
        self.driver.enter_parent_frame().await?;
        std::mem::forget(self); // Prevent double-exit in drop
        Ok(())
    }
}

impl Drop for FrameContext {
    fn drop(&mut self) {
        // Note: Can't await in drop, so we spawn a task
        // In practice, prefer explicit exit()
        let driver = self.driver.clone();
        tokio::spawn(async move {
            let _ = driver.enter_parent_frame().await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_element_new() {
        // Just verify we can construct the type
        // We can't create a real WebElement without a driver
    }

    #[test]
    fn test_frame_element_has_correct_api() {
        // Verify API exists at compile time
        fn _check_api_exists() {
            #[allow(unreachable_code)]
            #[allow(clippy::diverging_sub_expression)]
            {
                let _frame: FrameElement = panic!("not meant to run");
                let _ = _frame.inner();
            }
        }
    }

    #[test]
    fn test_frame_context_api_exists() {
        // Verify API exists at compile time
        fn _check_api_exists() {
            #[allow(unreachable_code)]
            #[allow(clippy::diverging_sub_expression)]
            {
                let _ctx: FrameContext = panic!("not meant to run");
            }
        }
    }
}
