//! Browser driver abstraction
//!
//! The [`UtamDriver`] and [`ElementHandle`] traits decouple the runtime
//! from any specific browser automation protocol. The bundled
//! [`ThirtyfourDriver`] adapter connects to WebDriver/Selenium via
//! the `thirtyfour` crate; alternative adapters (CDP, Playwright) can
//! be implemented externally.
//!
//! # Example
//!
//! ```rust,ignore
//! use utam_runtime::driver::ThirtyfourDriver;
//!
//! let driver = ThirtyfourDriver::connect(Browser::Chrome, "http://localhost:4444").await?;
//! driver.navigate("https://login.salesforce.com").await?;
//! let el = driver.find_element(Selector::Css(".submit")).await?;
//! el.click().await?;
//! ```

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
/// This is the protocol-agnostic equivalent of `thirtyfour::WebElement`.
/// Implementors wrap whatever the underlying driver uses to reference
/// an element (WebDriver element ID, CDP object ID, etc.).
#[async_trait]
pub trait ElementHandle: Send + Sync + Debug + 'static {
    /// Clone this handle into a boxed trait object
    fn clone_handle(&self) -> Box<dyn ElementHandle>;
    // -- Queries --

    /// Get the visible text content
    async fn text(&self) -> RuntimeResult<String>;

    /// Get an attribute value (returns None if absent)
    async fn attribute(&self, name: &str) -> RuntimeResult<Option<String>>;

    /// Get the `class` attribute value
    async fn class_name(&self) -> RuntimeResult<String>;

    /// Get a computed CSS property value
    async fn css_value(&self, name: &str) -> RuntimeResult<String>;

    /// Get the `value` property (for inputs)
    async fn property_value(&self) -> RuntimeResult<String>;

    /// Get the `title` attribute
    async fn title(&self) -> RuntimeResult<String>;

    // -- State --

    /// Whether the element is displayed (visible)
    async fn is_displayed(&self) -> RuntimeResult<bool>;

    /// Whether the element is enabled
    async fn is_enabled(&self) -> RuntimeResult<bool>;

    /// Whether the element is still attached to the DOM
    async fn is_present(&self) -> RuntimeResult<bool>;

    /// Whether this element currently has focus
    async fn is_focused(&self) -> RuntimeResult<bool>;

    // -- Actions --

    /// Click the element
    async fn click(&self) -> RuntimeResult<()>;

    /// Double-click the element
    async fn double_click(&self) -> RuntimeResult<()>;

    /// Right-click (context menu) the element
    async fn right_click(&self) -> RuntimeResult<()>;

    /// Click and hold the element
    async fn click_and_hold(&self) -> RuntimeResult<()>;

    /// Focus the element
    async fn focus(&self) -> RuntimeResult<()>;

    /// Remove focus from the element
    async fn blur(&self) -> RuntimeResult<()>;

    /// Type text into the element (appends)
    async fn send_keys(&self, text: &str) -> RuntimeResult<()>;

    /// Clear the element's content
    async fn clear(&self) -> RuntimeResult<()>;

    /// Press a special key
    async fn press_key(&self, key: &str) -> RuntimeResult<()>;

    /// Scroll the element into view
    async fn scroll_into_view(&self) -> RuntimeResult<()>;

    /// Drag this element by a pixel offset
    async fn drag_by_offset(&self, x: i64, y: i64) -> RuntimeResult<()>;

    // -- Shadow DOM --

    /// Get the shadow root of this element, if any
    async fn shadow_root(&self) -> RuntimeResult<Option<Box<dyn ShadowRootHandle>>>;

    // -- Sub-queries --

    /// Find a single child element matching the selector
    async fn find_element(&self, selector: &Selector) -> RuntimeResult<Box<dyn ElementHandle>>;

    /// Find all child elements matching the selector
    async fn find_elements(
        &self,
        selector: &Selector,
    ) -> RuntimeResult<Vec<Box<dyn ElementHandle>>>;
}

/// Handle to a shadow root, allowing queries within it.
#[async_trait]
pub trait ShadowRootHandle: Send + Sync + Debug {
    /// Find a single element within the shadow root
    async fn find_element(&self, selector: &Selector) -> RuntimeResult<Box<dyn ElementHandle>>;

    /// Find all elements within the shadow root
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
    /// Navigate to a URL
    async fn navigate(&self, url: &str) -> RuntimeResult<()>;

    /// Get the current page URL
    async fn current_url(&self) -> RuntimeResult<String>;

    /// Get the current page title
    async fn title(&self) -> RuntimeResult<String>;

    /// Take a full-page screenshot as PNG bytes
    async fn screenshot_png(&self) -> RuntimeResult<Vec<u8>>;

    /// Execute JavaScript and return the result as JSON
    async fn execute_script(
        &self,
        script: &str,
        args: Vec<serde_json::Value>,
    ) -> RuntimeResult<serde_json::Value>;

    /// Find a single element on the page
    async fn find_element(&self, selector: &Selector) -> RuntimeResult<Box<dyn ElementHandle>>;

    /// Find all elements matching the selector
    async fn find_elements(
        &self,
        selector: &Selector,
    ) -> RuntimeResult<Vec<Box<dyn ElementHandle>>>;

    /// Wait for an element to be present, polling with the given timeout
    async fn wait_for_element(
        &self,
        selector: &Selector,
        timeout: Duration,
    ) -> RuntimeResult<Box<dyn ElementHandle>>;

    /// Close the browser session
    async fn quit(&self) -> RuntimeResult<()>;
}

// ===========================================================================
// thirtyfour adapter
// ===========================================================================

use thirtyfour::prelude::*;

/// Adapter connecting [`UtamDriver`] to `thirtyfour` (WebDriver protocol).
pub struct ThirtyfourDriver {
    inner: WebDriver,
}

impl ThirtyfourDriver {
    /// Wrap an existing `thirtyfour::WebDriver`
    pub fn new(driver: WebDriver) -> Self {
        Self { inner: driver }
    }

    /// Get a reference to the underlying `thirtyfour::WebDriver`
    pub fn inner(&self) -> &WebDriver {
        &self.inner
    }
}

fn selector_to_by(sel: &Selector) -> By {
    match sel {
        Selector::Css(s) => By::Css(s),
        Selector::AccessibilityId(s) => By::Id(s),
        Selector::IosClassChain(s) => By::Tag(s), // best-effort mapping
        Selector::AndroidUiAutomator(s) => By::Tag(s),
    }
}

#[async_trait]
impl UtamDriver for ThirtyfourDriver {
    async fn navigate(&self, url: &str) -> RuntimeResult<()> {
        self.inner.goto(url).await.map_err(to_rt)?;
        Ok(())
    }

    async fn current_url(&self) -> RuntimeResult<String> {
        Ok(self.inner.current_url().await.map_err(to_rt)?.to_string())
    }

    async fn title(&self) -> RuntimeResult<String> {
        self.inner.title().await.map_err(to_rt)
    }

    async fn screenshot_png(&self) -> RuntimeResult<Vec<u8>> {
        self.inner.screenshot_as_png().await.map_err(to_rt)
    }

    async fn execute_script(
        &self,
        script: &str,
        args: Vec<serde_json::Value>,
    ) -> RuntimeResult<serde_json::Value> {
        let result = self.inner.execute(script, args).await.map_err(to_rt)?;
        Ok(result.json().clone())
    }

    async fn find_element(&self, selector: &Selector) -> RuntimeResult<Box<dyn ElementHandle>> {
        let el = self.inner.find(selector_to_by(selector)).await.map_err(to_rt)?;
        Ok(Box::new(ThirtyfourElement(el)))
    }

    async fn find_elements(
        &self,
        selector: &Selector,
    ) -> RuntimeResult<Vec<Box<dyn ElementHandle>>> {
        let els = self.inner.find_all(selector_to_by(selector)).await.map_err(to_rt)?;
        Ok(els
            .into_iter()
            .map(|e| Box::new(ThirtyfourElement(e)) as Box<dyn ElementHandle>)
            .collect())
    }

    async fn wait_for_element(
        &self,
        selector: &Selector,
        timeout: Duration,
    ) -> RuntimeResult<Box<dyn ElementHandle>> {
        let by = selector_to_by(selector);
        let driver = self.inner.clone();
        utam_core::wait::wait_for(
            || async {
                match driver.find(by.clone()).await {
                    Ok(el) => Ok(Some(el)),
                    Err(_) => Ok(None),
                }
            },
            &utam_core::wait::WaitConfig { timeout, ..Default::default() },
            &format!("element with selector {selector:?}"),
        )
        .await
        .map(|el| Box::new(ThirtyfourElement(el)) as Box<dyn ElementHandle>)
        .map_err(Into::into)
    }

    async fn quit(&self) -> RuntimeResult<()> {
        self.inner.clone().quit().await.map_err(to_rt)
    }
}

// ---------------------------------------------------------------------------
// thirtyfour ElementHandle
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ThirtyfourElement(WebElement);

/// Convert a WebDriverError to RuntimeError
fn to_rt(e: WebDriverError) -> crate::error::RuntimeError {
    crate::error::RuntimeError::Utam(utam_core::error::UtamError::WebDriver(e))
}

#[async_trait]
impl ElementHandle for ThirtyfourElement {
    fn clone_handle(&self) -> Box<dyn ElementHandle> {
        Box::new(self.clone())
    }

    async fn text(&self) -> RuntimeResult<String> {
        self.0.text().await.map_err(to_rt)
    }

    async fn attribute(&self, name: &str) -> RuntimeResult<Option<String>> {
        self.0.attr(name).await.map_err(to_rt)
    }

    async fn class_name(&self) -> RuntimeResult<String> {
        Ok(self.0.class_name().await.map_err(to_rt)?.unwrap_or_default())
    }

    async fn css_value(&self, name: &str) -> RuntimeResult<String> {
        self.0.css_value(name).await.map_err(to_rt)
    }

    async fn property_value(&self) -> RuntimeResult<String> {
        Ok(self.0.value().await.map_err(to_rt)?.unwrap_or_default())
    }

    async fn title(&self) -> RuntimeResult<String> {
        Ok(self.attribute("title").await?.unwrap_or_default())
    }

    async fn is_displayed(&self) -> RuntimeResult<bool> {
        self.0.is_displayed().await.map_err(to_rt)
    }

    async fn is_enabled(&self) -> RuntimeResult<bool> {
        self.0.is_enabled().await.map_err(to_rt)
    }

    async fn is_present(&self) -> RuntimeResult<bool> {
        match self.0.tag_name().await {
            Ok(_) => Ok(true),
            Err(e) => {
                let s = e.to_string().to_lowercase();
                if s.contains("stale") || s.contains("no such element") {
                    Ok(false)
                } else {
                    Err(to_rt(e))
                }
            }
        }
    }

    async fn is_focused(&self) -> RuntimeResult<bool> {
        let result = self
            .0
            .handle
            .execute(
                "return document.activeElement === arguments[0];",
                vec![self.0.to_json().map_err(to_rt)?],
            )
            .await
            .map_err(to_rt)?;
        Ok(result.json().as_bool().unwrap_or(false))
    }

    async fn click(&self) -> RuntimeResult<()> {
        self.0.click().await.map_err(to_rt)
    }

    async fn double_click(&self) -> RuntimeResult<()> {
        let driver = WebDriver { handle: self.0.handle.clone() };
        driver.action_chain().double_click_element(&self.0).perform().await.map_err(to_rt)
    }

    async fn right_click(&self) -> RuntimeResult<()> {
        let driver = WebDriver { handle: self.0.handle.clone() };
        driver.action_chain().context_click_element(&self.0).perform().await.map_err(to_rt)
    }

    async fn click_and_hold(&self) -> RuntimeResult<()> {
        let driver = WebDriver { handle: self.0.handle.clone() };
        driver.action_chain().click_and_hold_element(&self.0).perform().await.map_err(to_rt)
    }

    async fn focus(&self) -> RuntimeResult<()> {
        self.0.focus().await.map_err(to_rt)
    }

    async fn blur(&self) -> RuntimeResult<()> {
        let driver = WebDriver { handle: self.0.handle.clone() };
        driver
            .execute("arguments[0].blur();", vec![self.0.to_json().map_err(to_rt)?])
            .await
            .map_err(to_rt)?;
        Ok(())
    }

    async fn send_keys(&self, text: &str) -> RuntimeResult<()> {
        self.0.send_keys(text).await.map_err(to_rt)
    }

    async fn clear(&self) -> RuntimeResult<()> {
        self.0.clear().await.map_err(to_rt)
    }

    async fn press_key(&self, key: &str) -> RuntimeResult<()> {
        let tf_key = match key {
            "Enter" => thirtyfour::Key::Enter,
            "Tab" => thirtyfour::Key::Tab,
            "Escape" => thirtyfour::Key::Escape,
            "Backspace" => thirtyfour::Key::Backspace,
            "Delete" => thirtyfour::Key::Delete,
            "ArrowUp" => thirtyfour::Key::Up,
            "ArrowDown" => thirtyfour::Key::Down,
            "ArrowLeft" => thirtyfour::Key::Left,
            "ArrowRight" => thirtyfour::Key::Right,
            "Home" => thirtyfour::Key::Home,
            "End" => thirtyfour::Key::End,
            "PageUp" => thirtyfour::Key::PageUp,
            "PageDown" => thirtyfour::Key::PageDown,
            "Space" => thirtyfour::Key::Space,
            _ => {
                return Err(crate::error::RuntimeError::ArgumentTypeMismatch {
                    expected: "valid key name".into(),
                    actual: key.into(),
                })
            }
        };
        self.0.send_keys(tf_key).await.map_err(to_rt)
    }

    async fn scroll_into_view(&self) -> RuntimeResult<()> {
        let driver = WebDriver { handle: self.0.handle.clone() };
        driver
            .execute("arguments[0].scrollIntoView();", vec![self.0.to_json().map_err(to_rt)?])
            .await
            .map_err(to_rt)?;
        Ok(())
    }

    async fn drag_by_offset(&self, x: i64, y: i64) -> RuntimeResult<()> {
        let driver = WebDriver { handle: self.0.handle.clone() };
        driver
            .action_chain()
            .drag_and_drop_element_by_offset(&self.0, x, y)
            .perform()
            .await
            .map_err(to_rt)
    }

    async fn shadow_root(&self) -> RuntimeResult<Option<Box<dyn ShadowRootHandle>>> {
        match self.0.get_shadow_root().await {
            Ok(shadow) => Ok(Some(Box::new(ThirtyfourShadowRoot(shadow)))),
            Err(e) => {
                let s = e.to_string().to_lowercase();
                if s.contains("no such shadow root") || s.contains("no shadow root") {
                    Ok(None)
                } else {
                    Err(to_rt(e))
                }
            }
        }
    }

    async fn find_element(&self, selector: &Selector) -> RuntimeResult<Box<dyn ElementHandle>> {
        let el = self.0.find(selector_to_by(selector)).await.map_err(to_rt)?;
        Ok(Box::new(ThirtyfourElement(el)))
    }

    async fn find_elements(
        &self,
        selector: &Selector,
    ) -> RuntimeResult<Vec<Box<dyn ElementHandle>>> {
        let els = self.0.find_all(selector_to_by(selector)).await.map_err(to_rt)?;
        Ok(els
            .into_iter()
            .map(|e| Box::new(ThirtyfourElement(e)) as Box<dyn ElementHandle>)
            .collect())
    }
}

// ---------------------------------------------------------------------------
// thirtyfour ShadowRootHandle
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct ThirtyfourShadowRoot(WebElement);

#[async_trait]
impl ShadowRootHandle for ThirtyfourShadowRoot {
    async fn find_element(&self, selector: &Selector) -> RuntimeResult<Box<dyn ElementHandle>> {
        let el = self.0.find(selector_to_by(selector)).await.map_err(to_rt)?;
        Ok(Box::new(ThirtyfourElement(el)))
    }

    async fn find_elements(
        &self,
        selector: &Selector,
    ) -> RuntimeResult<Vec<Box<dyn ElementHandle>>> {
        let els = self.0.find_all(selector_to_by(selector)).await.map_err(to_rt)?;
        Ok(els
            .into_iter()
            .map(|e| Box::new(ThirtyfourElement(e)) as Box<dyn ElementHandle>)
            .collect())
    }
}

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
    fn test_selector_to_by() {
        let by = selector_to_by(&Selector::Css(".foo".into()));
        assert!(format!("{by:?}").contains("Css"));
    }

    #[test]
    fn test_selector_serde_roundtrip() {
        let sel = Selector::Css("div.test".into());
        let json = serde_json::to_string(&sel).unwrap();
        let back: Selector = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, Selector::Css(s) if s == "div.test"));
    }
}
