//! Trait for editable elements
//!
//! Extends Actionable with text input operations and keyboard key support.

use async_trait::async_trait;

use crate::error::UtamResult;
use crate::traits::Actionable;

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

/// Trait for editable elements (text inputs, textareas, etc.)
///
/// Extends Actionable with methods for typing text and pressing keys.
#[async_trait]
pub trait Editable: Actionable {
    /// Clear the text in this element
    async fn clear(&self) -> UtamResult<()> {
        self.inner().clear().await?;
        Ok(())
    }

    /// Set text without clearing first
    async fn set_text(&self, text: &str) -> UtamResult<()> {
        self.inner().send_keys(text).await?;
        Ok(())
    }

    /// Clear the element and then type text
    async fn clear_and_type(&self, text: &str) -> UtamResult<()> {
        self.clear().await?;
        self.set_text(text).await?;
        Ok(())
    }

    /// Press a keyboard key
    async fn press(&self, key: Key) -> UtamResult<()> {
        let tf_key: thirtyfour::Key = key.into();
        self.inner().send_keys(tf_key).await?;
        Ok(())
    }
}
