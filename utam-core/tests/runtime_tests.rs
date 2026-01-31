//! Integration tests for UTAM core runtime
//!
//! Tests runtime traits and element wrappers.

mod common;

use std::time::Duration;
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
fn test_wait_config_default() {
    // Test that WaitConfig has sensible defaults
    let config = WaitConfig::default();
    assert_eq!(config.timeout, Duration::from_secs(30));
    assert_eq!(config.poll_interval, Duration::from_millis(500));
}

#[test]
fn test_wait_config_custom() {
    // Test that WaitConfig can be customized
    let config =
        WaitConfig { timeout: Duration::from_secs(10), poll_interval: Duration::from_millis(100) };
    assert_eq!(config.timeout, Duration::from_secs(10));
    assert_eq!(config.poll_interval, Duration::from_millis(100));
}

#[tokio::test]
async fn test_wait_for_succeeds_immediately() {
    // Test wait_for when condition is immediately true
    let config =
        WaitConfig { timeout: Duration::from_secs(5), poll_interval: Duration::from_millis(100) };

    let result = wait_for(|| async { Ok(Some(42)) }, &config, "test condition").await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

#[tokio::test]
async fn test_wait_for_succeeds_after_polling() {
    // Test wait_for when condition becomes true after a few polls
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();

    let config =
        WaitConfig { timeout: Duration::from_secs(5), poll_interval: Duration::from_millis(100) };

    let result = wait_for(
        move || {
            let counter = counter_clone.clone();
            async move {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                if count >= 3 {
                    Ok(Some("success"))
                } else {
                    Ok(None)
                }
            }
        },
        &config,
        "counter to reach 3",
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");
    // Verify it polled at least 4 times (0, 1, 2, 3)
    assert!(counter.load(Ordering::SeqCst) >= 4);
}

#[tokio::test]
async fn test_wait_for_times_out() {
    // Test wait_for when condition never becomes true
    let config = WaitConfig {
        timeout: Duration::from_millis(500),
        poll_interval: Duration::from_millis(100),
    };

    let result: UtamResult<()> =
        wait_for(|| async { Ok(None) }, &config, "impossible condition").await;

    assert!(result.is_err());
    match result.unwrap_err() {
        UtamError::Timeout { condition } => {
            assert_eq!(condition, "impossible condition");
        }
        _ => panic!("Expected Timeout error"),
    }
}

#[tokio::test]
async fn test_wait_for_propagates_errors() {
    // Test that wait_for propagates errors from predicate
    let config =
        WaitConfig { timeout: Duration::from_secs(5), poll_interval: Duration::from_millis(100) };

    let result: UtamResult<()> = wait_for(
        || async {
            Err(UtamError::ElementNotFound {
                name: "test".to_string(),
                selector: ".test".to_string(),
            })
        },
        &config,
        "test condition",
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        UtamError::ElementNotFound { name, .. } => {
            assert_eq!(name, "test");
        }
        _ => panic!("Expected ElementNotFound error"),
    }
}

#[tokio::test]
async fn test_wait_respects_poll_interval() {
    // Test that wait_for respects the polling interval
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use std::time::Instant;

    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();
    let start = Instant::now();

    let config =
        WaitConfig { timeout: Duration::from_secs(2), poll_interval: Duration::from_millis(200) };

    let result = wait_for(
        move || {
            let counter = counter_clone.clone();
            async move {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                if count >= 3 {
                    Ok(Some(()))
                } else {
                    Ok(None)
                }
            }
        },
        &config,
        "counter to reach 3",
    )
    .await;

    let elapsed = start.elapsed();
    assert!(result.is_ok());
    // The first tick from tokio::time::interval completes immediately, then
    // three more ticks are needed (each after 200ms) to reach count >= 3.
    // Total minimum time: 3 * 200ms = 600ms
    // Allow some tolerance for timing
    assert!(elapsed >= Duration::from_millis(600), "Expected at least 600ms, got {:?}", elapsed);
}
fn test_error_messages_are_human_readable() {
    // Test ElementNotFound error message
    let error = UtamError::ElementNotFound {
        name: "submitButton".to_string(),
        selector: "button[type='submit']".to_string(),
    };
    let msg = error.to_string();
    assert!(msg.contains("submitButton"));
    assert!(msg.contains("button[type='submit']"));
    assert!(msg.contains("not found"));

    // Test Timeout error message
    let error = UtamError::Timeout { condition: "element to be visible".to_string() };
    let msg = error.to_string();
    assert!(msg.contains("Timeout"));
    assert!(msg.contains("element to be visible"));

    // Test ShadowRootNotFound error message
    let error = UtamError::ShadowRootNotFound { element: "customElement".to_string() };
    let msg = error.to_string();
    assert!(msg.contains("Shadow root not found"));
    assert!(msg.contains("customElement"));

    // Test InvalidSelector error message
    let error = UtamError::InvalidSelector { selector: "invalid>>selector".to_string() };
    let msg = error.to_string();
    assert!(msg.contains("Invalid selector"));
    assert!(msg.contains("invalid>>selector"));

    // Test FrameNotFound error message
    let error = UtamError::FrameNotFound { name: "paymentFrame".to_string() };
    let msg = error.to_string();
    assert!(msg.contains("Frame not found"));
    assert!(msg.contains("paymentFrame"));

    // Test AssertionFailed error message
    let error = UtamError::AssertionFailed {
        expected: "visible".to_string(),
        actual: "hidden".to_string(),
    };
    let msg = error.to_string();
    assert!(msg.contains("Assertion failed"));
    assert!(msg.contains("expected visible"));
    assert!(msg.contains("got hidden"));
}

#[test]
fn test_webdriver_error_conversion() {
    // Test that WebDriver errors can be converted to UtamError
    use thirtyfour::error::WebDriverError;

    // Create a WebDriver error (using ParseError variant which takes a String)
    let wd_error = WebDriverError::ParseError("test parse error".to_string());

    // Convert it to UtamError
    let utam_error: UtamError = wd_error.into();

    // Verify it's the WebDriver variant
    let msg = utam_error.to_string();
    assert!(msg.contains("WebDriver error"));
    assert!(msg.contains("parse error"));
}

#[test]
fn test_error_context_preserved() {
    // Test that error context (name, selector, etc.) is preserved
    let name = "loginButton".to_string();
    let selector = "#login-btn".to_string();

    let error = UtamError::ElementNotFound { name: name.clone(), selector: selector.clone() };

    // Verify context is accessible via display
    let msg = error.to_string();
    assert!(msg.contains(&name));
    assert!(msg.contains(&selector));

    // Test UtamResult type alias works correctly
    let result: UtamResult<String> = Err(error);
    assert!(result.is_err());

    if let Err(e) = result {
        let msg = e.to_string();
        assert!(msg.contains("loginButton"));
        assert!(msg.contains("#login-btn"));
    }
}
