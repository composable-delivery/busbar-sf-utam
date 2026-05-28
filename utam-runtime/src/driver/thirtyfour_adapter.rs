//! WebDriver/Selenium adapter via the `thirtyfour` crate.

use std::time::Duration;

use async_trait::async_trait;
use thirtyfour::prelude::*;

use super::{ElementHandle, Selector, ShadowRootHandle, UtamDriver};
use crate::error::RuntimeResult;

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
        Selector::IosClassChain(s) => By::Tag(s),
        Selector::AndroidUiAutomator(s) => By::Tag(s),
    }
}

fn to_rt(e: WebDriverError) -> crate::error::RuntimeError {
    crate::error::RuntimeError::Utam(utam_core::error::UtamError::WebDriver(e))
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
// ElementHandle
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ThirtyfourElement(WebElement);

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
// ShadowRootHandle
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
