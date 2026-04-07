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
//! - [`driver`] — [`UtamDriver`] / [`ElementHandle`] traits abstracting
//!   the browser protocol. Bundled [`ThirtyfourDriver`] for WebDriver.
//! - [`page_object`] — [`DynamicPageObject`] loads an AST and executes
//!   compose methods at runtime via [`PageObjectRuntime`].
//! - [`element`] — [`DynamicElement`] dispatches UTAM action names to
//!   `ElementHandle` methods.
//! - [`registry`] — [`PageObjectRegistry`] discovers and caches `.utam.json`
//!   files from the filesystem.
//! - [`error`] — [`RuntimeError`] for runtime-specific failures.

pub mod driver;
pub mod element;
pub mod error;
pub mod page_object;
pub mod registry;

pub use driver::{ElementHandle, Selector, ShadowRootHandle, ThirtyfourDriver, UtamDriver};
pub use element::{DynamicElement, ElementCapability, ElementRuntime, RuntimeValue};
pub use error::{RuntimeError, RuntimeResult};
pub use page_object::{DynamicPageObject, MethodInfo, PageObjectRuntime};
pub use registry::PageObjectRegistry;

pub mod prelude {
    pub use crate::driver::{ElementHandle, Selector, ThirtyfourDriver, UtamDriver};
    pub use crate::element::{DynamicElement, ElementRuntime, RuntimeValue};
    pub use crate::error::{RuntimeError, RuntimeResult};
    pub use crate::page_object::{DynamicPageObject, PageObjectRuntime};
    pub use crate::registry::PageObjectRegistry;
}
