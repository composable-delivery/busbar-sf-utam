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

mod error;
mod traits;
mod elements;
mod wait;
mod shadow;

pub mod prelude {
    pub use crate::error::{UtamError, UtamResult};
    pub use crate::traits::*;
    pub use crate::elements::*;
    pub use crate::wait::*;
    pub use crate::shadow::*;
    pub use thirtyfour::prelude::*;
}

pub use error::{UtamError, UtamResult};
pub use traits::*;
pub use elements::*;
pub use wait::*;
pub use shadow::*;
