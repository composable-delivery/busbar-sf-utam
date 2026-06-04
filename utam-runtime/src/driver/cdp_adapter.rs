//! Chrome DevTools Protocol adapter via the `chromiumoxide` crate.
//!
//! Provides [`CdpDriver`] with browser state checkpointing (cookies +
//! storage + URL) for efficient test resumption.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chromiumoxide::browser::Browser;
use chromiumoxide::cdp::browser_protocol::network::{CookieParam, CookieSameSite, TimeSinceEpoch};
use chromiumoxide::cdp::js_protocol::runtime::{
    CallArgument, CallFunctionOnParams, CallFunctionOnReturns, EvaluateParams, GetPropertiesParams,
    RemoteObjectId,
};
use chromiumoxide::error::CdpError;
use chromiumoxide::page::Page;
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
    // Broad "not found" match — chromiumoxide's variants include:
    //   CdpError::NotFound              → "Element was not found"
    //   "no such element"               (Chrome DevTools protocol error)
    //   "node with given id not found"  (runtime domain)
    //   "could not find node"           (DOM domain)
    //   "unable to locate"              (high-level)
    if lower.contains("not found")
        || lower.contains("no such element")
        || lower.contains("unable to locate")
        || lower.contains("could not find")
    {
        return RuntimeError::ElementNotFound { element: "<cdp>".into(), reason: msg };
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

    /// Capture a [`SessionState`] snapshot of the current browser: the full
    /// cookie jar plus `localStorage` / `sessionStorage` and the current URL.
    ///
    /// Cookies are read via the CDP **Network** domain (`Network.getCookies`),
    /// *not* `document.cookie`. This is the whole point of the upgrade: the
    /// Salesforce session cookie (`sid`) is **HttpOnly** and therefore invisible
    /// to `document.cookie` — a JS-based capture silently drops the very cookie
    /// that constitutes the authenticated session. The Network domain returns
    /// HttpOnly/Secure cookies, so the captured state can actually warm-start a
    /// logged-in org.
    pub async fn save_checkpoint(&self) -> RuntimeResult<SessionState> {
        let url = self.page.url().await.map_err(to_rt)?.unwrap_or_default();

        let cookies = self
            .page
            .get_cookies()
            .await
            .map_err(to_rt)?
            .into_iter()
            .map(|c| CookieData {
                name: c.name,
                value: c.value,
                domain: c.domain,
                path: c.path,
                // CDP reports a negative `expires` (and `session = true`) for
                // session cookies; normalize those to `None`.
                expires: if c.session || c.expires < 0.0 { None } else { Some(c.expires) },
                http_only: c.http_only,
                secure: c.secure,
                same_site: c.same_site.map(|s| {
                    match s {
                        CookieSameSite::Strict => "Strict",
                        CookieSameSite::Lax => "Lax",
                        CookieSameSite::None => "None",
                    }
                    .to_string()
                }),
            })
            .collect();

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

        Ok(SessionState { url, cookies, local_storage, session_storage })
    }

    /// Restore a previously captured [`SessionState`] onto this browser.
    ///
    /// Order matters: cookies are set **before** navigation so the page load is
    /// already authenticated (each `CookieParam` carries its own domain/path, so
    /// `Network.setCookies` works with no page loaded). Storage is restored
    /// **after** navigation, because `localStorage`/`sessionStorage` are
    /// origin-scoped and only writable once the document for that origin exists.
    pub async fn restore_checkpoint(&self, checkpoint: &SessionState) -> RuntimeResult<()> {
        // 1) Cookies first — authenticates the navigation in step 2.
        if !checkpoint.cookies.is_empty() {
            let params = checkpoint
                .cookies
                .iter()
                .map(|c| {
                    let mut b = CookieParam::builder()
                        .name(c.name.clone())
                        .value(c.value.clone())
                        .domain(c.domain.clone())
                        .path(c.path.clone())
                        .secure(c.secure)
                        .http_only(c.http_only);
                    if let Some(exp) = c.expires {
                        b = b.expires(TimeSinceEpoch::new(exp));
                    }
                    if let Some(ss) = &c.same_site {
                        if let Ok(parsed) = ss.parse::<CookieSameSite>() {
                            b = b.same_site(parsed);
                        }
                    }
                    b.build()
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| RuntimeError::UnsupportedAction {
                    action: "restore_checkpoint cookies".into(),
                    element_type: e,
                })?;
            self.page.set_cookies(params).await.map_err(to_rt)?;
        }

        // 2) Navigate (now carrying the restored cookies).
        self.page.goto(&checkpoint.url).await.map_err(to_rt)?;

        // 3) Storage — origin-scoped, so only after the document exists.
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

/// Serializable snapshot of a browser session — the full cookie jar plus
/// `localStorage` / `sessionStorage` and the captured URL.
///
/// This is designed to round-trip through an **external store** (Neon/Redis):
/// it derives `Serialize`/`Deserialize` and contains no live handles, so a
/// stateless/serverless host can persist it between invocations and replay it
/// onto any fresh (or pooled) remote browser via [`CdpDriver::restore_checkpoint`].
/// That makes the browser fungible — the authoritative state lives in the store,
/// not in a long-lived process — which is what serverless hosting requires, and
/// it doubles as a local warm-start (snapshot a logged-in org once, skip the
/// re-login on every author→test→heal cycle).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// Page URL at capture time; the restore target.
    pub url: String,
    /// Full cookie jar, captured via the CDP Network domain so **HttpOnly**
    /// cookies (notably Salesforce's `sid`) are included.
    pub cookies: Vec<CookieData>,
    /// `localStorage` for the captured origin, as a JSON object string.
    pub local_storage: String,
    /// `sessionStorage` for the captured origin, as a JSON object string.
    pub session_storage: String,
}

/// A single cookie in a [`SessionState`]. Mirrors the fields needed to faithfully
/// re-create the cookie on restore (CDP-agnostic so the persisted JSON is stable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieData {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    /// Seconds since epoch; `None` for a session cookie.
    pub expires: Option<f64>,
    pub http_only: bool,
    pub secure: bool,
    /// `"Strict"` | `"Lax"` | `"None"`, if the cookie set one.
    pub same_site: Option<String>,
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
        let css = css_selector(selector);
        match ObjElement::query_document(&self.page, css).await.map_err(to_rt)? {
            Some(el) => {
                Ok(Box::new(CdpElement { inner: Arc::new(el), page: Arc::clone(&self.page) }))
            }
            None => Err(RuntimeError::ElementNotFound {
                element: css.to_string(),
                reason: "no match for selector".into(),
            }),
        }
    }

    async fn find_elements(
        &self,
        selector: &Selector,
    ) -> RuntimeResult<Vec<Box<dyn ElementHandle>>> {
        // "find all matches, possibly none" → an empty Vec for zero matches
        // (matching WebDriver and the trait contract). querySelectorAll on an
        // empty match returns `[]`, so this falls out naturally.
        let els = ObjElement::query_all_document(&self.page, css_selector(selector))
            .await
            .map_err(to_rt)?;
        Ok(wrap_elements(els, &self.page))
    }

    async fn wait_for_element(
        &self,
        selector: &Selector,
        timeout: Duration,
    ) -> RuntimeResult<Box<dyn ElementHandle>> {
        let css = css_selector(selector).to_string();
        let page = Arc::clone(&self.page);
        // Capture the most recent underlying find error. A transient error
        // during navigation is still swallowed-and-retried (correct), but if
        // the find fails *persistently* we surface its real cause instead of an
        // opaque "Timeout waiting for <selector>" — which otherwise hides
        // whether the element is genuinely absent vs. a context-destroyed /
        // stale-node / DOM-agent error that no amount of waiting will resolve.
        let last_err: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
        // Bound each individual find attempt. On a busy Lightning record page
        // the CDP `DOM.getDocument`/`querySelector` call can *hang* — not error —
        // because chromiumoxide's single handler task is saturated by the page's
        // flood of CDP events (DOM mutations / network) while related lists load.
        // A single in-flight find would then consume the entire wait budget and
        // report a bare timeout with no cause. Capping each attempt cancels a hung
        // call so a later attempt can succeed once the page settles, and records
        // the hang so it is no longer invisible.
        let attempt = std::time::Duration::from_secs(5).min(timeout);
        let outcome = utam_core::wait::wait_for(
            || async {
                match tokio::time::timeout(attempt, ObjElement::query_document(&page, &css)).await {
                    Ok(Ok(Some(el))) => Ok(Some(el)),
                    Ok(Ok(None)) => Ok(None), // not present yet — keep polling
                    Ok(Err(e)) => {
                        *last_err.lock().unwrap() = Some(e.to_string());
                        Ok(None)
                    }
                    Err(_) => {
                        *last_err.lock().unwrap() = Some(format!(
                            "find did not return within {attempt:?} (CDP command hang)"
                        ));
                        Ok(None)
                    }
                }
            },
            &utam_core::wait::WaitConfig { timeout, ..Default::default() },
            &format!("CDP element {selector:?}"),
        )
        .await;
        match outcome {
            Ok(el) => {
                Ok(Box::new(CdpElement { inner: Arc::new(el), page: Arc::clone(&self.page) }))
            }
            Err(timeout_err) => match last_err.into_inner().unwrap() {
                Some(detail) => Err(RuntimeError::ElementNotFound {
                    element: css,
                    reason: format!("not found within {timeout:?}; last find error: {detail}"),
                }),
                None => Err(timeout_err.into()),
            },
        }
    }

    async fn quit(&self) -> RuntimeResult<()> {
        Ok(()) // chromiumoxide cleans up on drop
    }
}

// ---------------------------------------------------------------------------
// ObjElement — a Runtime-domain element handle
// ---------------------------------------------------------------------------

/// An element handle backed by a Runtime **object id**, not a DOM `NodeId`.
///
/// All resolution and operations go through `Runtime.evaluate` /
/// `Runtime.callFunctionOn` — never the CDP **DOM** domain. On a busy Lightning
/// record page the DOM domain's `getDocument`/`querySelector` responses *hang*
/// (chromiumoxide's handler is starved by the page's DOM-mutation event flood),
/// while the Runtime domain stays responsive (~1ms). Resolving via JS is what
/// makes finds reliable on heavy pages. It mirrors the slice of chromiumoxide's
/// `Element` API this adapter used, so the `ElementHandle` impl below is
/// unchanged.
#[derive(Debug)]
struct ObjElement {
    object_id: RemoteObjectId,
    page: Arc<Page>,
}

impl ObjElement {
    /// Resolve `document.querySelector(css)` to a handle, or `None`.
    async fn query_document(page: &Arc<Page>, css: &str) -> Result<Option<ObjElement>, CdpError> {
        Self::eval_object(page, format!("document.querySelector({})", js_arg(css))).await
    }

    /// Resolve `document.querySelectorAll(css)` to handles.
    async fn query_all_document(page: &Arc<Page>, css: &str) -> Result<Vec<ObjElement>, CdpError> {
        Self::eval_array(page, format!("Array.from(document.querySelectorAll({}))", js_arg(css)))
            .await
    }

    async fn eval_object(page: &Arc<Page>, expr: String) -> Result<Option<ObjElement>, CdpError> {
        let params = EvaluateParams::builder()
            .expression(expr)
            .return_by_value(false)
            .await_promise(false)
            .build()
            .map_err(CdpError::msg)?;
        let object_id = page.execute(params).await?.result.result.object_id;
        Ok(object_id.map(|object_id| ObjElement { object_id, page: Arc::clone(page) }))
    }

    async fn eval_array(page: &Arc<Page>, expr: String) -> Result<Vec<ObjElement>, CdpError> {
        let params = EvaluateParams::builder()
            .expression(expr)
            .return_by_value(false)
            .await_promise(false)
            .build()
            .map_err(CdpError::msg)?;
        match page.execute(params).await?.result.result.object_id {
            None => Ok(Vec::new()),
            Some(arr) => array_elements(page, &arr).await,
        }
    }

    /// Call a function on this element, result returned by value.
    async fn call_js_fn(
        &self,
        decl: impl Into<String>,
        await_promise: bool,
    ) -> Result<CallFunctionOnReturns, CdpError> {
        let params = CallFunctionOnParams::builder()
            .object_id(self.object_id.clone())
            .function_declaration(decl)
            .return_by_value(true)
            .await_promise(await_promise)
            .build()
            .map_err(CdpError::msg)?;
        Ok(self.page.execute(params).await?.result)
    }

    /// Call a function returning a single element handle (or `None`).
    async fn call_object(
        &self,
        decl: &str,
        args: Vec<serde_json::Value>,
    ) -> Result<Option<ObjElement>, CdpError> {
        let object_id = self.call_raw(decl, args).await?.result.object_id;
        Ok(object_id.map(|object_id| ObjElement { object_id, page: Arc::clone(&self.page) }))
    }

    /// Call a function returning an array of element handles.
    async fn call_array(
        &self,
        decl: &str,
        args: Vec<serde_json::Value>,
    ) -> Result<Vec<ObjElement>, CdpError> {
        match self.call_raw(decl, args).await?.result.object_id {
            None => Ok(Vec::new()),
            Some(arr) => array_elements(&self.page, &arr).await,
        }
    }

    async fn call_raw(
        &self,
        decl: &str,
        args: Vec<serde_json::Value>,
    ) -> Result<CallFunctionOnReturns, CdpError> {
        let arguments = if args.is_empty() {
            None
        } else {
            Some(
                args.into_iter()
                    .map(|v| CallArgument {
                        value: Some(v),
                        unserializable_value: None,
                        object_id: None,
                    })
                    .collect(),
            )
        };
        let params = CallFunctionOnParams {
            arguments,
            ..CallFunctionOnParams::builder()
                .object_id(self.object_id.clone())
                .function_declaration(decl.to_string())
                .return_by_value(false)
                .await_promise(false)
                .build()
                .map_err(CdpError::msg)?
        };
        Ok(self.page.execute(params).await?.result)
    }

    async fn inner_text(&self) -> Result<Option<String>, CdpError> {
        let ret = self
            .call_js_fn(
                "function(){ return this.innerText != null ? this.innerText : this.textContent; }",
                false,
            )
            .await?;
        Ok(ret.result.value.and_then(|v| v.as_str().map(String::from)))
    }

    async fn attribute(&self, name: &str) -> Result<Option<String>, CdpError> {
        let ret = self
            .call_js_fn(
                format!("function(){{ return this.getAttribute({}); }}", js_arg(name)),
                false,
            )
            .await?;
        Ok(ret.result.value.and_then(|v| v.as_str().map(String::from)))
    }

    async fn type_str(&self, text: &str) -> Result<(), CdpError> {
        // JS-only typing (focus, append, fire input/change) — avoids the Input
        // and DOM domains entirely.
        self.call_js_fn(
            format!(
                "function(){{ this.focus(); this.value = (this.value || '') + {}; \
                 this.dispatchEvent(new Event('input', {{bubbles:true}})); \
                 this.dispatchEvent(new Event('change', {{bubbles:true}})); }}",
                js_arg(text)
            ),
            false,
        )
        .await?;
        Ok(())
    }

    async fn press_key(&self, key: &str) -> Result<(), CdpError> {
        self.call_js_fn(
            format!(
                "function(){{ var k = {}; ['keydown','keyup'].forEach(function(t){{ \
                 this.dispatchEvent(new KeyboardEvent(t, {{key:k, bubbles:true, cancelable:true}})); \
                 }}.bind(this)); }}",
                js_arg(key)
            ),
            false,
        )
        .await?;
        Ok(())
    }

    async fn scroll_into_view(&self) -> Result<(), CdpError> {
        self.call_js_fn(
            "function(){ this.scrollIntoView({block:'center', inline:'center'}); }",
            false,
        )
        .await?;
        Ok(())
    }

    async fn find_element(&self, css: &str) -> Result<ObjElement, CdpError> {
        match self
            .call_object(
                "function(s){ return this.querySelector(s); }",
                vec![serde_json::json!(css)],
            )
            .await?
        {
            Some(el) => Ok(el),
            None => Err(CdpError::msg(format!("Element not found: {css}"))),
        }
    }

    async fn find_elements(&self, css: &str) -> Result<Vec<ObjElement>, CdpError> {
        self.call_array(
            "function(s){ return Array.from(this.querySelectorAll(s)); }",
            vec![serde_json::json!(css)],
        )
        .await
    }

    async fn shadow_query(&self, css: &str) -> Result<Option<ObjElement>, CdpError> {
        self.call_object(
            "function(s){ return this.shadowRoot ? this.shadowRoot.querySelector(s) : null; }",
            vec![serde_json::json!(css)],
        )
        .await
    }

    async fn shadow_query_all(&self, css: &str) -> Result<Vec<ObjElement>, CdpError> {
        self.call_array(
            "function(s){ return this.shadowRoot ? Array.from(this.shadowRoot.querySelectorAll(s)) : []; }",
            vec![serde_json::json!(css)],
        )
        .await
    }
}

/// Quote a string as a JS literal for safe embedding in a function body.
fn js_arg(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

/// Expand a Runtime array object into element handles via `Runtime.getProperties`.
async fn array_elements(
    page: &Arc<Page>,
    array_id: &RemoteObjectId,
) -> Result<Vec<ObjElement>, CdpError> {
    let params = GetPropertiesParams::builder()
        .object_id(array_id.clone())
        .own_properties(true)
        .build()
        .map_err(CdpError::msg)?;
    let props = page.execute(params).await?.result.result;
    let mut out = Vec::new();
    for p in props {
        if p.name.parse::<usize>().is_err() {
            continue; // skip "length" and any non-index properties
        }
        if let Some(object_id) = p.value.and_then(|o| o.object_id) {
            out.push(ObjElement { object_id, page: Arc::clone(page) });
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// CdpElement
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct CdpElement {
    inner: Arc<ObjElement>,
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
        // Use JavaScript `.click()` rather than CDP's coordinate-based
        // `Input.dispatchMouseEvent`.  The JS method fires the click event
        // directly on the element and propagates it through the DOM in the
        // same way a user click does, which is essential for SPA frameworks
        // (e.g. Lightning) that intercept click events via event delegation
        // and route navigation programmatically.  CDP input events can miss
        // the router when the element's viewport coordinates are imprecise
        // in headless mode.  `focus`/`blur` already use this pattern.
        self.inner.call_js_fn("function(){ this.click(); }", false).await.map_err(to_rt)?;
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
            // Scope shadow queries to *this* host's shadow root via Runtime
            // (`this.shadowRoot.querySelector`), so the traversal is exact
            // rather than relying on global shadow-piercing.
            Ok(Some(Box::new(CdpShadowRoot {
                host: Arc::clone(&self.inner),
                page: Arc::clone(&self.page),
            })))
        } else {
            Ok(None)
        }
    }

    async fn find_element(&self, selector: &Selector) -> RuntimeResult<Box<dyn ElementHandle>> {
        // Scoped child query via `this.querySelector` (Runtime), not the DOM
        // domain. Object handles carry no DOM NodeId, so there is no
        // stale-node case to re-resolve — a detached element simply yields no
        // match.
        let child = self.inner.find_element(css_selector(selector)).await.map_err(to_rt)?;
        Ok(Box::new(CdpElement { inner: Arc::new(child), page: Arc::clone(&self.page) }))
    }

    async fn find_elements(
        &self,
        selector: &Selector,
    ) -> RuntimeResult<Vec<Box<dyn ElementHandle>>> {
        let children = self.inner.find_elements(css_selector(selector)).await.map_err(to_rt)?;
        Ok(wrap_elements(children, &self.page))
    }
}

/// Wrap `ObjElement`s into boxed `ElementHandle`s sharing `page`.
fn wrap_elements(els: Vec<ObjElement>, page: &Arc<Page>) -> Vec<Box<dyn ElementHandle>> {
    els.into_iter()
        .map(|e| {
            Box::new(CdpElement { inner: Arc::new(e), page: Arc::clone(page) })
                as Box<dyn ElementHandle>
        })
        .collect()
}

// ---------------------------------------------------------------------------
// CdpShadowRoot — queries scoped to a host element's shadow root via Runtime
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct CdpShadowRoot {
    host: Arc<ObjElement>,
    page: Arc<Page>,
}

#[async_trait]
impl ShadowRootHandle for CdpShadowRoot {
    async fn find_element(&self, selector: &Selector) -> RuntimeResult<Box<dyn ElementHandle>> {
        let css = css_selector(selector);
        match self.host.shadow_query(css).await.map_err(to_rt)? {
            Some(el) => {
                Ok(Box::new(CdpElement { inner: Arc::new(el), page: Arc::clone(&self.page) }))
            }
            None => Err(RuntimeError::ElementNotFound {
                element: css.to_string(),
                reason: "no match in shadow root".into(),
            }),
        }
    }

    async fn find_elements(
        &self,
        selector: &Selector,
    ) -> RuntimeResult<Vec<Box<dyn ElementHandle>>> {
        let els = self.host.shadow_query_all(css_selector(selector)).await.map_err(to_rt)?;
        Ok(wrap_elements(els, &self.page))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_state_round_trips_through_json() {
        // SessionState is the contract persisted to an external store; lock its
        // serialized shape and prove an HttpOnly cookie survives the round trip.
        let state = SessionState {
            url: "https://example.my.salesforce.com/lightning/page/home".into(),
            cookies: vec![CookieData {
                name: "sid".into(),
                value: "00D...!secret".into(),
                domain: ".my.salesforce.com".into(),
                path: "/".into(),
                expires: None, // session cookie
                http_only: true,
                secure: true,
                same_site: Some("None".into()),
            }],
            local_storage: "{}".into(),
            session_storage: "{}".into(),
        };

        let json = serde_json::to_string(&state).expect("serialize");
        let back: SessionState = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(back.url, state.url);
        assert_eq!(back.cookies.len(), 1);
        let c = &back.cookies[0];
        assert_eq!(c.name, "sid");
        assert!(c.http_only, "HttpOnly flag must survive — it's the whole point");
        assert!(c.secure);
        assert_eq!(c.expires, None);
        assert_eq!(c.same_site.as_deref(), Some("None"));
    }
}
