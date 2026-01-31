//! BaseElement - core element wrapper with common operations
//!
//! This is the primary element wrapper providing UTAM-specific methods
//! with proper error handling, including DOM queries, attribute access,
//! shadow DOM support, and wait utilities.

use std::time::Duration;

use async_trait::async_trait;
use thirtyfour::prelude::*;

use crate::elements::ElementRectangle;
use crate::error::{UtamError, UtamResult};
use crate::shadow::ShadowRoot;
use crate::traits::Actionable;
use crate::wait::{wait_for, WaitConfig};

/// Base element wrapper providing common actions
///
/// This struct wraps a thirtyfour::WebElement and provides
/// UTAM-specific methods with proper error handling.
#[derive(Debug, Clone)]
pub struct BaseElement {
    inner: WebElement,
}

impl BaseElement {
    /// Create a new BaseElement wrapping a WebElement
    pub fn new(element: WebElement) -> Self {
        Self { inner: element }
    }

    /// Get a reference to the inner WebElement
    pub fn inner(&self) -> &WebElement {
        &self.inner
    }

    // -- Attribute / property queries --

    /// Get the text content of the element
    pub async fn get_text(&self) -> UtamResult<String> {
        Ok(self.inner.text().await?)
    }

    /// Get an attribute value from the element
    ///
    /// Returns None if the attribute doesn't exist
    pub async fn get_attribute(&self, name: &str) -> UtamResult<Option<String>> {
        Ok(self.inner.attr(name).await?)
    }

    /// Get the class attribute value
    pub async fn get_class_attribute(&self) -> UtamResult<String> {
        Ok(self.inner.class_name().await?.unwrap_or_default())
    }

    /// Get a CSS property value
    pub async fn get_css_property_value(&self, name: &str) -> UtamResult<String> {
        Ok(self.inner.css_value(name).await?)
    }

    /// Get the element's rectangle (position and size)
    pub async fn get_rect(&self) -> UtamResult<ElementRectangle> {
        let rect = self.inner.rect().await?;
        Ok(ElementRectangle::from(rect))
    }

    /// Get the title attribute value
    pub async fn get_title(&self) -> UtamResult<String> {
        Ok(self.get_attribute("title").await?.unwrap_or_default())
    }

    /// Get the value attribute (typically for input elements)
    pub async fn get_value(&self) -> UtamResult<String> {
        Ok(self.inner.value().await?.unwrap_or_default())
    }

    // -- State queries --

    /// Check if the element is enabled
    pub async fn is_enabled(&self) -> UtamResult<bool> {
        Ok(self.inner.is_enabled().await?)
    }

    /// Check if the element has focus
    pub async fn is_focused(&self) -> UtamResult<bool> {
        let script = "return document.activeElement === arguments[0];";
        let result = self.inner.handle.execute(script, vec![self.inner.to_json()?]).await?;
        Ok(result.json().as_bool().unwrap_or(false))
    }

    /// Check if the element is present in the DOM
    pub async fn is_present(&self) -> UtamResult<bool> {
        match self.inner.tag_name().await {
            Ok(_) => Ok(true),
            Err(e) => {
                let err_str = e.to_string().to_lowercase();
                if err_str.contains("stale") || err_str.contains("no such element") {
                    Ok(false)
                } else {
                    Err(UtamError::WebDriver(e))
                }
            }
        }
    }

    /// Check if the element is visible
    pub async fn is_visible(&self) -> UtamResult<bool> {
        Ok(self.inner.is_displayed().await?)
    }

    // -- Shadow DOM --

    /// Get the shadow root of this element
    pub async fn get_shadow_root(&self) -> UtamResult<ShadowRoot> {
        let shadow = self
            .inner
            .get_shadow_root()
            .await
            .map_err(|_| UtamError::ShadowRootNotFound { element: "unknown".to_string() })?;
        Ok(ShadowRoot::new(shadow))
    }

    // -- Child element queries --

    /// Check if the element contains a child element matching the selector
    pub async fn contains_element(&self, selector: &str, expand_shadow: bool) -> UtamResult<bool> {
        if expand_shadow {
            self.element_exists_in_shadow(selector).await
        } else {
            self.element_exists(selector).await
        }
    }

    async fn element_exists(&self, selector: &str) -> UtamResult<bool> {
        match self.inner.find(By::Css(selector)).await {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("no such element") {
                    Ok(false)
                } else {
                    Err(UtamError::WebDriver(e))
                }
            }
        }
    }

    async fn element_exists_in_shadow(&self, selector: &str) -> UtamResult<bool> {
        match self.inner.get_shadow_root().await {
            Ok(shadow_root) => match shadow_root.find(By::Css(selector)).await {
                Ok(_) => Ok(true),
                Err(e) => {
                    if e.to_string().contains("no such element") {
                        Ok(false)
                    } else {
                        Err(UtamError::WebDriver(e))
                    }
                }
            },
            Err(e) => {
                if e.to_string().contains("no such shadow root") {
                    self.element_exists(selector).await
                } else {
                    Err(UtamError::WebDriver(e))
                }
            }
        }
    }

    // -- Wait utilities --

    /// Wait for the element to become visible
    pub async fn wait_for_visible(&self, timeout: Duration) -> UtamResult<()> {
        let element = self.clone();
        wait_for(
            || async {
                if element.is_visible().await? {
                    Ok(Some(()))
                } else {
                    Ok(None)
                }
            },
            &WaitConfig { timeout, ..Default::default() },
            "element to become visible",
        )
        .await
    }

    /// Wait for the element to become invisible
    pub async fn wait_for_invisible(&self, timeout: Duration) -> UtamResult<()> {
        let element = self.clone();
        wait_for(
            || async {
                if !element.is_present().await? {
                    return Ok(Some(()));
                }
                if !element.is_visible().await? {
                    Ok(Some(()))
                } else {
                    Ok(None)
                }
            },
            &WaitConfig { timeout, ..Default::default() },
            "element to become invisible",
        )
        .await
    }

    /// Wait for the element to be removed from the DOM
    pub async fn wait_for_absence(&self, timeout: Duration) -> UtamResult<()> {
        let element = self.clone();
        wait_for(
            || async {
                if !element.is_present().await? {
                    Ok(Some(()))
                } else {
                    Ok(None)
                }
            },
            &WaitConfig { timeout, ..Default::default() },
            "element to be removed from DOM",
        )
        .await
    }

    /// Wait for the element to become enabled
    pub async fn wait_for_enabled(&self, timeout: Duration) -> UtamResult<()> {
        let element = self.clone();
        wait_for(
            || async {
                if element.is_enabled().await? {
                    Ok(Some(()))
                } else {
                    Ok(None)
                }
            },
            &WaitConfig { timeout, ..Default::default() },
            "element to become enabled",
        )
        .await
    }
}

#[async_trait]
impl Actionable for BaseElement {
    fn inner(&self) -> &WebElement {
        &self.inner
    }
}
