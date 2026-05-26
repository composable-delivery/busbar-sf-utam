//! Driver adapters for UTAM runtime
//!
//! This module provides driver adapters for different browser automation backends.
//! Use [`DriverKind`] to select which backend to use when creating a driver.
//!
//! # Backends
//!
//! - [`DriverKind::WebDriver`] — Selenium/WebDriver protocol via `thirtyfour` (always available)
//! - [`DriverKind::Cdp`] — Chrome DevTools Protocol via `chromiumoxide` (feature: `cdp`)
//!
//! # Example
//!
//! ```rust,ignore
//! use utam_core::drivers::DriverKind;
//!
//! // Use WebDriver backend
//! let kind = DriverKind::WebDriver;
//!
//! // Use CDP backend (requires `cdp` feature)
//! #[cfg(feature = "cdp")]
//! let kind = DriverKind::Cdp;
//! ```

mod webdriver;

#[cfg(feature = "cdp")]
mod cdp;

pub use webdriver::ThirtyfourDriver;

#[cfg(feature = "cdp")]
pub use cdp::CdpDriver;

/// Selects the browser automation backend to use for a test or session.
///
/// Passed to driver factory functions to create the appropriate adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverKind {
    /// WebDriver / Selenium protocol via `thirtyfour`.
    ///
    /// Requires a running ChromeDriver (or other WebDriver server).
    WebDriver,

    /// Chrome DevTools Protocol via `chromiumoxide`.
    ///
    /// Connects directly to Chrome's CDP debugging port — no ChromeDriver needed.
    /// Provides lower-level access to console logs, network events, and DOM snapshots.
    #[cfg(feature = "cdp")]
    Cdp,
}

impl std::fmt::Display for DriverKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriverKind::WebDriver => write!(f, "webdriver"),
            #[cfg(feature = "cdp")]
            DriverKind::Cdp => write!(f, "cdp"),
        }
    }
}
