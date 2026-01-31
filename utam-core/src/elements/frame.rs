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
        
        // SAFETY: WebDriver is a simple wrapper around Arc<SessionHandle>.
        // We're constructing it from the same handle that's already in use by
        // the WebElement, so this is safe and maintains all existing session state.
        let driver = WebDriver { handle: element.handle.clone() };
        
        // Switch to the frame context
        element.enter_frame().await?;
        
        Ok(FrameContext { driver, exited: false })
    }
}

/// RAII guard for frame context - switches back to parent on drop
///
/// This guard ensures that when you're done working within a frame,
/// the WebDriver context automatically switches back to the parent frame.
///
/// # Cleanup Behavior
///
/// When `FrameContext` is dropped without calling `exit()`, it spawns a
/// background task to switch back to the parent frame. This cleanup is
/// best-effort and has limitations:
///
/// - The spawned task may not complete if the program/test exits immediately
/// - Errors during cleanup cannot be observed or handled
/// - If the tokio runtime is shutting down, cleanup may not execute at all
///
/// **Always prefer calling `exit()` explicitly for reliable cleanup.**
///
/// # Safety
///
/// The drop implementation spawns a tokio task to perform the async
/// operation of switching back to parent frame. For more reliable cleanup,
/// prefer explicitly calling `exit()` when possible.
pub struct FrameContext {
    driver: WebDriver,
    // Flag to prevent double-exit when exit() is called explicitly
    exited: bool,
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
    pub async fn exit(mut self) -> UtamResult<()> {
        // Mark as exited before the async operation to prevent double-exit
        // even if the operation fails
        self.exited = true;
        self.driver.enter_parent_frame().await?;
        Ok(())
    }
}

impl Drop for FrameContext {
    fn drop(&mut self) {
        // Only run drop cleanup if exit() was not called
        if !self.exited {
            // Note: Can't await in drop, so we spawn a task
            // 
            // WARNING: The spawned task may not complete before the program exits,
            // potentially leaving the WebDriver in the wrong frame context.
            // This is a best-effort cleanup mechanism.
            // 
            // For reliable cleanup, always prefer calling exit() explicitly.
            let driver = self.driver.clone();
            tokio::spawn(async move {
                let _ = driver.enter_parent_frame().await;
            });
        }
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
