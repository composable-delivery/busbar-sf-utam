//! Chrome DevTools Protocol adapter via the `chromiumoxide` crate.
//!
//! Provides [`CdpDriver`] with browser state checkpointing (cookies +
//! storage + URL) for efficient test resumption.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chromiumoxide::browser::Browser;
use chromiumoxide::page::Page;
use chromiumoxide::Element;
use futures::StreamExt;
use serde::{Deserialize, Serialize};

use super::{ElementHandle, Selector, ShadowRootHandle, UtamDriver};
use crate::error::{RuntimeError, RuntimeResult};

/// Map a chromiumoxide CDP error into the closest-matching RuntimeError
/// variant so classify() can bucket failures correctly.
///
/// - Element-not-found / selector misses → `ElementNotFound` (→ StaleSelector)
/// - Timeouts / wait failures → `Utam(Timeout)` (→ Timeout)
/// - Everything else → `UnsupportedAction { action: "CDP", ... }` as a
///   last-resort catch-all for genuinely unexpected driver errors.
fn to_rt(e: chromiumoxide::error::CdpError) -> RuntimeError {
    let msg = format!("{e}");
    let lower = msg.to_lowercase();
    if lower.contains("no such element")
        || lower.contains("element not found")
        || lower.contains("unable to locate")
        || lower.contains("node with given id not found")
    {
        return RuntimeError::ElementNotFound {
            element: "<cdp>".into(),
            reason: msg,
        };
    }
    if lower.contains("timeout") || lower.contains("timed out") {
        return RuntimeError::Utam(utam_core::error::UtamError::Timeout { condition: msg });
    }
    RuntimeError::UnsupportedAction { action: "CDP".into(), element_type: msg }
}

fn css_selector(sel: &Selector) -> &str {
    match sel {
        Selector::Css(s)
        | Selector::AccessibilityId(s)
        | Selector::IosClassChain(s)
        | Selector::AndroidUiAutomator(s) => s,
    }
}

/// Extract a JSON value from a `CallFunctionOnReturns`.
fn extract_value(
    ret: chromiumoxide::cdp::js_protocol::runtime::CallFunctionOnReturns,
) -> Option<serde_json::Value> {
    ret.result.value
}

// ---------------------------------------------------------------------------
// CdpDriver
// ---------------------------------------------------------------------------

/// CDP-based browser driver via chromiumoxide.
///
/// Faster than WebDriver and supports checkpointing for test state
/// capture/restore.
pub struct CdpDriver {
    page: Arc<Page>,
    _browser: Browser,
}

impl CdpDriver {
    /// Create from an existing chromiumoxide Page and Browser.
    pub fn new(browser: Browser, page: Page) -> Self {
        Self { page: Arc::new(page), _browser: browser }
    }

    /// Launch a headless Chrome and open a blank page.
    pub async fn launch() -> RuntimeResult<Self> {
        Self::launch_with_config(chromiumoxide::BrowserConfig::builder().build().map_err(|e| {
            RuntimeError::UnsupportedAction { action: "launch".into(), element_type: e }
        })?)
        .await
    }

    /// Launch Chrome with custom config and open a blank page.
    pub async fn launch_with_config(config: chromiumoxide::BrowserConfig) -> RuntimeResult<Self> {
        let (browser, mut handler) = Browser::launch(config).await.map_err(to_rt)?;

        tokio::spawn(async move { while handler.next().await.is_some() {} });

        let page = browser.new_page("about:blank").await.map_err(to_rt)?;
        Ok(Self::new(browser, page))
    }

    /// Get a reference to the underlying chromiumoxide Page.
    pub fn page(&self) -> &Page {
        &self.page
    }

    /// Capture a checkpoint of the current browser state.
    pub async fn save_checkpoint(&self) -> RuntimeResult<BrowserCheckpoint> {
        let url = self.page.url().await.map_err(to_rt)?.unwrap_or_default();

        let cookies = self
            .page
            .evaluate("document.cookie")
            .await
            .map_err(to_rt)?
            .into_value::<String>()
            .unwrap_or_default();

        let local_storage = self
            .page
            .evaluate(
                r#"(() => {
                const o = {};
                for (let i = 0; i < localStorage.length; i++) {
                    const k = localStorage.key(i);
                    o[k] = localStorage.getItem(k);
                }
                return JSON.stringify(o);
            })()"#,
            )
            .await
            .map_err(to_rt)?
            .into_value::<String>()
            .unwrap_or_else(|_| "{}".into());

        let session_storage = self
            .page
            .evaluate(
                r#"(() => {
                const o = {};
                for (let i = 0; i < sessionStorage.length; i++) {
                    const k = sessionStorage.key(i);
                    o[k] = sessionStorage.getItem(k);
                }
                return JSON.stringify(o);
            })()"#,
            )
            .await
            .map_err(to_rt)?
            .into_value::<String>()
            .unwrap_or_else(|_| "{}".into());

        Ok(BrowserCheckpoint { url, cookies, local_storage, session_storage })
    }

    /// Restore a previously captured checkpoint.
    pub async fn restore_checkpoint(&self, checkpoint: &BrowserCheckpoint) -> RuntimeResult<()> {
        self.page.goto(&checkpoint.url).await.map_err(to_rt)?;

        let ls_escaped = checkpoint.local_storage.replace('\\', "\\\\").replace('\'', "\\'");
        self.page
            .evaluate(format!(
                "(() => {{ const o = JSON.parse('{ls_escaped}'); for (const [k, v] of Object.entries(o)) localStorage.setItem(k, v); }})()"
            ))
            .await
            .map_err(to_rt)?;

        let ss_escaped = checkpoint.session_storage.replace('\\', "\\\\").replace('\'', "\\'");
        self.page
            .evaluate(format!(
                "(() => {{ const o = JSON.parse('{ss_escaped}'); for (const [k, v] of Object.entries(o)) sessionStorage.setItem(k, v); }})()"
            ))
            .await
            .map_err(to_rt)?;

        Ok(())
    }
}

/// Serializable snapshot of browser state for checkpoint/restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserCheckpoint {
    pub url: String,
    pub cookies: String,
    pub local_storage: String,
    pub session_storage: String,
}

// ---------------------------------------------------------------------------
// UtamDriver impl
// ---------------------------------------------------------------------------

#[async_trait]
impl UtamDriver for CdpDriver {
    async fn navigate(&self, url: &str) -> RuntimeResult<()> {
        self.page.goto(url).await.map_err(to_rt)?;
        Ok(())
    }

    async fn current_url(&self) -> RuntimeResult<String> {
        Ok(self.page.url().await.map_err(to_rt)?.unwrap_or_default())
    }

    async fn title(&self) -> RuntimeResult<String> {
        self.page.evaluate("document.title").await.map_err(to_rt)?.into_value::<String>().map_err(
            |e| RuntimeError::UnsupportedAction {
                action: "title".into(),
                element_type: format!("{e:?}"),
            },
        )
    }

    async fn screenshot_png(&self) -> RuntimeResult<Vec<u8>> {
        self.page
            .screenshot(
                chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotParams::builder()
                    .format(
                        chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat::Png,
                    )
                    .build(),
            )
            .await
            .map_err(to_rt)
    }

    async fn execute_script(
        &self,
        script: &str,
        _args: Vec<serde_json::Value>,
    ) -> RuntimeResult<serde_json::Value> {
        // WebDriver's executeScript wraps code in a function body where "return"
        // is valid. CDP's Runtime.evaluate evaluates an expression where "return"
        // is a syntax error. Strip leading "return " for compatibility.
        let expr = script.trim();
        let expr = expr.strip_prefix("return ").unwrap_or(expr);
        let expr = expr.strip_suffix(';').unwrap_or(expr);

        let result = self.page.evaluate(expr).await.map_err(to_rt)?;
        Ok(result.into_value::<serde_json::Value>().unwrap_or(serde_json::Value::Null))
    }

    async fn find_element(&self, selector: &Selector) -> RuntimeResult<Box<dyn ElementHandle>> {
        let el = self.page.find_element(css_selector(selector)).await.map_err(to_rt)?;
        Ok(Box::new(CdpElement { inner: Arc::new(el), page: Arc::clone(&self.page) }))
    }

    async fn find_elements(
        &self,
        selector: &Selector,
    ) -> RuntimeResult<Vec<Box<dyn ElementHandle>>> {
        let els = self.page.find_elements(css_selector(selector)).await.map_err(to_rt)?;
        Ok(els
            .into_iter()
            .map(|e| {
                Box::new(CdpElement { inner: Arc::new(e), page: Arc::clone(&self.page) })
                    as Box<dyn ElementHandle>
            })
            .collect())
    }

    async fn wait_for_element(
        &self,
        selector: &Selector,
        timeout: Duration,
    ) -> RuntimeResult<Box<dyn ElementHandle>> {
        let css = css_selector(selector).to_string();
        let page = Arc::clone(&self.page);
        utam_core::wait::wait_for(
            || async {
                match page.find_element(&css).await {
                    Ok(el) => Ok(Some(el)),
                    Err(_) => Ok(None),
                }
            },
            &utam_core::wait::WaitConfig { timeout, ..Default::default() },
            &format!("CDP element {selector:?}"),
        )
        .await
        .map(|el| {
            Box::new(CdpElement { inner: Arc::new(el), page: Arc::clone(&self.page) })
                as Box<dyn ElementHandle>
        })
        .map_err(Into::into)
    }

    async fn quit(&self) -> RuntimeResult<()> {
        Ok(()) // chromiumoxide cleans up on drop
    }
}

// ---------------------------------------------------------------------------
// CdpElement
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct CdpElement {
    inner: Arc<Element>,
    page: Arc<Page>,
}

fn js_bool(ret: chromiumoxide::cdp::js_protocol::runtime::CallFunctionOnReturns) -> bool {
    extract_value(ret).and_then(|v| v.as_bool()).unwrap_or(false)
}

fn js_string(ret: chromiumoxide::cdp::js_protocol::runtime::CallFunctionOnReturns) -> String {
    extract_value(ret).and_then(|v| v.as_str().map(String::from)).unwrap_or_default()
}

#[async_trait]
impl ElementHandle for CdpElement {
    fn clone_handle(&self) -> Box<dyn ElementHandle> {
        Box::new(CdpElement { inner: Arc::clone(&self.inner), page: Arc::clone(&self.page) })
    }

    async fn text(&self) -> RuntimeResult<String> {
        Ok(self.inner.inner_text().await.map_err(to_rt)?.unwrap_or_default())
    }

    async fn attribute(&self, name: &str) -> RuntimeResult<Option<String>> {
        self.inner.attribute(name).await.map_err(to_rt)
    }

    async fn class_name(&self) -> RuntimeResult<String> {
        Ok(self.inner.attribute("class").await.map_err(to_rt)?.unwrap_or_default())
    }

    async fn css_value(&self, name: &str) -> RuntimeResult<String> {
        let script =
            format!("function(){{ return getComputedStyle(this).getPropertyValue('{name}'); }}");
        let ret = self.inner.call_js_fn(script, false).await.map_err(to_rt)?;
        Ok(js_string(ret))
    }

    async fn property_value(&self) -> RuntimeResult<String> {
        let ret = self
            .inner
            .call_js_fn("function(){ return this.value || ''; }", false)
            .await
            .map_err(to_rt)?;
        Ok(js_string(ret))
    }

    async fn title(&self) -> RuntimeResult<String> {
        Ok(self.inner.attribute("title").await.map_err(to_rt)?.unwrap_or_default())
    }

    async fn is_displayed(&self) -> RuntimeResult<bool> {
        let ret = self
            .inner
            .call_js_fn(
                "function(){ var s = getComputedStyle(this); return s.display !== 'none' && s.visibility !== 'hidden'; }",
                false,
            )
            .await
            .map_err(to_rt)?;
        Ok(js_bool(ret))
    }

    async fn is_enabled(&self) -> RuntimeResult<bool> {
        let ret = self
            .inner
            .call_js_fn("function(){ return !this.disabled; }", false)
            .await
            .map_err(to_rt)?;
        Ok(js_bool(ret))
    }

    async fn is_present(&self) -> RuntimeResult<bool> {
        match self.inner.call_js_fn("function(){ return document.contains(this); }", false).await {
            Ok(ret) => Ok(js_bool(ret)),
            Err(_) => Ok(false),
        }
    }

    async fn is_focused(&self) -> RuntimeResult<bool> {
        let ret = self
            .inner
            .call_js_fn("function(){ return document.activeElement === this; }", false)
            .await
            .map_err(to_rt)?;
        Ok(js_bool(ret))
    }

    async fn click(&self) -> RuntimeResult<()> {
        self.inner.click().await.map_err(to_rt)?;
        Ok(())
    }

    async fn double_click(&self) -> RuntimeResult<()> {
        self.inner
            .call_js_fn(
                "function(){ this.dispatchEvent(new MouseEvent('dblclick', {bubbles: true})); }",
                false,
            )
            .await
            .map_err(to_rt)?;
        Ok(())
    }

    async fn right_click(&self) -> RuntimeResult<()> {
        self.inner
            .call_js_fn(
                "function(){ this.dispatchEvent(new MouseEvent('contextmenu', {bubbles: true})); }",
                false,
            )
            .await
            .map_err(to_rt)?;
        Ok(())
    }

    async fn click_and_hold(&self) -> RuntimeResult<()> {
        self.inner
            .call_js_fn(
                "function(){ this.dispatchEvent(new MouseEvent('mousedown', {bubbles: true})); }",
                false,
            )
            .await
            .map_err(to_rt)?;
        Ok(())
    }

    async fn focus(&self) -> RuntimeResult<()> {
        self.inner.call_js_fn("function(){ this.focus(); }", false).await.map_err(to_rt)?;
        Ok(())
    }

    async fn blur(&self) -> RuntimeResult<()> {
        self.inner.call_js_fn("function(){ this.blur(); }", false).await.map_err(to_rt)?;
        Ok(())
    }

    async fn send_keys(&self, text: &str) -> RuntimeResult<()> {
        self.inner.type_str(text).await.map_err(to_rt)?;
        Ok(())
    }

    async fn clear(&self) -> RuntimeResult<()> {
        self.inner
            .call_js_fn(
                "function(){ this.value = ''; this.dispatchEvent(new Event('input', {bubbles: true})); }",
                false,
            )
            .await
            .map_err(to_rt)?;
        Ok(())
    }

    async fn press_key(&self, key: &str) -> RuntimeResult<()> {
        self.inner.press_key(key).await.map_err(to_rt)?;
        Ok(())
    }

    async fn scroll_into_view(&self) -> RuntimeResult<()> {
        self.inner.scroll_into_view().await.map_err(to_rt)?;
        Ok(())
    }

    async fn drag_by_offset(&self, x: i64, y: i64) -> RuntimeResult<()> {
        let script = format!(
            "function(){{ var r = this.getBoundingClientRect(); var cx = r.left + r.width/2; var cy = r.top + r.height/2; this.dispatchEvent(new MouseEvent('mousedown', {{clientX:cx, clientY:cy, bubbles:true}})); this.dispatchEvent(new MouseEvent('mousemove', {{clientX:cx+{x}, clientY:cy+{y}, bubbles:true}})); this.dispatchEvent(new MouseEvent('mouseup', {{clientX:cx+{x}, clientY:cy+{y}, bubbles:true}})); }}"
        );
        self.inner.call_js_fn(script, false).await.map_err(to_rt)?;
        Ok(())
    }

    async fn shadow_root(&self) -> RuntimeResult<Option<Box<dyn ShadowRootHandle>>> {
        let ret = self
            .inner
            .call_js_fn("function(){ return !!this.shadowRoot; }", false)
            .await
            .map_err(to_rt)?;
        if js_bool(ret) {
            // We can't clone Element, so the shadow root queries via the
            // same element reference. This works as long as the element
            // stays alive in the DOM (which it should for shadow hosts).
            Ok(Some(Box::new(CdpShadowRootViaPage { page: Arc::clone(&self.page) })))
        } else {
            Ok(None)
        }
    }

    async fn find_element(&self, selector: &Selector) -> RuntimeResult<Box<dyn ElementHandle>> {
        let child = self.inner.find_element(css_selector(selector)).await.map_err(to_rt)?;
        Ok(Box::new(CdpElement { inner: Arc::new(child), page: Arc::clone(&self.page) }))
    }

    async fn find_elements(
        &self,
        selector: &Selector,
    ) -> RuntimeResult<Vec<Box<dyn ElementHandle>>> {
        let children = self.inner.find_elements(css_selector(selector)).await.map_err(to_rt)?;
        Ok(children
            .into_iter()
            .map(|e| {
                Box::new(CdpElement { inner: Arc::new(e), page: Arc::clone(&self.page) })
                    as Box<dyn ElementHandle>
            })
            .collect())
    }
}

// ---------------------------------------------------------------------------
// CdpShadowRoot — uses page-level queries (chromiumoxide pierces shadow DOM)
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct CdpShadowRootViaPage {
    page: Arc<Page>,
}

#[async_trait]
impl ShadowRootHandle for CdpShadowRootViaPage {
    async fn find_element(&self, selector: &Selector) -> RuntimeResult<Box<dyn ElementHandle>> {
        // chromiumoxide's find_element pierces shadow DOM by default
        let el = self.page.find_element(css_selector(selector)).await.map_err(to_rt)?;
        Ok(Box::new(CdpElement { inner: Arc::new(el), page: Arc::clone(&self.page) }))
    }

    async fn find_elements(
        &self,
        selector: &Selector,
    ) -> RuntimeResult<Vec<Box<dyn ElementHandle>>> {
        let els = self.page.find_elements(css_selector(selector)).await.map_err(to_rt)?;
        Ok(els
            .into_iter()
            .map(|e| {
                Box::new(CdpElement { inner: Arc::new(e), page: Arc::clone(&self.page) })
                    as Box<dyn ElementHandle>
            })
            .collect())
    }
}
