//! UTAM Runtime Interpreter
//!
//! Loads UTAM page object JSON definitions at runtime and provides a
//! trait-based interface to execute actions dynamically — no compilation
//! step required.
//!
//! # Architecture
//!
//! ```text
//! UTAM JSON ──→ PageObjectAst ──→ DynamicPageObject ──→ browser
//!                (serde)           (interpreter)        (UtamDriver)
//! ```
//!
//! # Features
//!
//! - `webdriver` (default) — [`ThirtyfourDriver`] for WebDriver/Selenium
//! - `cdp` — [`CdpDriver`] for Chrome DevTools Protocol with checkpointing

pub mod discovery;
pub mod driver;
pub mod element;
pub mod error;
pub mod page_object;
pub mod registry;

pub use driver::{ElementHandle, Selector, ShadowRootHandle, UtamDriver};
pub use element::{DynamicElement, ElementCapability, ElementRuntime, RuntimeValue};
pub use error::{RuntimeError, RuntimeResult};
pub use page_object::{DynamicPageObject, MethodInfo, PageObjectRuntime};
pub use registry::PageObjectRegistry;

#[cfg(feature = "webdriver")]
pub use driver::ThirtyfourDriver;

#[cfg(feature = "cdp")]
pub use driver::CdpDriver;

pub mod prelude {
    pub use crate::driver::{ElementHandle, Selector, UtamDriver};
    pub use crate::element::{DynamicElement, ElementRuntime, RuntimeValue};
    pub use crate::error::{RuntimeError, RuntimeResult};
    pub use crate::page_object::{DynamicPageObject, PageObjectRuntime};
    pub use crate::registry::PageObjectRegistry;

    #[cfg(feature = "webdriver")]
    pub use crate::driver::ThirtyfourDriver;

    #[cfg(feature = "cdp")]
    pub use crate::driver::CdpDriver;
}
