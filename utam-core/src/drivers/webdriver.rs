//! WebDriver adapter for UTAM using `thirtyfour`.
//!
//! `ThirtyfourDriver` wraps a `thirtyfour::WebDriver` and exposes helper
//! methods used by the test utilities.  The underlying `WebDriver` is
//! accessible via `Deref` so all thirtyfour APIs are available directly.

use std::ops::{Deref, DerefMut};
use std::time::Duration;

use thirtyfour::{ChromiumLikeCapabilities, DesiredCapabilities, WebDriver};

use crate::error::{UtamError, UtamResult};

/// WebDriver adapter wrapping a `thirtyfour::WebDriver`.
///
/// Created via [`ThirtyfourDriver::connect`] or
/// [`ThirtyfourDriver::connect_headless`].  Derefs to the inner
/// `WebDriver` so all thirtyfour APIs are usable directly.
pub struct ThirtyfourDriver {
    inner: WebDriver,
}

impl ThirtyfourDriver {
    /// Connect to a running ChromeDriver instance.
    ///
    /// # Arguments
    ///
    /// * `chromedriver_url` - URL of the ChromeDriver server (e.g. `"http://localhost:9515"`)
    /// * `headless` - Run the browser without a visible window
    /// * `implicit_wait_ms` - Default element-lookup timeout in milliseconds
    ///
    /// # Errors
    ///
    /// Returns [`UtamError::WebDriver`] if the connection or capability
    /// negotiation fails.
    pub async fn connect(
        chromedriver_url: &str,
        headless: bool,
        implicit_wait_ms: u64,
    ) -> UtamResult<Self> {
        let mut caps = DesiredCapabilities::chrome();
        if headless {
            caps.set_headless().map_err(UtamError::WebDriver)?;
        }

        let driver = WebDriver::new(chromedriver_url, caps).await.map_err(UtamError::WebDriver)?;

        driver
            .set_implicit_wait_timeout(Duration::from_millis(implicit_wait_ms))
            .await?;

        Ok(Self { inner: driver })
    }

    /// Connect using sensible defaults: headless Chrome on `localhost:9515`
    /// with a 5 s implicit wait.
    ///
    /// # Errors
    ///
    /// Returns [`UtamError::WebDriver`] if the connection fails.
    pub async fn connect_default() -> UtamResult<Self> {
        Self::connect("http://localhost:9515", true, 5_000).await
    }

    /// Consume the adapter and quit the browser session.
    ///
    /// # Errors
    ///
    /// Returns [`UtamError::WebDriver`] if the quit command fails.
    pub async fn quit(self) -> UtamResult<()> {
        self.inner.quit().await?;
        Ok(())
    }
}

impl Deref for ThirtyfourDriver {
    type Target = WebDriver;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for ThirtyfourDriver {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
