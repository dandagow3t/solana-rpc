use crate::config::RetryConfig;
use crate::errors::RpcError;
use std::future::Future;
use std::time::Duration;
use tracing::{debug, warn};

/// Execute an async operation with exponential backoff retry.
///
/// Only retries on errors classified as retryable by `RpcError::is_retryable()`.
/// Non-retryable errors are returned immediately.
/// Respects both max_retries count and max_total_time budget.
pub async fn with_retry<F, Fut, T>(config: &RetryConfig, operation: F) -> Result<T, RpcError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, RpcError>>,
{
    let start_time = std::time::Instant::now();
    let mut last_error: Option<RpcError> = None;

    for attempt in 0..=config.max_retries {
        // Check if we've exceeded total time budget
        if start_time.elapsed() >= config.max_total_time {
            warn!(
                elapsed_ms = start_time.elapsed().as_millis() as u64,
                max_total_ms = config.max_total_time.as_millis() as u64,
                "Retry budget exhausted"
            );
            return Err(last_error.unwrap_or(RpcError::Timeout {
                message: "Max total retry time exceeded".into(),
            }));
        }

        match operation().await {
            Ok(result) => return Ok(result),
            Err(err) => {
                if !err.is_retryable() || attempt == config.max_retries {
                    return Err(err);
                }

                let delay = calculate_backoff_delay(attempt, config.base_delay, config.max_delay);
                warn!(
                    attempt = attempt + 1,
                    max_retries = config.max_retries,
                    delay_ms = delay.as_millis() as u64,
                    error = %err,
                    "Retryable error, backing off"
                );

                tokio::time::sleep(delay).await;
                last_error = Some(err);
            }
        }
    }

    Err(last_error.unwrap_or(RpcError::ConnectionError {
        message: "retry loop exhausted without error".into(),
    }))
}

/// Calculate delay with exponential backoff and jitter: base_delay * 2^attempt * jitter, capped at max_delay.
/// Jitter prevents thundering herd when multiple requests retry simultaneously.
fn calculate_backoff_delay(attempt: u32, base_delay: Duration, max_delay: Duration) -> Duration {
    let base = base_delay.saturating_mul(2u32.saturating_pow(attempt));
    let jitter = rand::random::<f64>(); // [0.0, 1.0)
    let delay = base.mul_f64(0.5 + (jitter * 0.5)); // Random delay between 50-100% of base
    let final_delay = delay.min(max_delay);
    debug!(
        attempt,
        base_delay_ms = base.as_millis() as u64,
        delay_ms = final_delay.as_millis() as u64,
        "Calculated backoff delay with jitter"
    );
    final_delay
}

#[cfg(test)]
#[path = "retry_tests.rs"]
mod retry_tests;
