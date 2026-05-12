mod common;

use aignt_solana_rpc::client::SolanaRpc;
use aignt_solana_rpc::config::RpcConfig;
use solana_sdk::pubkey::Pubkey;

/// Verify that the rate limiter caps throughput to roughly `rps` requests per second.
///
/// Strategy: set a low rate limit, fire many requests sequentially, and assert that
/// the elapsed time is consistent with the configured RPS (within a tolerance).
#[tokio::test]
async fn stress_rate_limiter_caps_throughput() {
    let server = common::mock_balance_server();

    let rps: u32 = 10;
    let total_requests: usize = 30; // should take ~3 seconds at 10 RPS

    let config = RpcConfig::custom(&server.url("/"), None).with_rate_limit(rps);
    let rpc = SolanaRpc::new(config).unwrap();

    let elapsed = common::fire_requests(&rpc, total_requests).await;

    // Governor's token-bucket allows an initial burst of up to `rps` tokens,
    // then refills at `rps` tokens/second.  So the minimum time is:
    //   (total_requests - burst_capacity) / rps
    // where burst_capacity = rps.
    let burst = rps as f64;
    let expected_secs = (total_requests as f64 - burst).max(0.0) / rps as f64;

    println!(
        "rate_limit={rps} RPS, requests={total_requests}, elapsed={:.3}s, expected>={expected_secs:.3}s",
        elapsed.as_secs_f64()
    );

    // Allow 20% tolerance below expected (governor may burst slightly).
    assert!(
        elapsed.as_secs_f64() >= expected_secs * 0.8,
        "Requests completed too fast ({:.3}s) for {rps} RPS limit (expected >= {:.3}s)",
        elapsed.as_secs_f64(),
        expected_secs * 0.8,
    );
}

/// Verify that a higher rate limit lets requests complete faster.
#[tokio::test]
async fn stress_higher_rate_limit_is_faster() {
    let server = common::mock_balance_server();
    let total_requests: usize = 20;

    // Slow config: 10 RPS
    let slow_config = RpcConfig::custom(&server.url("/"), None).with_rate_limit(10);
    let slow_rpc = SolanaRpc::new(slow_config).unwrap();
    let slow_elapsed = common::fire_requests(&slow_rpc, total_requests).await;

    // Fast config: 100 RPS
    let fast_config = RpcConfig::custom(&server.url("/"), None).with_rate_limit(100);
    let fast_rpc = SolanaRpc::new(fast_config).unwrap();
    let fast_elapsed = common::fire_requests(&fast_rpc, total_requests).await;

    println!(
        "slow (10 RPS): {:.3}s, fast (100 RPS): {:.3}s",
        slow_elapsed.as_secs_f64(),
        fast_elapsed.as_secs_f64()
    );

    // The 10 RPS run should be significantly slower than the 100 RPS run.
    assert!(
        slow_elapsed > fast_elapsed,
        "Expected 10 RPS ({:.3}s) to be slower than 100 RPS ({:.3}s)",
        slow_elapsed.as_secs_f64(),
        fast_elapsed.as_secs_f64()
    );
}

/// Burst test: fire many concurrent requests and verify none are rejected
/// (governor delays rather than rejects).
#[tokio::test]
async fn stress_burst_no_rejections() {
    let server = common::mock_balance_server();

    let rps: u32 = 10;
    let burst_size: usize = 50;

    let config = RpcConfig::custom(&server.url("/"), None).with_rate_limit(rps);
    let rpc = SolanaRpc::new(config).unwrap();
    let pubkey = Pubkey::default();

    // Spawn all requests concurrently.
    let mut handles = Vec::with_capacity(burst_size);
    for _ in 0..burst_size {
        let rpc = rpc.clone();
        handles.push(tokio::spawn(async move { rpc.get_balance(&pubkey).await }));
    }

    let mut successes = 0;
    let mut failures = 0;
    for handle in handles {
        match handle.await.unwrap() {
            Ok(_) => successes += 1,
            Err(e) => {
                // Rate-limit errors from the *mock* are not expected -- the governor
                // should delay, not reject.  Other errors (connection, etc.) might
                // happen under load, so we just count them.
                eprintln!("request error: {e}");
                failures += 1;
            }
        }
    }

    println!("burst={burst_size}, successes={successes}, failures={failures}");

    // All requests should succeed (governor delays, never rejects).
    assert_eq!(
        failures, 0,
        "Expected zero failures but got {failures} out of {burst_size}"
    );
}
