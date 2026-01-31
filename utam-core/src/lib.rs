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
    pub use crate::elements::{BaseElement, ElementRectangle};
    pub use crate::error::{UtamError, UtamResult};
    // pub use crate::shadow::*;
    // pub use crate::traits::*;
    // pub use crate::wait::*;
    pub use thirtyfour::prelude::*;
}

pub use elements::{BaseElement, ElementRectangle};
pub use error::{UtamError, UtamResult};
// pub use shadow::*;
// pub use traits::*;
// pub use wait::*;
