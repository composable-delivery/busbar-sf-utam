//! Wait utilities for element conditions and page loading
//!
//! This module provides generic and specific wait functions for polling
//! element conditions with configurable timeout and polling intervals.

use crate::error::{UtamError, UtamResult};
use std::time::Duration;
use tokio::time::interval;

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
        Self {
            timeout: Duration::from_secs(30),
            poll_interval: Duration::from_millis(500),
        }
    }
}

/// Generic wait function that polls a predicate until it returns Some(T) or times out
///
/// # Arguments
///
/// * `predicate` - Function that returns `UtamResult<Option<T>>`. Returns `Some(T)` when condition is met.
/// * `config` - Wait configuration specifying timeout and poll interval
/// * `description` - Human-readable description of the condition for error messages
///
/// # Returns
///
/// Returns `Ok(T)` when the predicate returns `Some(T)` within the timeout period.
///
/// # Errors
///
/// * `UtamError::Timeout` - When the condition is not met within the timeout period
/// * Other errors from the predicate are propagated
///
/// # Examples
///
/// ```rust,ignore
/// use std::time::Duration;
/// use utam_core::wait::{wait_for, WaitConfig};
///
/// let config = WaitConfig {
///     timeout: Duration::from_secs(10),
///     ..Default::default()
/// };
///
/// let result = wait_for(
///     || async {
///         if some_condition().await? {
///             Ok(Some(()))
///         } else {
///             Ok(None)
///         }
///     },
///     &config,
///     "some condition to be true",
/// ).await?;
/// ```
pub async fn wait_for<T, F, Fut>(
    predicate: F,
    config: &WaitConfig,
    description: &str,
) -> UtamResult<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = UtamResult<Option<T>>>,
{
    let start = std::time::Instant::now();
    let mut interval = interval(config.poll_interval);

    loop {
        interval.tick().await;

        match predicate().await? {
            Some(value) => return Ok(value),
            None if start.elapsed() > config.timeout => {
                return Err(UtamError::Timeout {
                    condition: description.to_string(),
                });
            }
            None => continue,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wait_config_default_values() {
        let config = WaitConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(30));
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

        let result = wait_for(
            || async { Ok(Some(123)) },
            &config,
            "test",
        )
        .await;

        assert_eq!(result.unwrap(), 123);
    }

    #[tokio::test]
    async fn test_wait_for_timeout_error() {
        let config = WaitConfig {
            timeout: Duration::from_millis(200),
            poll_interval: Duration::from_millis(50),
        };

        let result: UtamResult<()> = wait_for(
            || async { Ok(None) },
            &config,
            "test condition",
        )
        .await;

        assert!(result.is_err());
        if let Err(UtamError::Timeout { condition }) = result {
            assert_eq!(condition, "test condition");
        } else {
            panic!("Expected Timeout error");
        }
    }
}
