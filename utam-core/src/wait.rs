//! Wait utilities for UTAM operations
//!
//! Provides configurable wait functions for polling conditions.

use crate::error::{UtamError, UtamResult};
use std::future::Future;
use std::time::Duration;
use tokio::time::{sleep, timeout};

/// Configuration for wait operations
#[derive(Debug, Clone)]
pub struct WaitConfig {
    /// Maximum time to wait for condition
    pub timeout: Duration,
    /// Time between polling attempts
    pub poll_interval: Duration,
}

impl Default for WaitConfig {
    fn default() -> Self {
        Self { timeout: Duration::from_secs(10), poll_interval: Duration::from_millis(500) }
    }
}

/// Wait for a condition to be met
///
/// Repeatedly polls the condition function until it returns Some(value)
/// or the timeout is reached.
///
/// # Arguments
///
/// * `condition` - Async function that returns Option<T>
/// * `config` - Wait configuration
/// * `description` - Description of what we're waiting for (for error messages)
///
/// # Returns
///
/// The value from the condition function if it succeeds within the timeout
///
/// # Errors
///
/// * `UtamError::Timeout` - If condition is not met within timeout
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
                    // Continue polling on errors, but sleep first
                    sleep(config.poll_interval).await;
                    // Log or ignore the error
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
