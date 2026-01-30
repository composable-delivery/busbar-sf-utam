//! Element wrappers for UTAM runtime
//!
//! This module provides wrappers around thirtyfour::WebElement with
//! convenient methods for common element operations.

use thirtyfour::prelude::*;
use thirtyfour::ElementRect;

use crate::error::{UtamError, UtamResult};

/// Rectangle representing an element's position and size
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElementRectangle {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl ElementRectangle {
    /// Create a new ElementRectangle
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }
}

impl From<ElementRect> for ElementRectangle {
    fn from(rect: ElementRect) -> Self {
        Self::new(rect.x, rect.y, rect.width, rect.height)
    }
}

/// Base element wrapper providing common actions
///
/// This struct wraps a thirtyfour::WebElement and provides
/// UTAM-specific methods with proper error handling.
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

    /// Check if the element is enabled
    pub async fn is_enabled(&self) -> UtamResult<bool> {
        Ok(self.inner.is_enabled().await?)
    }

    /// Check if the element has focus
    pub async fn is_focused(&self) -> UtamResult<bool> {
        // Check if the element is the active element
        let script = "return document.activeElement === arguments[0];";
        let result = self.inner.handle.execute(script, vec![self.inner.to_json()?]).await?;
        Ok(result.json().as_bool().unwrap_or(false))
    }

    /// Check if the element is present in the DOM
    pub async fn is_present(&self) -> UtamResult<bool> {
        // Try to get tag name - if it succeeds, element is present
        match self.inner.tag_name().await {
            Ok(_) => Ok(true),
            Err(e) => {
                // Check if it's a stale element error
                if e.to_string().contains("stale element") {
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

    /// Check if the element contains a child element matching the selector
    ///
    /// # Arguments
    ///
    /// * `selector` - CSS selector to search for
    /// * `expand_shadow` - Whether to expand shadow DOM when searching
    pub async fn contains_element(&self, selector: &str, expand_shadow: bool) -> UtamResult<bool> {
        if expand_shadow {
            // Try to get shadow root and search within it
            match self.inner.get_shadow_root().await {
                Ok(shadow_root) => {
                    match shadow_root.find(By::Css(selector)).await {
                        Ok(_) => Ok(true),
                        Err(e) => {
                            // If element not found, return false
                            if e.to_string().contains("no such element") {
                                Ok(false)
                            } else {
                                Err(UtamError::WebDriver(e))
                            }
                        }
                    }
                }
                Err(e) => {
                    // If no shadow root, fall back to regular search
                    if e.to_string().contains("no such shadow root") {
                        // Regular search within the element (non-recursive)
                        match self.inner.find(By::Css(selector)).await {
                            Ok(_) => Ok(true),
                            Err(e) => {
                                // If element not found, return false
                                if e.to_string().contains("no such element") {
                                    Ok(false)
                                } else {
                                    Err(UtamError::WebDriver(e))
                                }
                            }
                        }
                    } else {
                        Err(UtamError::WebDriver(e))
                    }
                }
            }
        } else {
            // Regular search within the element
            match self.inner.find(By::Css(selector)).await {
                Ok(_) => Ok(true),
                Err(e) => {
                    // If element not found, return false
                    if e.to_string().contains("no such element") {
                        Ok(false)
                    } else {
                        Err(UtamError::WebDriver(e))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_rectangle_creation() {
        let rect = ElementRectangle::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(rect.x, 10.0);
        assert_eq!(rect.y, 20.0);
        assert_eq!(rect.width, 100.0);
        assert_eq!(rect.height, 50.0);
    }

    #[test]
    fn test_element_rectangle_from_rect() {
        let tf_rect = ElementRect { x: 5.0, y: 10.0, width: 200.0, height: 100.0 };
        let rect = ElementRectangle::from(tf_rect);
        assert_eq!(rect.x, 5.0);
        assert_eq!(rect.y, 10.0);
        assert_eq!(rect.width, 200.0);
        assert_eq!(rect.height, 100.0);
    }
}
