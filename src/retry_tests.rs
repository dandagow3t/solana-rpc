use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

#[test]
fn test_backoff_delay_calculation_with_jitter() {
    let base = Duration::from_millis(100);
    let max = Duration::from_secs(5);

    // With jitter, delays are randomized between 50-100% of base exponential value
    // For attempt 0: base * 2^0 = 100ms -> with jitter: 50-100ms
    let delay0 = calculate_backoff_delay(0, base, max);
    assert!(delay0 >= Duration::from_millis(50) && delay0 <= Duration::from_millis(100));

    // For attempt 1: base * 2^1 = 200ms -> with jitter: 100-200ms
    let delay1 = calculate_backoff_delay(1, base, max);
    assert!(delay1 >= Duration::from_millis(100) && delay1 <= Duration::from_millis(200));

    // For attempt 2: base * 2^2 = 400ms -> with jitter: 200-400ms
    let delay2 = calculate_backoff_delay(2, base, max);
    assert!(delay2 >= Duration::from_millis(200) && delay2 <= Duration::from_millis(400));

    // For attempt 3: base * 2^3 = 800ms -> with jitter: 400-800ms
    let delay3 = calculate_backoff_delay(3, base, max);
    assert!(delay3 >= Duration::from_millis(400) && delay3 <= Duration::from_millis(800));
}

#[test]
fn test_backoff_delay_capped_at_max() {
    let base = Duration::from_millis(500);
    let max = Duration::from_secs(2);

    // Attempt 0: base * 2^0 = 500ms -> with jitter: 250-500ms
    let delay0 = calculate_backoff_delay(0, base, max);
    assert!(delay0 >= Duration::from_millis(250) && delay0 <= Duration::from_millis(500));

    // Attempt 1: base * 2^1 = 1000ms -> with jitter: 500-1000ms
    let delay1 = calculate_backoff_delay(1, base, max);
    assert!(delay1 >= Duration::from_millis(500) && delay1 <= Duration::from_millis(1000));

    // Attempt 2: base * 2^2 = 2000ms -> with jitter: 1000-2000ms
    let delay2 = calculate_backoff_delay(2, base, max);
    assert!(delay2 >= Duration::from_millis(1000) && delay2 <= Duration::from_millis(2000));

    // Attempt 3: base * 2^3 = 4000ms -> capped to 2000ms -> with jitter: 1000-2000ms
    let delay3 = calculate_backoff_delay(3, base, max);
    assert!(delay3 >= Duration::from_millis(1000) && delay3 <= Duration::from_secs(2));

    // Attempt 10: way over max, should still cap to max
    let delay10 = calculate_backoff_delay(10, base, max);
    assert!(delay10 >= Duration::from_millis(1000) && delay10 <= Duration::from_secs(2));
}

#[tokio::test]
async fn test_retry_succeeds_immediately() {
    let config = RetryConfig {
        max_retries: 3,
        base_delay: Duration::from_millis(1),
        max_delay: Duration::from_millis(10),
        max_total_time: Duration::from_secs(1),
    };

    let result = with_retry(&config, || async { Ok::<_, RpcError>(42) }).await;
    assert_eq!(result.unwrap(), 42);
}

#[tokio::test]
async fn test_retry_succeeds_after_retries() {
    let config = RetryConfig {
        max_retries: 3,
        base_delay: Duration::from_millis(1),
        max_delay: Duration::from_millis(10),
        max_total_time: Duration::from_secs(1),
    };

    let attempt_count = Arc::new(AtomicU32::new(0));
    let count = attempt_count.clone();

    let result = with_retry(&config, || {
        let count = count.clone();
        async move {
            let attempt = count.fetch_add(1, Ordering::SeqCst);
            if attempt < 2 {
                Err(RpcError::Timeout {
                    message: "timed out".into(),
                })
            } else {
                Ok(42)
            }
        }
    })
    .await;

    assert_eq!(result.unwrap(), 42);
    assert_eq!(attempt_count.load(Ordering::SeqCst), 3); // 2 failures + 1 success
}

#[tokio::test]
async fn test_retry_exhausts_retries() {
    let config = RetryConfig {
        max_retries: 2,
        base_delay: Duration::from_millis(1),
        max_delay: Duration::from_millis(10),
        max_total_time: Duration::from_secs(1),
    };

    let attempt_count = Arc::new(AtomicU32::new(0));
    let count = attempt_count.clone();

    let result = with_retry(&config, || {
        let count = count.clone();
        async move {
            count.fetch_add(1, Ordering::SeqCst);
            Err::<i32, _>(RpcError::Timeout {
                message: "always fails".into(),
            })
        }
    })
    .await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), RpcError::Timeout { .. }));
    // initial attempt + 2 retries = 3 total
    assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_retry_non_retryable_error_returns_immediately() {
    let config = RetryConfig {
        max_retries: 5,
        base_delay: Duration::from_millis(1),
        max_delay: Duration::from_millis(10),
        max_total_time: Duration::from_secs(1),
    };

    let attempt_count = Arc::new(AtomicU32::new(0));
    let count = attempt_count.clone();

    let result = with_retry(&config, || {
        let count = count.clone();
        async move {
            count.fetch_add(1, Ordering::SeqCst);
            Err::<i32, _>(RpcError::SimulationFailed {
                message: "bad tx".into(),
            })
        }
    })
    .await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        RpcError::SimulationFailed { .. }
    ));
    // Should only try once since SimulationFailed is non-retryable
    assert_eq!(attempt_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_retry_respects_total_time_budget() {
    let config = RetryConfig {
        max_retries: 100, // Very high retry count (would normally take forever)
        base_delay: Duration::from_millis(50),
        max_delay: Duration::from_secs(10),
        max_total_time: Duration::from_millis(300), // Give up after 300ms total
    };

    let attempt_count = Arc::new(AtomicU32::new(0));
    let count = attempt_count.clone();

    let start = std::time::Instant::now();
    let result = with_retry(&config, || {
        let count = count.clone();
        async move {
            count.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(10)).await;
            Err::<i32, _>(RpcError::Timeout {
                message: "always fails".into(),
            })
        }
    })
    .await;
    let elapsed = start.elapsed();

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), RpcError::Timeout { .. }));

    // Should stop reasonably quickly (within 1 second, likely closer to 300-400ms)
    assert!(
        elapsed < Duration::from_secs(1),
        "Should stop within 1 second, took {:?}",
        elapsed
    );

    // Should not exhaust all 100 retries - should stop early due to time budget
    let attempts = attempt_count.load(Ordering::SeqCst);
    assert!(
        attempts < 20,
        "Should stop well before max_retries due to time budget, got {} attempts",
        attempts
    );
}
