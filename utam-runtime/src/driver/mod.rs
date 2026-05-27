//! Browser driver abstraction
//!
//! The [`UtamDriver`] and [`ElementHandle`] traits decouple the runtime
//! from any specific browser automation protocol. Adapters are feature-gated:
//!
//! - `webdriver` (default): [`ThirtyfourDriver`] for WebDriver/Selenium
//! - `cdp`: [`CdpDriver`] for Chrome DevTools Protocol via chromiumoxide

use std::fmt::Debug;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::RuntimeResult;

// ---------------------------------------------------------------------------
// Selector
// ---------------------------------------------------------------------------

/// Protocol-agnostic selector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Selector {
    /// CSS selector (most common in UTAM)
    Css(String),
    /// Accessibility ID (mobile)
    AccessibilityId(String),
    /// iOS class chain (mobile)
    IosClassChain(String),
    /// Android UiAutomator (mobile)
    AndroidUiAutomator(String),
}

// ---------------------------------------------------------------------------
// ElementHandle
// ---------------------------------------------------------------------------

/// A handle to an element in the browser DOM.
///
/// Protocol-agnostic equivalent of `thirtyfour::WebElement` or
/// `chromiumoxide::Element`. Implementors wrap whatever the underlying
/// driver uses to reference an element.
#[async_trait]
pub trait ElementHandle: Send + Sync + Debug + 'static {
    /// Clone this handle into a boxed trait object
    fn clone_handle(&self) -> Box<dyn ElementHandle>;

    // -- Queries --
    async fn text(&self) -> RuntimeResult<String>;
    async fn attribute(&self, name: &str) -> RuntimeResult<Option<String>>;
    async fn class_name(&self) -> RuntimeResult<String>;
    async fn css_value(&self, name: &str) -> RuntimeResult<String>;
    async fn property_value(&self) -> RuntimeResult<String>;
    async fn title(&self) -> RuntimeResult<String>;

    // -- State --
    async fn is_displayed(&self) -> RuntimeResult<bool>;
    async fn is_enabled(&self) -> RuntimeResult<bool>;
    async fn is_present(&self) -> RuntimeResult<bool>;
    async fn is_focused(&self) -> RuntimeResult<bool>;

    // -- Actions --
    async fn click(&self) -> RuntimeResult<()>;
    async fn double_click(&self) -> RuntimeResult<()>;
    async fn right_click(&self) -> RuntimeResult<()>;
    async fn click_and_hold(&self) -> RuntimeResult<()>;
    async fn focus(&self) -> RuntimeResult<()>;
    async fn blur(&self) -> RuntimeResult<()>;
    async fn send_keys(&self, text: &str) -> RuntimeResult<()>;
    async fn clear(&self) -> RuntimeResult<()>;
    async fn press_key(&self, key: &str) -> RuntimeResult<()>;
    async fn scroll_into_view(&self) -> RuntimeResult<()>;
    async fn drag_by_offset(&self, x: i64, y: i64) -> RuntimeResult<()>;

    // -- Shadow DOM --
    async fn shadow_root(&self) -> RuntimeResult<Option<Box<dyn ShadowRootHandle>>>;

    // -- Sub-queries --
    async fn find_element(&self, selector: &Selector) -> RuntimeResult<Box<dyn ElementHandle>>;
    async fn find_elements(
        &self,
        selector: &Selector,
    ) -> RuntimeResult<Vec<Box<dyn ElementHandle>>>;
}

/// Handle to a shadow root, allowing queries within it.
#[async_trait]
pub trait ShadowRootHandle: Send + Sync + Debug {
    async fn find_element(&self, selector: &Selector) -> RuntimeResult<Box<dyn ElementHandle>>;
    async fn find_elements(
        &self,
        selector: &Selector,
    ) -> RuntimeResult<Vec<Box<dyn ElementHandle>>>;
}

// ---------------------------------------------------------------------------
// UtamDriver
// ---------------------------------------------------------------------------

/// Browser driver abstraction.
///
/// Represents a connection to a browser session. Each test should get
/// its own driver instance for parallel execution.
#[async_trait]
pub trait UtamDriver: Send + Sync {
    async fn navigate(&self, url: &str) -> RuntimeResult<()>;
    async fn current_url(&self) -> RuntimeResult<String>;
    async fn title(&self) -> RuntimeResult<String>;
    async fn screenshot_png(&self) -> RuntimeResult<Vec<u8>>;
    async fn execute_script(
        &self,
        script: &str,
        args: Vec<serde_json::Value>,
    ) -> RuntimeResult<serde_json::Value>;
    async fn find_element(&self, selector: &Selector) -> RuntimeResult<Box<dyn ElementHandle>>;
    async fn find_elements(
        &self,
        selector: &Selector,
    ) -> RuntimeResult<Vec<Box<dyn ElementHandle>>>;
    async fn wait_for_element(
        &self,
        selector: &Selector,
        timeout: Duration,
    ) -> RuntimeResult<Box<dyn ElementHandle>>;
    async fn quit(&self) -> RuntimeResult<()>;
}

// ===========================================================================
// Adapters (feature-gated)
// ===========================================================================

#[cfg(feature = "webdriver")]
mod thirtyfour_adapter;
#[cfg(feature = "webdriver")]
pub use thirtyfour_adapter::ThirtyfourDriver;

#[cfg(feature = "cdp")]
mod cdp_adapter;
#[cfg(feature = "cdp")]
pub use cdp_adapter::CdpDriver;

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selector_variants() {
        let css = Selector::Css("button.submit".into());
        let aid = Selector::AccessibilityId("login-btn".into());
        assert!(matches!(css, Selector::Css(_)));
        assert!(matches!(aid, Selector::AccessibilityId(_)));
    }

    #[test]
    fn test_selector_serde_roundtrip() {
        let sel = Selector::Css("div.test".into());
        let json = serde_json::to_string(&sel).unwrap();
        let back: Selector = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, Selector::Css(s) if s == "div.test"));
    }
}
