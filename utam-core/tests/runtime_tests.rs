//! Integration tests for UTAM core runtime
//!
//! Tests runtime traits and element wrappers.

mod common;

use utam_core::prelude::*;
use std::time::Duration;

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

// Test structs for validating trait implementations
#[cfg(test)]
mod page_object_tests {
    use super::*;

    /// Mock page object for testing PageObject trait
    #[allow(dead_code)]
    struct MockPageObject {
        root: WebElement,
    }

    impl PageObject for MockPageObject {
        fn root(&self) -> &WebElement {
            &self.root
        }
    }

    /// Mock root page object for testing RootPageObject trait
    struct MockRootPageObject {
        root: WebElement,
    }

    impl PageObject for MockRootPageObject {
        fn root(&self) -> &WebElement {
            &self.root
        }
    }

    #[async_trait::async_trait]
    impl RootPageObject for MockRootPageObject {
        const ROOT_SELECTOR: &'static str = ".mock-root";

        async fn load(driver: &WebDriver) -> UtamResult<Self> {
            let root = driver.find(By::Css(Self::ROOT_SELECTOR)).await?;
            Self::from_element(root).await
        }

        async fn from_element(element: WebElement) -> UtamResult<Self> {
            Ok(Self { root: element })
        }
    }

    #[test]
    fn test_page_object_trait_compiles() {
        // This test just validates that the trait can be implemented
        // and that the basic interface is usable at compile time
        fn _assert_page_object<T: PageObject>(_: T) {}
    }

    #[test]
    fn test_root_page_object_trait_compiles() {
        // This test validates that RootPageObject trait can be implemented
        // and that the interface is usable at compile time
        fn _assert_root_page_object<T: RootPageObject>(_: T) {}
    }

    #[test]
    fn test_root_selector_constant() {
        // Test that the ROOT_SELECTOR constant is accessible
        assert_eq!(MockRootPageObject::ROOT_SELECTOR, ".mock-root");
    }
}

#[cfg(test)]
mod wait_tests {
    use super::*;

    #[tokio::test]
    async fn test_wait_for_success() {
        // Test wait_for with a condition that succeeds immediately
        let config = WaitConfig {
            timeout: Duration::from_secs(1),
            poll_interval: Duration::from_millis(100),
        };

        let result = wait_for(
            || async { Ok(Some(42)) },
            &config,
            "test condition",
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_wait_for_timeout() {
        // Test wait_for with a condition that never succeeds
        let config = WaitConfig {
            timeout: Duration::from_millis(200),
            poll_interval: Duration::from_millis(50),
        };

        let result = wait_for(
            || async { Ok(None::<i32>) },
            &config,
            "test timeout condition",
        )
        .await;

        assert!(result.is_err());
        match result {
            Err(UtamError::Timeout { condition }) => {
                assert!(condition.contains("test timeout condition"));
            }
            _ => panic!("Expected Timeout error"),
        }
    }

    #[tokio::test]
    async fn test_wait_for_eventual_success() {
        // Test wait_for with a condition that succeeds after a few attempts
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_clone = counter.clone();

        let config = WaitConfig {
            timeout: Duration::from_secs(2),
            poll_interval: Duration::from_millis(100),
        };

        let result = wait_for(
            move || {
                let counter = counter_clone.clone();
                async move {
                    let count = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if count >= 3 {
                        Ok(Some("success"))
                    } else {
                        Ok(None)
                    }
                }
            },
            &config,
            "eventual success condition",
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert!(counter.load(std::sync::atomic::Ordering::SeqCst) >= 4);
    }

    #[test]
    fn test_wait_config_default() {
        // Test default WaitConfig values
        let config = WaitConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(10));
        assert_eq!(config.poll_interval, Duration::from_millis(500));
    }
}
