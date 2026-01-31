//! UTAM Core Runtime Library
//!
//! This crate provides the runtime traits and types for the UTAM
//! (UI Test Automation Model) framework.
//!
//! # Module Structure
//!
//! - [`elements`] - Element wrappers (BaseElement, ClickableElement, etc.)
//! - [`traits`] - Async traits (Actionable, Clickable, Editable, Draggable, PageObject)
//! - [`error`] - Error types (UtamError, UtamResult)
//! - [`shadow`] - Shadow DOM support (ShadowRoot, traverse_shadow_path)
//! - [`wait`] - Wait utilities (WaitConfig, wait_for)
//!
//! # Example
//!
//! ```rust,ignore
//! use utam_core::prelude::*;
//!
//! // Generated page object
//! let login = LoginForm::load(&driver).await?;
//! login.login("user", "pass").await?;
//! ```

pub mod elements;
pub mod error;
pub mod shadow;
pub mod traits;
pub mod wait;

pub mod prelude {
    pub use crate::elements::*;
    pub use crate::error::{UtamError, UtamResult};
    pub use crate::shadow::*;
    pub use crate::traits::*;
    pub use crate::wait::*;
    // Re-export thirtyfour essentials explicitly to avoid Key name collision
    pub use thirtyfour::prelude::{By, WebDriver, WebDriverError, WebElement};
}
