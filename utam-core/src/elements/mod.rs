//! Element wrappers for UTAM runtime
//!
//! This module provides wrappers around thirtyfour::WebElement with
//! convenient methods for common element operations. Each element type
//! is in its own submodule to allow independent development.
//!
//! # Element Types
//!
//! - [`BaseElement`] - Core wrapper with attribute queries, state checks, wait utilities
//! - [`ClickableElement`] - Implements [`Clickable`](crate::traits::Clickable)
//! - [`EditableElement`] - Implements [`Editable`](crate::traits::Editable)
//! - [`DraggableElement`] - Implements [`Draggable`](crate::traits::Draggable)
//! - [`Container`] - Generic container for dynamic/slot content
//! - [`FrameElement`] - Wrapper for iframe elements with context switching
//! - [`FrameContext`] - RAII guard for iframe context management
//! - [`ElementRectangle`] - Position and size data

mod base;
mod clickable;
mod container;
mod draggable;
mod editable;
mod frame;
mod rectangle;

pub use base::BaseElement;
pub use clickable::ClickableElement;
pub use container::Container;
pub use draggable::DraggableElement;
pub use editable::EditableElement;
pub use frame::{FrameContext, FrameElement};
pub use rectangle::ElementRectangle;
