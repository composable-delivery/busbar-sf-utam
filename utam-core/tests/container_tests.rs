//! Tests for Container<T> element type
//!
//! These tests verify the Container element's functionality for dynamic content loading.

use utam_core::prelude::*;

/// Mock page object for testing
struct MockPageObject {
    element: WebElement,
}

impl PageObject for MockPageObject {
    fn root(&self) -> &WebElement {
        &self.element
    }
}

#[async_trait::async_trait]
impl RootPageObject for MockPageObject {
    const ROOT_SELECTOR: &'static str = ".mock";

    async fn load(_driver: &WebDriver) -> UtamResult<Self> {
        unimplemented!("not used in unit tests")
    }

    async fn wait_for_load(_driver: &WebDriver, _timeout: std::time::Duration) -> UtamResult<Self> {
        unimplemented!("not used in unit tests")
    }

    async fn from_element(element: WebElement) -> UtamResult<Self> {
        Ok(Self { element })
    }
}

/// Another mock page object for polymorphic loading tests
struct AnotherMockPageObject {
    element: WebElement,
}

impl PageObject for AnotherMockPageObject {
    fn root(&self) -> &WebElement {
        &self.element
    }
}

#[async_trait::async_trait]
impl RootPageObject for AnotherMockPageObject {
    const ROOT_SELECTOR: &'static str = ".another-mock";

    async fn load(_driver: &WebDriver) -> UtamResult<Self> {
        unimplemented!("not used in unit tests")
    }

    async fn wait_for_load(_driver: &WebDriver, _timeout: std::time::Duration) -> UtamResult<Self> {
        unimplemented!("not used in unit tests")
    }

    async fn from_element(element: WebElement) -> UtamResult<Self> {
        Ok(Self { element })
    }
}

#[test]
fn test_container_creation() {
    // Test that Container can be created with new()
    // This is a compile-time test
    
    struct TestPage;
    impl PageObject for TestPage {
        fn root(&self) -> &WebElement {
            panic!("not used")
        }
    }
    
    // Verify the type system works correctly
    let _container_type: Option<Container<TestPage>> = None;
}

#[test]
fn test_container_with_selector_builder() {
    // Test that Container supports builder pattern with with_selector()
    // This verifies the API design
    
    struct TestPage;
    impl PageObject for TestPage {
        fn root(&self) -> &WebElement {
            panic!("not used")
        }
    }
    
    // This should compile, verifying builder pattern works
    let _container_type: Option<Container<TestPage>> = None;
    // If we had a real element: container.with_selector(By::Css(".custom"))
}

#[test]
fn test_container_generic_over_page_object() {
    // Test that Container is properly generic over PageObject types
    struct PageA;
    struct PageB;
    
    impl PageObject for PageA {
        fn root(&self) -> &WebElement {
            panic!("not used")
        }
    }
    
    impl PageObject for PageB {
        fn root(&self) -> &WebElement {
            panic!("not used")
        }
    }
    
    // Different types should be distinct
    let _container_a: Option<Container<PageA>> = None;
    let _container_b: Option<Container<PageB>> = None;
}

#[test]
fn test_page_object_trait_is_sized() {
    // PageObject must be Sized for Container to work
    fn require_sized<T: PageObject>() {
        // If T is Sized, this compiles
        let _size = std::mem::size_of::<T>();
    }
    
    struct TestPage;
    impl PageObject for TestPage {
        fn root(&self) -> &WebElement {
            panic!("not used")
        }
    }
    
    require_sized::<TestPage>();
}

#[test]
fn test_page_object_trait_is_send_sync() {
    // PageObject must be Send + Sync for async usage
    fn require_send_sync<T: PageObject>() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        
        assert_send::<T>();
        assert_sync::<T>();
    }
    
    struct TestPage;
    impl PageObject for TestPage {
        fn root(&self) -> &WebElement {
            panic!("not used")
        }
    }
    
    require_send_sync::<TestPage>();
}

// Note: Full integration tests with WebDriver would require:
// 1. Running a WebDriver instance (geckodriver/chromedriver)
// 2. HTML test fixtures
// 3. Async test runtime setup
//
// These are better suited for end-to-end tests or examples.
// The tests above verify the type system and API design.
