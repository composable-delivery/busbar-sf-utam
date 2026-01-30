//! UTAM Core Runtime Library
//!
//! This crate provides the runtime traits and types for the UTAM
//! (UI Test Automation Model) framework.
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

mod elements;
mod error;
mod shadow;
mod traits;
mod wait;

pub mod prelude {
    pub use crate::elements::*;
    pub use crate::error::{UtamError, UtamResult};
    pub use crate::traits::*;
    // Re-export commonly used thirtyfour types, but not Key to avoid conflicts
    pub use thirtyfour::{WebDriver, WebElement};
    pub use thirtyfour::error::{WebDriverError, WebDriverResult};
}

// TODO: Re-enable once modules are implemented
pub use elements::*;
pub use error::{UtamError, UtamResult};
// pub use shadow::*;
pub use traits::*;
// pub use wait::*;
