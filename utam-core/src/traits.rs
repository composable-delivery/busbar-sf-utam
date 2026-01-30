//! Traits for element interactions
//!
//! This module defines the traits that element wrappers implement to provide
//! WebDriver automation capabilities.

use crate::error::UtamResult;
use async_trait::async_trait;
use thirtyfour::WebElement;

/// Key codes for keyboard input
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// Enter key
    Enter,
    /// Tab key
    Tab,
    /// Escape key
    Escape,
    /// Backspace key
    Backspace,
    /// Delete key
    Delete,
    /// Up arrow key
    ArrowUp,
    /// Down arrow key
    ArrowDown,
    /// Left arrow key
    ArrowLeft,
    /// Right arrow key
    ArrowRight,
    /// Home key
    Home,
    /// End key
    End,
    /// Page Up key
    PageUp,
    /// Page Down key
    PageDown,
    /// Space key
    Space,
}

impl From<Key> for thirtyfour::Key {
    fn from(key: Key) -> Self {
        match key {
            Key::Enter => thirtyfour::Key::Enter,
            Key::Tab => thirtyfour::Key::Tab,
            Key::Escape => thirtyfour::Key::Escape,
            Key::Backspace => thirtyfour::Key::Backspace,
            Key::Delete => thirtyfour::Key::Delete,
            Key::ArrowUp => thirtyfour::Key::Up,
            Key::ArrowDown => thirtyfour::Key::Down,
            Key::ArrowLeft => thirtyfour::Key::Left,
            Key::ArrowRight => thirtyfour::Key::Right,
            Key::Home => thirtyfour::Key::Home,
            Key::End => thirtyfour::Key::End,
            Key::PageUp => thirtyfour::Key::PageUp,
            Key::PageDown => thirtyfour::Key::PageDown,
            Key::Space => thirtyfour::Key::Space,
        }
    }
}

/// Base trait for actionable elements
///
/// This trait provides access to the underlying WebElement and serves as the
/// foundation for more specialized traits like `Clickable` and `Editable`.
#[async_trait]
pub trait Actionable: Send + Sync {
    /// Get a reference to the underlying WebElement
    fn inner(&self) -> &WebElement;

    /// Focus on this element
    async fn focus(&self) -> UtamResult<()> {
        self.inner().focus().await?;
        Ok(())
    }

    /// Scroll the element into view
    async fn scroll_into_view(&self) -> UtamResult<()> {
        self.inner().scroll_into_view().await?;
        Ok(())
    }
}

/// Trait for editable elements (text inputs, textareas, etc.)
///
/// This trait extends `Actionable` with methods for typing text and pressing keys.
#[async_trait]
pub trait Editable: Actionable {
    /// Clear the element's value
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// element.clear().await?;
    /// ```
    async fn clear(&self) -> UtamResult<()> {
        self.inner().clear().await?;
        Ok(())
    }

    /// Type text into the element without clearing
    ///
    /// # Arguments
    ///
    /// * `text` - The text to type
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// element.set_text("hello").await?;
    /// ```
    async fn set_text(&self, text: &str) -> UtamResult<()> {
        self.inner().send_keys(text).await?;
        Ok(())
    }

    /// Clear the element and then type text
    ///
    /// # Arguments
    ///
    /// * `text` - The text to type
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// element.clear_and_type("new text").await?;
    /// ```
    async fn clear_and_type(&self, text: &str) -> UtamResult<()> {
        self.clear().await?;
        self.set_text(text).await?;
        Ok(())
    }

    /// Press a keyboard key
    ///
    /// # Arguments
    ///
    /// * `key` - The key to press
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// element.press(Key::Enter).await?;
    /// ```
    async fn press(&self, key: Key) -> UtamResult<()> {
        let tf_key: thirtyfour::Key = key.into();
        self.inner().send_keys(tf_key).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_conversion() {
        // Test that our Key enum converts to thirtyfour::Key correctly
        let _: thirtyfour::Key = Key::Enter.into();
        let _: thirtyfour::Key = Key::Tab.into();
        let _: thirtyfour::Key = Key::Escape.into();
        let _: thirtyfour::Key = Key::Backspace.into();
        let _: thirtyfour::Key = Key::Delete.into();
        let _: thirtyfour::Key = Key::ArrowUp.into();
        let _: thirtyfour::Key = Key::ArrowDown.into();
        let _: thirtyfour::Key = Key::ArrowLeft.into();
        let _: thirtyfour::Key = Key::ArrowRight.into();
        let _: thirtyfour::Key = Key::Home.into();
        let _: thirtyfour::Key = Key::End.into();
        let _: thirtyfour::Key = Key::PageUp.into();
        let _: thirtyfour::Key = Key::PageDown.into();
        let _: thirtyfour::Key = Key::Space.into();
    }

    #[test]
    fn test_key_mappings() {
        // Test that arrow keys map to the correct thirtyfour variants
        let up: thirtyfour::Key = Key::ArrowUp.into();
        let down: thirtyfour::Key = Key::ArrowDown.into();
        let left: thirtyfour::Key = Key::ArrowLeft.into();
        let right: thirtyfour::Key = Key::ArrowRight.into();
        
        // Verify they produce different key codes
        assert_ne!(up.value(), down.value());
        assert_ne!(left.value(), right.value());
    }

    #[test]
    fn test_key_clone_and_copy() {
        // Test that Key implements Clone and Copy
        let key = Key::Enter;
        let _cloned = key; // Copy, not clone, since Key implements Copy
        let _copied = key;
        let _used_again = key; // Should work because Key is Copy
    }

    #[test]
    fn test_key_debug() {
        // Test that Key implements Debug
        let key = Key::Enter;
        let debug_str = format!("{:?}", key);
        assert!(debug_str.contains("Enter"));
    }
}
