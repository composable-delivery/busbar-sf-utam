//! Assertion helpers for UTAM page object testing
//!
//! Provides [`PageObjectAssertions`] for common element assertions and
//! [`ElementAssertion`] for a fluent builder pattern with configurable timeouts.
//!
//! # Example
//!
//! ```rust,ignore
//! use utam_test::prelude::*;
//!
//! // Direct assertions on BaseElement
//! let element = page.get_submit_button().await?;
//! element.assert_visible().await?;
//! element.assert_text_equals("Submit").await?;
//!
//! // Fluent builder with custom timeout
//! assert_element(&element)
//!     .with_timeout(Duration::from_secs(5))
//!     .is_visible()
//!     .await?;
//! ```

use std::time::Duration;

use async_trait::async_trait;
use thirtyfour::WebElement;

use utam_core::elements::BaseElement;
use utam_core::error::{UtamError, UtamResult};
use utam_core::wait::{wait_for, WaitConfig};

/// Trait providing assertion methods for elements
///
/// Implemented for [`BaseElement`] to allow convenient inline assertions
/// during test execution.
#[async_trait]
pub trait PageObjectAssertions {
    /// Assert that the element is visible on the page
    async fn assert_visible(&self) -> UtamResult<()>;

    /// Assert that the element is hidden (not visible)
    async fn assert_hidden(&self) -> UtamResult<()>;

    /// Assert that the element is present in the DOM
    async fn assert_present(&self) -> UtamResult<()>;

    /// Assert that the element is enabled
    async fn assert_enabled(&self) -> UtamResult<()>;

    /// Assert that the element is disabled
    async fn assert_disabled(&self) -> UtamResult<()>;

    /// Assert the element's text content equals the expected value
    async fn assert_text_equals(&self, expected: &str) -> UtamResult<()>;

    /// Assert the element's text content contains the expected substring
    async fn assert_text_contains(&self, expected: &str) -> UtamResult<()>;

    /// Assert an attribute equals the expected value
    async fn assert_attribute_equals(&self, attr: &str, expected: &str) -> UtamResult<()>;

    /// Assert an attribute contains the expected substring
    async fn assert_attribute_contains(&self, attr: &str, expected: &str) -> UtamResult<()>;

    /// Assert the element has the given CSS class
    async fn assert_has_class(&self, class_name: &str) -> UtamResult<()>;
}

#[async_trait]
impl PageObjectAssertions for BaseElement {
    async fn assert_visible(&self) -> UtamResult<()> {
        if !self.is_visible().await? {
            return Err(UtamError::AssertionFailed {
                expected: "element to be visible".to_string(),
                actual: "element is hidden".to_string(),
            });
        }
        Ok(())
    }

    async fn assert_hidden(&self) -> UtamResult<()> {
        if self.is_visible().await? {
            return Err(UtamError::AssertionFailed {
                expected: "element to be hidden".to_string(),
                actual: "element is visible".to_string(),
            });
        }
        Ok(())
    }

    async fn assert_present(&self) -> UtamResult<()> {
        if !self.is_present().await? {
            return Err(UtamError::AssertionFailed {
                expected: "element to be present in DOM".to_string(),
                actual: "element is absent".to_string(),
            });
        }
        Ok(())
    }

    async fn assert_enabled(&self) -> UtamResult<()> {
        if !self.is_enabled().await? {
            return Err(UtamError::AssertionFailed {
                expected: "element to be enabled".to_string(),
                actual: "element is disabled".to_string(),
            });
        }
        Ok(())
    }

    async fn assert_disabled(&self) -> UtamResult<()> {
        if self.is_enabled().await? {
            return Err(UtamError::AssertionFailed {
                expected: "element to be disabled".to_string(),
                actual: "element is enabled".to_string(),
            });
        }
        Ok(())
    }

    async fn assert_text_equals(&self, expected: &str) -> UtamResult<()> {
        let actual = self.get_text().await?;
        if actual != expected {
            return Err(UtamError::AssertionFailed {
                expected: format!("text to equal \"{expected}\""),
                actual: format!("got \"{actual}\""),
            });
        }
        Ok(())
    }

    async fn assert_text_contains(&self, expected: &str) -> UtamResult<()> {
        let actual = self.get_text().await?;
        if !actual.contains(expected) {
            return Err(UtamError::AssertionFailed {
                expected: format!("text to contain \"{expected}\""),
                actual: format!("got \"{actual}\""),
            });
        }
        Ok(())
    }

    async fn assert_attribute_equals(&self, attr: &str, expected: &str) -> UtamResult<()> {
        let actual = self.get_attribute(attr).await?.unwrap_or_default();
        if actual != expected {
            return Err(UtamError::AssertionFailed {
                expected: format!("attribute \"{attr}\" to equal \"{expected}\""),
                actual: format!("got \"{actual}\""),
            });
        }
        Ok(())
    }

    async fn assert_attribute_contains(&self, attr: &str, expected: &str) -> UtamResult<()> {
        let actual = self.get_attribute(attr).await?.unwrap_or_default();
        if !actual.contains(expected) {
            return Err(UtamError::AssertionFailed {
                expected: format!("attribute \"{attr}\" to contain \"{expected}\""),
                actual: format!("got \"{actual}\""),
            });
        }
        Ok(())
    }

    async fn assert_has_class(&self, class_name: &str) -> UtamResult<()> {
        let classes = self.get_class_attribute().await?;
        if !classes.split_whitespace().any(|c| c == class_name) {
            return Err(UtamError::AssertionFailed {
                expected: format!("element to have class \"{class_name}\""),
                actual: format!("classes are \"{classes}\""),
            });
        }
        Ok(())
    }
}

/// Assertion helpers for collections of elements
pub struct CollectionAssertions;

impl CollectionAssertions {
    /// Assert the collection has the expected number of items
    pub fn assert_count(elements: &[WebElement], expected: usize) -> UtamResult<()> {
        let actual = elements.len();
        if actual != expected {
            return Err(UtamError::AssertionFailed {
                expected: format!("collection to have {expected} elements"),
                actual: format!("got {actual} elements"),
            });
        }
        Ok(())
    }

    /// Assert the collection is not empty
    pub fn assert_not_empty(elements: &[WebElement]) -> UtamResult<()> {
        if elements.is_empty() {
            return Err(UtamError::AssertionFailed {
                expected: "collection to be non-empty".to_string(),
                actual: "collection is empty".to_string(),
            });
        }
        Ok(())
    }

    /// Assert the collection is empty
    pub fn assert_empty(elements: &[WebElement]) -> UtamResult<()> {
        if !elements.is_empty() {
            return Err(UtamError::AssertionFailed {
                expected: "collection to be empty".to_string(),
                actual: format!("collection has {} elements", elements.len()),
            });
        }
        Ok(())
    }
}

/// Fluent builder for element assertions with configurable timeouts
///
/// Created via [`assert_element()`]. Allows setting a timeout before
/// making async assertions that poll until the condition is met.
///
/// # Example
///
/// ```rust,ignore
/// assert_element(&element)
///     .with_timeout(Duration::from_secs(5))
///     .is_visible()
///     .await?;
/// ```
pub struct ElementAssertion<'a> {
    element: &'a BaseElement,
    config: WaitConfig,
}

/// Create a fluent assertion builder for an element
pub fn assert_element(element: &BaseElement) -> ElementAssertion<'_> {
    ElementAssertion { element, config: WaitConfig::default() }
}

impl<'a> ElementAssertion<'a> {
    /// Set the timeout for this assertion
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Set the polling interval for this assertion
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.config.poll_interval = interval;
        self
    }

    /// Assert the element becomes visible within the timeout
    pub async fn is_visible(self) -> UtamResult<()> {
        let element = self.element.clone();
        wait_for(
            || async {
                match element.is_visible().await {
                    Ok(true) => Ok(Some(())),
                    Ok(false) => Ok(None),
                    Err(_) => Ok(None),
                }
            },
            &self.config,
            "element to become visible",
        )
        .await
    }

    /// Assert the element becomes hidden within the timeout
    pub async fn is_hidden(self) -> UtamResult<()> {
        let element = self.element.clone();
        wait_for(
            || async {
                match element.is_visible().await {
                    Ok(false) => Ok(Some(())),
                    Ok(true) => Ok(None),
                    Err(_) => Ok(None),
                }
            },
            &self.config,
            "element to become hidden",
        )
        .await
    }

    /// Assert the element's text equals the expected value within the timeout
    pub async fn has_text(self, expected: &str) -> UtamResult<()> {
        let element = self.element.clone();
        let expected = expected.to_string();
        wait_for(
            || async {
                match element.get_text().await {
                    Ok(text) if text == expected => Ok(Some(())),
                    _ => Ok(None),
                }
            },
            &self.config,
            &format!("element text to equal \"{expected}\""),
        )
        .await
    }

    /// Assert the element's text contains the expected substring within the timeout
    pub async fn text_contains(self, expected: &str) -> UtamResult<()> {
        let element = self.element.clone();
        let expected = expected.to_string();
        wait_for(
            || async {
                match element.get_text().await {
                    Ok(text) if text.contains(&expected) => Ok(Some(())),
                    _ => Ok(None),
                }
            },
            &self.config,
            &format!("element text to contain \"{expected}\""),
        )
        .await
    }

    /// Assert an attribute equals the expected value within the timeout
    pub async fn has_attribute(self, attr: &str, expected: &str) -> UtamResult<()> {
        let element = self.element.clone();
        let attr = attr.to_string();
        let expected = expected.to_string();
        wait_for(
            || async {
                match element.get_attribute(&attr).await {
                    Ok(Some(val)) if val == expected => Ok(Some(())),
                    _ => Ok(None),
                }
            },
            &self.config,
            &format!("attribute \"{attr}\" to equal \"{expected}\""),
        )
        .await
    }

    /// Assert the element becomes enabled within the timeout
    pub async fn is_enabled(self) -> UtamResult<()> {
        let element = self.element.clone();
        wait_for(
            || async {
                match element.is_enabled().await {
                    Ok(true) => Ok(Some(())),
                    Ok(false) => Ok(None),
                    Err(_) => Ok(None),
                }
            },
            &self.config,
            "element to become enabled",
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_assert_count_pass() {
        let elements: Vec<WebElement> = vec![];
        assert!(CollectionAssertions::assert_count(&elements, 0).is_ok());
    }

    #[test]
    fn test_collection_assert_count_fail() {
        let elements: Vec<WebElement> = vec![];
        let result = CollectionAssertions::assert_count(&elements, 5);
        assert!(result.is_err());
        if let Err(UtamError::AssertionFailed { expected, actual }) = result {
            assert!(expected.contains("5"));
            assert!(actual.contains("0"));
        }
    }

    #[test]
    fn test_collection_assert_empty_pass() {
        let elements: Vec<WebElement> = vec![];
        assert!(CollectionAssertions::assert_empty(&elements).is_ok());
    }

    #[test]
    fn test_collection_assert_not_empty_fail() {
        let elements: Vec<WebElement> = vec![];
        let result = CollectionAssertions::assert_not_empty(&elements);
        assert!(result.is_err());
    }
}
