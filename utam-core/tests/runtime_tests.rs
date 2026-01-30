//! Integration tests for UTAM core runtime
//!
//! Tests runtime traits and element wrappers.

mod common;

use utam_core::prelude::*;

#[test]
fn test_error_types() {
    // Test that error types can be constructed
    let _error = UtamError::ElementNotFound {
        name: "testButton".to_string(),
        selector: ".test".to_string(),
    };
}

#[test]
fn test_prelude_exports() {
    // Test that all expected types are exported from prelude
    // This ensures the public API is stable
    let _result: UtamResult<()> = Ok(());
}

#[test]
fn test_container_can_be_constructed() {
    // Test that Container type is exported and can be type-checked
    // This is a compile-time test ensuring the API is stable
    
    // Mock page object for testing
    struct MockPageObject {
        _root: String,
    }
    
    impl PageObject for MockPageObject {
        fn root(&self) -> &WebElement {
            panic!("not implemented in compile test")
        }
    }
    
    // This verifies Container<T> is exported and properly typed
    let _container_type_check: Option<Container<MockPageObject>> = None;
}

#[test]
fn test_page_object_trait_bounds() {
    // Test that PageObject trait is exported with correct bounds
    fn accepts_page_object<T: PageObject>(_obj: T) {}
    
    struct TestPageObject;
    
    impl PageObject for TestPageObject {
        fn root(&self) -> &WebElement {
            panic!("not implemented in test")
        }
    }
    
    // This should compile, verifying trait is correctly defined
    let obj = TestPageObject;
    accepts_page_object(obj);
}

// Integration tests with actual WebDriver are in separate test modules
// that can be run with specific feature flags or ignored by default
