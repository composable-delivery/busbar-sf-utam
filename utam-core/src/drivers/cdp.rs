//! CDP adapter for UTAM using `chromiumoxide`.
//!
//! `CdpDriver` launches or connects to a Chrome/Chromium instance via the
//! Chrome DevTools Protocol.  Unlike the WebDriver adapter it does **not**
//! require ChromeDriver — it communicates directly with Chrome over CDP.
//!
//! # Capabilities
//!
//! In addition to standard element interaction, CDP provides:
//! - Console log capture
//! - Network request/response interception
//! - Performance metrics
//! - DOM snapshot without screenshots
//!
//! # Feature flag
//!
//! This module is compiled only when the `cdp` feature is enabled.

use std::ops::{Deref, DerefMut};

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::Page;
use futures::StreamExt as _;

use crate::error::{UtamError, UtamResult};

/// CDP adapter wrapping a `chromiumoxide::Browser`.
///
/// Created via [`CdpDriver::launch`] or [`CdpDriver::launch_headless`].
/// The active [`Page`] is accessible via [`CdpDriver::page`].
/// Derefs to the inner `Browser` so all chromiumoxide APIs are available.
pub struct CdpDriver {
    inner: Browser,
    /// The active page (first/only tab opened on launch).
    active_page: Page,
    /// Background task handle — must be kept alive for the browser to work.
    _handler: tokio::task::JoinHandle<()>,
}

impl CdpDriver {
    /// Launch a new Chrome instance connected via CDP.
    ///
    /// # Arguments
    ///
    /// * `headless` - Run the browser without a visible window
    ///
    /// # Errors
    ///
    /// Returns [`UtamError::Cdp`] if the browser fails to launch or the
    /// initial page cannot be opened.
    pub async fn launch(headless: bool) -> UtamResult<Self> {
        let mut builder = BrowserConfig::builder();
        if !headless {
            builder = builder.with_head();
        }
        let config = builder
            .build()
            .map_err(|e| UtamError::Cdp(format!("failed to build CDP browser config: {e}")))?;

        let (browser, handler) = Browser::launch(config)
            .await
            .map_err(|e| UtamError::Cdp(format!("failed to launch Chrome via CDP: {e}")))?;

        // The handler drives the CDP connection; spawn it as a background task.
        let join_handle = tokio::spawn(async move {
            let mut handler = handler;
            loop {
                if handler.next().await.is_none() {
                    break;
                }
            }
        });

        let page = browser
            .new_page("about:blank")
            .await
            .map_err(|e| UtamError::Cdp(format!("failed to open initial page: {e}")))?;

        Ok(Self { inner: browser, active_page: page, _handler: join_handle })
    }

    /// Launch a headless Chrome instance — shorthand for `launch(true)`.
    ///
    /// # Errors
    ///
    /// Returns [`UtamError::Cdp`] if the browser fails to launch.
    pub async fn launch_headless() -> UtamResult<Self> {
        Self::launch(true).await
    }

    /// Navigate the active page to a URL.
    ///
    /// # Errors
    ///
    /// Returns [`UtamError::Cdp`] if navigation fails.
    pub async fn goto(&self, url: &str) -> UtamResult<()> {
        self.active_page
            .goto(url)
            .await
            .map_err(|e| UtamError::Cdp(format!("CDP navigation to '{url}' failed: {e}")))?;
        Ok(())
    }

    /// Return a reference to the active [`Page`].
    pub fn page(&self) -> &Page {
        &self.active_page
    }

    /// Evaluate a JavaScript expression on the active page and return the
    /// result as a JSON value.
    ///
    /// # Errors
    ///
    /// Returns [`UtamError::Cdp`] if evaluation fails.
    pub async fn evaluate_js(&self, expression: &str) -> UtamResult<serde_json::Value> {
        let result = self
            .active_page
            .evaluate(expression)
            .await
            .map_err(|e| UtamError::Cdp(format!("CDP JS evaluation failed: {e}")))?;

        result
            .into_value::<serde_json::Value>()
            .map_err(|e| UtamError::Cdp(format!("CDP JS result deserialization failed: {e}")))
    }

    /// Capture all console messages that have been emitted since the page
    /// loaded.  Returns a `Vec` of `(level, text)` pairs.
    ///
    /// Requires the page to have registered a `window.__utamConsoleLogs`
    /// accumulator (see [`CdpDriver::install_console_interceptor`]).
    ///
    /// # Errors
    ///
    /// Returns [`UtamError::Cdp`] if the script execution fails.
    pub async fn console_logs(&self) -> UtamResult<Vec<(String, String)>> {
        let result = self
            .evaluate_js(
                "JSON.stringify(
                    (window.__utamConsoleLogs || []).map(e => [e.level, e.text])
                )",
            )
            .await?;

        let logs: Vec<(String, String)> =
            serde_json::from_value(result).unwrap_or_default();
        Ok(logs)
    }

    /// Install a console log interceptor on the active page.
    ///
    /// After calling this method, any `console.log()` calls on the page
    /// are recorded in `window.__utamConsoleLogs` and retrievable via
    /// [`CdpDriver::console_logs`].
    ///
    /// # Errors
    ///
    /// Returns [`UtamError::Cdp`] if the script injection fails.
    pub async fn install_console_interceptor(&self) -> UtamResult<()> {
        self.evaluate_js(
            "window.__utamConsoleLogs = [];
             const _orig = console.log.bind(console);
             console.log = function(...args) {
                 window.__utamConsoleLogs.push({ level: 'log', text: args.join(' ') });
                 _orig(...args);
             };",
        )
        .await?;
        Ok(())
    }

    /// Close the browser and terminate the CDP connection.
    ///
    /// # Errors
    ///
    /// Returns [`UtamError::Cdp`] if the close command fails.
    pub async fn quit(mut self) -> UtamResult<()> {
        self.inner
            .close()
            .await
            .map_err(|e| UtamError::Cdp(format!("CDP browser close failed: {e}")))?;
        Ok(())
    }
}

impl Deref for CdpDriver {
    type Target = Browser;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for CdpDriver {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
