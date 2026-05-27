//! Test harness for UTAM browser tests
//!
//! Provides [`TestHarness`] for managing WebDriver sessions with automatic
//! screenshot capture on failure, retry logic for flaky tests, and support
//! for parallel test execution via separate driver instances.
//!
//! # Example
//!
//! ```rust,ignore
//! use utam_test::prelude::*;
//!
//! #[tokio::test]
//! async fn test_login() -> UtamResult<()> {
//!     let harness = TestHarness::new(Browser::Chrome).await?;
//!     harness.navigate("https://login.salesforce.com").await?;
//!
//!     harness.run_with_screenshots("login_test", || async {
//!         let login = LoginPage::load(harness.driver()).await?;
//!         login.login("user", "pass").await?;
//!         Ok(())
//!     }).await
//! }
//! ```

use std::future::Future;
use std::path::PathBuf;
use std::time::Duration;

use thirtyfour::prelude::*;
use tokio::fs;

use utam_core::error::{UtamError, UtamResult};
use utam_core::wait::{wait_for, WaitConfig};

/// Supported browser types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Browser {
    Chrome,
    Firefox,
}

impl Browser {
    fn capabilities(&self) -> Capabilities {
        match self {
            Browser::Chrome => {
                let mut caps = DesiredCapabilities::chrome();
                let _ = caps.set_headless();
                let _ = caps.set_no_sandbox();
                let _ = caps.set_disable_gpu();
                caps.into()
            }
            Browser::Firefox => {
                let mut caps = DesiredCapabilities::firefox();
                let _ = caps.set_headless();
                caps.into()
            }
        }
    }
}

/// Configuration for the test harness
#[derive(Debug, Clone)]
pub struct HarnessConfig {
    /// WebDriver server URL (default: http://localhost:4444)
    pub webdriver_url: String,
    /// Directory to store screenshots (default: ./screenshots)
    pub screenshots_dir: PathBuf,
    /// Default timeout for wait operations
    pub default_timeout: Duration,
    /// Number of retry attempts for flaky tests
    pub retry_attempts: u32,
    /// Delay between retry attempts
    pub retry_delay: Duration,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            webdriver_url: "http://localhost:4444".to_string(),
            screenshots_dir: PathBuf::from("./screenshots"),
            default_timeout: Duration::from_secs(10),
            retry_attempts: 0,
            retry_delay: Duration::from_secs(1),
        }
    }
}

/// Test harness managing a WebDriver session
///
/// Each `TestHarness` owns its own WebDriver instance, so tests
/// running in parallel get independent browser sessions.
pub struct TestHarness {
    driver: WebDriver,
    config: HarnessConfig,
}

impl TestHarness {
    /// Create a new test harness with default configuration
    pub async fn new(browser: Browser) -> UtamResult<Self> {
        Self::with_config(browser, HarnessConfig::default()).await
    }

    /// Create a new test harness with custom configuration
    pub async fn with_config(browser: Browser, config: HarnessConfig) -> UtamResult<Self> {
        let caps = browser.capabilities();
        let driver =
            WebDriver::new(&config.webdriver_url, caps).await.map_err(UtamError::WebDriver)?;

        Ok(Self { driver, config })
    }

    /// Get a reference to the WebDriver
    pub fn driver(&self) -> &WebDriver {
        &self.driver
    }

    /// Navigate to a URL
    pub async fn navigate(&self, url: &str) -> UtamResult<()> {
        self.driver.goto(url).await?;
        Ok(())
    }

    /// Get the current page title
    pub async fn title(&self) -> UtamResult<String> {
        Ok(self.driver.title().await?)
    }

    /// Get the current URL
    pub async fn current_url(&self) -> UtamResult<String> {
        Ok(self.driver.current_url().await?.to_string())
    }

    /// Take a screenshot and save it to the screenshots directory
    pub async fn screenshot(&self, name: &str) -> UtamResult<PathBuf> {
        fs::create_dir_all(&self.config.screenshots_dir)
            .await
            .map_err(|e| UtamError::Timeout { condition: format!("create screenshot dir: {e}") })?;

        let path = self.config.screenshots_dir.join(format!("{name}.png"));
        let png = self.driver.screenshot_as_png().await?;
        fs::write(&path, &png)
            .await
            .map_err(|e| UtamError::Timeout { condition: format!("write screenshot: {e}") })?;
        Ok(path)
    }

    /// Run a test body, capturing a screenshot on failure
    ///
    /// If the test body returns an error, a screenshot is saved with the
    /// given `test_name` before the error is propagated.
    pub async fn run_with_screenshots<F, Fut>(
        &self,
        test_name: &str,
        test_body: F,
    ) -> UtamResult<()>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = UtamResult<()>>,
    {
        match test_body().await {
            Ok(()) => Ok(()),
            Err(e) => {
                let _ = self.screenshot(&format!("FAIL_{test_name}")).await;
                Err(e)
            }
        }
    }

    /// Run a test body with retry logic and screenshot-on-failure
    ///
    /// Retries the test body up to `retry_attempts` times (from config).
    /// On each failure, a screenshot is taken. Only the final error
    /// is returned if all attempts fail.
    pub async fn run_with_retries<F, Fut>(&self, test_name: &str, test_body: F) -> UtamResult<()>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = UtamResult<()>>,
    {
        let max_attempts = self.config.retry_attempts + 1;
        let mut last_error = None;

        for attempt in 1..=max_attempts {
            match test_body().await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    let _ = self.screenshot(&format!("FAIL_{test_name}_attempt{attempt}")).await;
                    if attempt < max_attempts {
                        tokio::time::sleep(self.config.retry_delay).await;
                    }
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap())
    }

    /// Wait for the page to be fully loaded (document.readyState === "complete")
    pub async fn wait_for_page_load(&self) -> UtamResult<()> {
        let driver = self.driver.clone();
        wait_for(
            || async {
                let result = driver
                    .execute("return document.readyState", vec![])
                    .await
                    .map_err(UtamError::WebDriver)?;
                if result.json().as_str() == Some("complete") {
                    Ok(Some(()))
                } else {
                    Ok(None)
                }
            },
            &WaitConfig { timeout: self.config.default_timeout, ..Default::default() },
            "page to finish loading",
        )
        .await
    }

    /// Wait for a URL to contain the given substring
    pub async fn wait_for_url_contains(&self, substring: &str) -> UtamResult<()> {
        let driver = self.driver.clone();
        let target = substring.to_string();
        wait_for(
            || async {
                let url = driver.current_url().await.map_err(UtamError::WebDriver)?;
                if url.as_str().contains(&target) {
                    Ok(Some(()))
                } else {
                    Ok(None)
                }
            },
            &WaitConfig { timeout: self.config.default_timeout, ..Default::default() },
            &format!("URL to contain '{substring}'"),
        )
        .await
    }

    /// Quit the browser session
    ///
    /// Clones the driver handle internally so that the `Drop` guard
    /// does not attempt a redundant shutdown.
    pub async fn quit(self) -> UtamResult<()> {
        self.driver.clone().quit().await?;
        Ok(())
    }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        // Best-effort cleanup: spawn a detached task to quit the driver.
        // This prevents leaking browser processes when tests panic.
        let handle = self.driver.handle.clone();
        tokio::spawn(async move {
            let driver = WebDriver { handle };
            let _ = driver.quit().await;
        });
    }
}

/// Convenience macro for creating UTAM tests with automatic harness setup
///
/// # Example
///
/// ```rust,ignore
/// utam_test!(test_login, Browser::Chrome, |harness| async move {
///     harness.navigate("https://example.com").await?;
///     Ok(())
/// });
/// ```
#[macro_export]
macro_rules! utam_test {
    ($name:ident, $browser:expr, $body:expr) => {
        #[tokio::test]
        async fn $name() -> utam_core::error::UtamResult<()> {
            let harness = $crate::TestHarness::new($browser).await?;
            let test_fn = $body;
            let result =
                harness.run_with_screenshots(stringify!($name), || test_fn(&harness)).await;
            harness.quit().await?;
            result
        }
    };

    ($name:ident, $config:expr, $body:expr) => {
        #[tokio::test]
        async fn $name() -> utam_core::error::UtamResult<()> {
            let config: $crate::HarnessConfig = $config;
            let browser = $crate::Browser::Chrome;
            let harness = $crate::TestHarness::with_config(browser, config).await?;
            let test_fn = $body;
            let result =
                harness.run_with_screenshots(stringify!($name), || test_fn(&harness)).await;
            harness.quit().await?;
            result
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_capabilities_chrome() {
        let caps = Browser::Chrome.capabilities();
        // Chrome capabilities should be constructable
        assert!(!format!("{:?}", caps).is_empty());
    }

    #[test]
    fn test_browser_capabilities_firefox() {
        let caps = Browser::Firefox.capabilities();
        assert!(!format!("{:?}", caps).is_empty());
    }

    #[test]
    fn test_harness_config_defaults() {
        let config = HarnessConfig::default();
        assert_eq!(config.webdriver_url, "http://localhost:4444");
        assert_eq!(config.screenshots_dir, PathBuf::from("./screenshots"));
        assert_eq!(config.default_timeout, Duration::from_secs(10));
        assert_eq!(config.retry_attempts, 0);
        assert_eq!(config.retry_delay, Duration::from_secs(1));
    }

    #[test]
    fn test_harness_config_custom() {
        let config = HarnessConfig {
            webdriver_url: "http://selenium:4444".to_string(),
            screenshots_dir: PathBuf::from("/tmp/shots"),
            default_timeout: Duration::from_secs(30),
            retry_attempts: 3,
            retry_delay: Duration::from_millis(500),
        };
        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.screenshots_dir, PathBuf::from("/tmp/shots"));
    }

    #[test]
    fn test_browser_eq() {
        assert_eq!(Browser::Chrome, Browser::Chrome);
        assert_ne!(Browser::Chrome, Browser::Firefox);
    }
}
