use super::*;
use crate::config::RpcConfig;

#[test]
fn test_fallback_construction() {
    let primary = SolanaRpc::new(RpcConfig::helius("key1")).unwrap();
    let secondary = SolanaRpc::new(RpcConfig::alchemy("key2")).unwrap();
    let fallback = FallbackRpc::new(primary, secondary)
        .with_failure_threshold(5)
        .with_primary_cooldown(Duration::from_secs(120));

    assert_eq!(fallback.failure_threshold, 5);
    assert_eq!(fallback.primary_cooldown, Duration::from_secs(120));
}

#[test]
fn test_primary_starts_healthy() {
    let primary = SolanaRpc::new(RpcConfig::helius("key1")).unwrap();
    let secondary = SolanaRpc::new(RpcConfig::alchemy("key2")).unwrap();
    let fallback = FallbackRpc::new(primary, secondary);

    assert!(!fallback.is_primary_unhealthy());
    assert!(fallback.should_use_primary());
}

#[test]
fn test_record_failures_marks_unhealthy() {
    let primary = SolanaRpc::new(RpcConfig::helius("key1")).unwrap();
    let secondary = SolanaRpc::new(RpcConfig::alchemy("key2")).unwrap();
    let fallback = FallbackRpc::new(primary, secondary).with_failure_threshold(3);

    // First two failures shouldn't mark unhealthy
    fallback.record_primary_failure();
    fallback.record_primary_failure();
    assert!(!fallback.is_primary_unhealthy());

    // Third failure should mark unhealthy
    fallback.record_primary_failure();
    assert!(fallback.is_primary_unhealthy());
    assert!(!fallback.should_use_primary());
}

#[test]
fn test_primary_recovery_resets_counter() {
    let primary = SolanaRpc::new(RpcConfig::helius("key1")).unwrap();
    let secondary = SolanaRpc::new(RpcConfig::alchemy("key2")).unwrap();
    let fallback = FallbackRpc::new(primary, secondary).with_failure_threshold(3);

    fallback.record_primary_failure();
    fallback.record_primary_failure();
    assert_eq!(fallback.primary_failures.load(Ordering::SeqCst), 2);

    fallback.record_primary_success();
    assert_eq!(fallback.primary_failures.load(Ordering::SeqCst), 0);
    assert!(!fallback.is_primary_unhealthy());
}

#[test]
fn test_cooldown_allows_primary_probe() {
    let primary = SolanaRpc::new(RpcConfig::helius("key1")).unwrap();
    let secondary = SolanaRpc::new(RpcConfig::alchemy("key2")).unwrap();
    let fallback = FallbackRpc::new(primary, secondary)
        .with_failure_threshold(1)
        .with_primary_cooldown(Duration::from_millis(50));

    // Mark primary as unhealthy
    fallback.record_primary_failure();
    assert!(fallback.is_primary_unhealthy());
    assert!(!fallback.should_use_primary());

    // Wait for cooldown
    std::thread::sleep(Duration::from_millis(60));

    // Should now probe primary again
    assert!(fallback.should_use_primary());
}

#[tokio::test]
async fn test_config_accessors() {
    let primary = SolanaRpc::new(RpcConfig::helius("key1")).unwrap();
    let secondary = SolanaRpc::new(RpcConfig::alchemy("key2")).unwrap();
    let fallback = FallbackRpc::new(primary, secondary);

    assert_eq!(
        fallback.primary_config().provider,
        crate::types::Provider::Helius
    );
    assert_eq!(
        fallback.secondary_config().provider,
        crate::types::Provider::Alchemy
    );
}
