//! Traits for element interactions and page objects
//!
//! This module provides async traits for different types of element interactions.
//! Each trait is in its own submodule to allow independent development.
//!
//! # Trait Hierarchy
//!
//! - [`Actionable`] - Base trait (focus, blur, scroll, move)
//!   - [`Clickable`] - Click operations
//!   - [`Editable`] - Text input operations
//!   - [`Draggable`] - Drag-and-drop operations
//! - [`PageObject`] - Base trait for all page objects
//!   - [`RootPageObject`] - Page objects that can be loaded directly

mod actionable;
mod clickable;
mod draggable;
mod editable;
mod page_object;

pub use actionable::Actionable;
pub use clickable::Clickable;
pub use draggable::Draggable;
pub use editable::{Editable, Key};
pub use page_object::{PageObject, RootPageObject};
