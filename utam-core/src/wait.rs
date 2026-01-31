//! Wait utilities for element conditions and page loading
//!
//! This module provides generic wait functions for polling
//! element conditions with configurable timeout and polling intervals.

use std::future::Future;
use std::time::Duration;

use tokio::time::{sleep, timeout};

use crate::error::{UtamError, UtamResult};

/// Configuration for wait operations
#[derive(Debug, Clone)]
pub struct WaitConfig {
    /// Maximum time to wait for condition to be true
    pub timeout: Duration,
    /// Time between polling attempts
    pub poll_interval: Duration,
}

impl Default for WaitConfig {
    fn default() -> Self {
        Self { timeout: Duration::from_secs(10), poll_interval: Duration::from_millis(500) }
    }
}

/// Wait for a condition to be met by polling
///
/// Repeatedly polls the condition function until it returns `Some(T)`
/// or the timeout is reached.
///
/// # Arguments
///
/// * `condition` - Async function returning `UtamResult<Option<T>>`.
///   Returns `Some(T)` when the condition is met.
/// * `config` - Wait configuration specifying timeout and poll interval
/// * `description` - Human-readable description for error messages
///
/// # Errors
///
/// * `UtamError::Timeout` - When the condition is not met within the timeout
/// * Other errors from the condition function are propagated
pub async fn wait_for<F, Fut, T>(
    condition: F,
    config: &WaitConfig,
    description: &str,
) -> UtamResult<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = UtamResult<Option<T>>>,
{
    let result = timeout(config.timeout, async {
        loop {
            match condition().await {
                Ok(Some(value)) => return Ok(value),
                Ok(None) => {
                    sleep(config.poll_interval).await;
                }
                Err(e) => {
                    // Continue polling on transient errors
                    sleep(config.poll_interval).await;
                    let _ = e;
                }
            }
        }
    })
    .await;

    match result {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(UtamError::Timeout { condition: description.to_string() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wait_config_default_values() {
        let config = WaitConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(10));
        assert_eq!(config.poll_interval, Duration::from_millis(500));
    }

    #[test]
    fn test_wait_config_clone() {
        let config1 = WaitConfig {
            timeout: Duration::from_secs(10),
            poll_interval: Duration::from_millis(100),
        };
        let config2 = config1.clone();
        assert_eq!(config1.timeout, config2.timeout);
        assert_eq!(config1.poll_interval, config2.poll_interval);
    }

    #[tokio::test]
    async fn test_wait_for_immediate_success() {
        let config = WaitConfig {
            timeout: Duration::from_secs(1),
            poll_interval: Duration::from_millis(50),
        };

        let result = wait_for(|| async { Ok(Some(123)) }, &config, "test").await;
        assert_eq!(result.unwrap(), 123);
    }

    #[tokio::test]
    async fn test_wait_for_timeout_error() {
        let config = WaitConfig {
            timeout: Duration::from_millis(200),
            poll_interval: Duration::from_millis(50),
        };

        let result: UtamResult<()> =
            wait_for(|| async { Ok(None) }, &config, "test condition").await;

        assert!(result.is_err());
        if let Err(UtamError::Timeout { condition }) = result {
            assert_eq!(condition, "test condition");
        } else {
            panic!("Expected Timeout error");
        }
    }
}
