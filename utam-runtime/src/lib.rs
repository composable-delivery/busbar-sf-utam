//! UTAM Runtime Interpreter
//!
//! Loads UTAM page object JSON definitions at runtime and provides a
//! trait-based interface to execute actions dynamically — no compilation
//! step required.
//!
//! # Architecture
//!
//! - [`driver`] — [`UtamDriver`](driver::UtamDriver) and [`ElementHandle`](driver::ElementHandle)
//!   traits abstracting the browser automation protocol. Bundled
//!   [`ThirtyfourDriver`](driver::ThirtyfourDriver) adapter for WebDriver;
//!   alternative backends (CDP, Playwright) can be plugged in externally.
//! - [`element`] — [`DynamicElement`] dispatches UTAM action names to
//!   `ElementHandle` methods at runtime.
//! - [`element::RuntimeValue`] — dynamically-typed values flowing through
//!   the interpreter.
//! - [`error`] — [`RuntimeError`](error::RuntimeError) for runtime-specific failures.
//!
//! # Example
//!
//! ```rust,ignore
//! use utam_runtime::prelude::*;
//!
//! let driver = ThirtyfourDriver::new(webdriver);
//! driver.navigate("https://login.salesforce.com").await?;
//! let el = driver.find_element(&Selector::Css(".submit".into())).await?;
//! el.click().await?;
//! ```

pub mod driver;
pub mod element;
pub mod error;

pub use driver::{ElementHandle, Selector, ShadowRootHandle, ThirtyfourDriver, UtamDriver};
pub use element::{DynamicElement, ElementCapability, ElementRuntime, RuntimeValue};
pub use error::{RuntimeError, RuntimeResult};

pub mod prelude {
    pub use crate::driver::{ElementHandle, Selector, ThirtyfourDriver, UtamDriver};
    pub use crate::element::{DynamicElement, ElementRuntime, RuntimeValue};
    pub use crate::error::{RuntimeError, RuntimeResult};
}
