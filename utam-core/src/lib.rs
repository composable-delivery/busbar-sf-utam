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
    // Re-export traits
    pub use crate::error::{UtamError, UtamResult};
    pub use crate::traits::{PageObject, RootPageObject};
    pub use crate::wait::{wait_for, WaitConfig};
    // TODO: Re-enable once modules are implemented
    // pub use crate::elements::*;
    // pub use crate::shadow::*;
    pub use thirtyfour::prelude::*;
}

// Re-export main types
pub use error::{UtamError, UtamResult};
pub use traits::{PageObject, RootPageObject};
pub use wait::{wait_for, WaitConfig};
// TODO: Re-enable once modules are implemented
// pub use elements::*;
// pub use shadow::*;
