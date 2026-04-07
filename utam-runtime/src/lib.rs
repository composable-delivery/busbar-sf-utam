//! UTAM Runtime Interpreter
//!
//! Loads UTAM page object JSON definitions at runtime and provides a
//! trait-based interface to execute actions dynamically — no compilation step required.
//!
//! This is the foundation for agent-driven browser testing: an AI agent
//! can load a `.utam.json` file, discover its methods and elements, and
//! call them through [`DynamicPageObject`].
//!
//! # Architecture
//!
//! - [`element`] — [`DynamicElement`](element::DynamicElement) wraps utam-core
//!   element types behind a uniform [`ElementRuntime`](element::ElementRuntime) trait
//! - [`element::RuntimeValue`] — dynamically-typed values flowing through the interpreter
//! - [`error`] — [`RuntimeError`](error::RuntimeError) for runtime-specific failures
//!
//! # Status
//!
//! Work in progress. The page object loader, compose method interpreter,
//! and page object registry are coming next.

pub mod element;
pub mod error;

pub use element::{DynamicElement, ElementRuntime, RuntimeValue};
pub use error::{RuntimeError, RuntimeResult};
