//! Container element for slots and dynamic content injection
//!
//! `Container<T>` is a generic wrapper for elements that contain dynamic content
//! loaded as page objects, useful for shadow DOM slots, dynamic content injection,
//! and polymorphic components.

use std::marker::PhantomData;

use thirtyfour::{By, WebElement};

use crate::error::{UtamError, UtamResult};
use crate::traits::{PageObject, RootPageObject};

/// Default selector for container content: first direct child
const DEFAULT_CONTAINER_SELECTOR: &str = ":scope > *:first-child";

/// Container element for slots and dynamic content injection
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
/// let container: Container<LoginForm> = Container::new(root_element);
/// let form = container.load().await?;
///
/// // Polymorphic loading
/// let admin_form = container.load_as::<AdminLoginForm>().await?;
/// ```
pub struct Container<T: PageObject> {
    root: WebElement,
    selector: Option<By>,
    _phantom: PhantomData<T>,
}

impl<T: PageObject> Container<T> {
    /// Create a new Container with the default selector
    ///
    /// The default selector is `:scope > *:first-child`.
    pub fn new(root: WebElement) -> Self {
        Self { root, selector: None, _phantom: PhantomData }
    }

    /// Set a custom selector for finding the contained element
    pub fn with_selector(mut self, selector: By) -> Self {
        self.selector = Some(selector);
        self
    }

    async fn find_element(&self) -> UtamResult<WebElement> {
        let selector = self
            .selector
            .as_ref()
            .cloned()
            .unwrap_or_else(|| By::Css(DEFAULT_CONTAINER_SELECTOR.to_string()));

        self.root.find(selector.clone()).await.map_err(|e| UtamError::ElementNotFound {
            name: format!("container content ({})", e),
            selector: self
                .selector
                .as_ref()
                .map(|s| format!("{:?}", s))
                .unwrap_or_else(|| DEFAULT_CONTAINER_SELECTOR.to_string()),
        })
    }

    /// Load the contained page object
    pub async fn load(&self) -> UtamResult<T>
    where
        T: RootPageObject,
    {
        let element = self.find_element().await?;
        T::from_element(element).await
    }

    /// Load the contained element as a different page object type
    ///
    /// This allows polymorphic loading where the same container can load
    /// different types of page objects based on runtime conditions.
    pub async fn load_as<U: RootPageObject>(&self) -> UtamResult<U> {
        let element = self.find_element().await?;
        U::from_element(element).await
    }
}
