//! Element types for UTAM page objects
//!
//! This module provides element wrapper types used in page objects:
//! - `Container<T>` - Generic wrapper for slot/dynamic content

use crate::error::{UtamError, UtamResult};
use crate::traits::{PageObject, RootPageObject};
use std::marker::PhantomData;
use thirtyfour::{By, WebElement};

/// Container element for slots and dynamic content injection
///
/// `Container<T>` is a generic wrapper for elements that contain dynamic content
/// loaded as page objects. This is useful for:
/// - Shadow DOM slots
/// - Dynamic content injection
/// - Polymorphic components
///
/// # Type Parameters
///
/// * `T` - The page object type contained within this container
///
/// # Examples
///
/// ```rust,ignore
/// use utam_core::prelude::*;
/// use utam_core::elements::Container;
///
/// // Load default first child
/// let container: Container<LoginForm> = Container::new(root_element);
/// let form = container.load().await?;
///
/// // Load with custom selector
/// let container = Container::new(root_element)
///     .with_selector(By::Css(".dynamic-content"));
/// let form = container.load().await?;
///
/// // Polymorphic loading
/// let admin_form = container.load_as::<AdminLoginForm>().await?;
/// ```
pub struct Container<T: PageObject> {
    /// The root WebElement containing the dynamic content
    root: WebElement,
    
    /// Optional custom selector to find the contained element
    /// If None, defaults to `:scope > *:first-child`
    selector: Option<By>,
    
    /// PhantomData to hold the type parameter
    _phantom: PhantomData<T>,
}

impl<T: PageObject> Container<T> {
    /// Create a new Container with the default selector
    ///
    /// The default selector is `:scope > *:first-child`, which selects
    /// the first direct child of the container element.
    ///
    /// # Arguments
    ///
    /// * `root` - The WebElement that contains the dynamic content
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let container = Container::<MyComponent>::new(element);
    /// ```
    pub fn new(root: WebElement) -> Self {
        Self {
            root,
            selector: None,
            _phantom: PhantomData,
        }
    }

    /// Set a custom selector for finding the contained element
    ///
    /// This allows overriding the default `:scope > *:first-child` selector
    /// with a custom selector.
    ///
    /// # Arguments
    ///
    /// * `selector` - The selector to use for finding the contained element
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let container = Container::<MyComponent>::new(element)
    ///     .with_selector(By::Css(".specific-class"));
    /// ```
    pub fn with_selector(mut self, selector: By) -> Self {
        self.selector = Some(selector);
        self
    }

    /// Load the contained page object
    ///
    /// Finds the element using the configured selector (or default) and
    /// constructs a page object of type `T` from it.
    ///
    /// # Returns
    ///
    /// The loaded page object instance
    ///
    /// # Errors
    ///
    /// * `UtamError::WebDriver` - If the element cannot be found
    /// * Other errors from `T::from_element()`
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let form = container.load().await?;
    /// form.fill_username("user").await?;
    /// ```
    pub async fn load(&self) -> UtamResult<T>
    where
        T: RootPageObject,
    {
        let selector = self
            .selector
            .clone()
            .unwrap_or_else(|| By::Css(":scope > *:first-child".to_string()));

        let element = self.root.find(selector).await.map_err(|_e| {
            UtamError::ElementNotFound {
                name: "container content".to_string(),
                selector: self
                    .selector
                    .as_ref()
                    .map(|s| format!("{:?}", s))
                    .unwrap_or_else(|| ":scope > *:first-child".to_string()),
            }
        })?;

        T::from_element(element).await
    }

    /// Load the contained element as a different page object type
    ///
    /// This allows polymorphic loading, where the same container can load
    /// different types of page objects based on runtime conditions.
    ///
    /// # Type Parameters
    ///
    /// * `U` - The page object type to load
    ///
    /// # Returns
    ///
    /// The loaded page object instance of type `U`
    ///
    /// # Errors
    ///
    /// * `UtamError::WebDriver` - If the element cannot be found
    /// * Other errors from `U::from_element()`
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Container defined for base type
    /// let container: Container<BaseForm> = Container::new(element);
    ///
    /// // Load as specialized type
    /// let admin_form = container.load_as::<AdminForm>().await?;
    /// ```
    pub async fn load_as<U: RootPageObject>(&self) -> UtamResult<U> {
        let selector = self
            .selector
            .clone()
            .unwrap_or_else(|| By::Css(":scope > *:first-child".to_string()));

        let element = self.root.find(selector).await.map_err(|_e| {
            UtamError::ElementNotFound {
                name: "container content".to_string(),
                selector: self
                    .selector
                    .as_ref()
                    .map(|s| format!("{:?}", s))
                    .unwrap_or_else(|| ":scope > *:first-child".to_string()),
            }
        })?;

        U::from_element(element).await
    }
}
