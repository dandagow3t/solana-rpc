use super::*;
use crate::config::RetryConfig;
use crate::types::Provider;
use solana_sdk::commitment_config::CommitmentConfig;
use std::time::Duration;

// Helper to safely set/remove env vars in tests (unsafe in Rust 2024 edition).
unsafe fn set_env(key: &str, val: &str) {
    unsafe { std::env::set_var(key, val) }
}
unsafe fn remove_env(key: &str) {
    unsafe { std::env::remove_var(key) }
}

// ---------------------------------------------------------------------------
// SolanaRpcBuilder::new -- provider parsing
// ---------------------------------------------------------------------------

#[test]
fn builder_new_helius() {
    let builder = SolanaRpcBuilder::new("helius", "test-key").unwrap();
    assert_eq!(builder.primary_provider, Provider::Helius);
    assert_eq!(builder.primary_api_key, "test-key");
    assert!(builder.secondary.is_none());
}

#[test]
fn builder_new_alchemy() {
    let builder = SolanaRpcBuilder::new("alchemy", "test-key").unwrap();
    assert_eq!(builder.primary_provider, Provider::Alchemy);
}

#[test]
fn builder_new_case_insensitive() {
    assert!(SolanaRpcBuilder::new("HELIUS", "k").is_ok());
    assert!(SolanaRpcBuilder::new("Alchemy", "k").is_ok());
    assert!(SolanaRpcBuilder::new("hElIuS", "k").is_ok());
}

#[test]
fn builder_new_invalid_provider() {
    let err = SolanaRpcBuilder::new("invalid", "key").unwrap_err();
    assert!(err.to_string().contains("invalid"), "got: {err}");
}

#[test]
fn builder_new_custom_rejected() {
    let err = SolanaRpcBuilder::new("custom", "key").unwrap_err();
    assert!(
        err.to_string().contains("Custom provider is not supported"),
        "got: {err}"
    );
}

// ---------------------------------------------------------------------------
// with_secondary
// ---------------------------------------------------------------------------

#[test]
fn builder_with_secondary() {
    let builder = SolanaRpcBuilder::new("helius", "key1")
        .unwrap()
        .with_secondary("alchemy", "key2")
        .unwrap();

    let (provider, key) = builder.secondary.as_ref().unwrap();
    assert_eq!(*provider, Provider::Alchemy);
    assert_eq!(key, "key2");
}

#[test]
fn builder_with_secondary_invalid() {
    let err = SolanaRpcBuilder::new("helius", "key")
        .unwrap()
        .with_secondary("unknown", "key2")
        .unwrap_err();
    assert!(err.to_string().contains("unknown"), "got: {err}");
}

#[test]
fn builder_with_secondary_custom_rejected() {
    let err = SolanaRpcBuilder::new("helius", "key")
        .unwrap()
        .with_secondary("custom", "key2")
        .unwrap_err();
    assert!(
        err.to_string().contains("Custom provider is not supported"),
        "got: {err}"
    );
}

// ---------------------------------------------------------------------------
// build -- single provider
// ---------------------------------------------------------------------------

#[test]
fn build_single_provider() {
    let client = SolanaRpcBuilder::new("helius", "test-key")
        .unwrap()
        .build()
        .unwrap();

    assert!(!client.is_fallback_configured());
    assert!(matches!(client, SolanaRpcClient::Single(_)));
    assert_eq!(client.config().provider, Provider::Helius);
    assert!(client.config().rpc_url.contains("test-key"));
}

#[test]
fn build_single_alchemy() {
    let client = SolanaRpcBuilder::new("alchemy", "alchemy-key")
        .unwrap()
        .build()
        .unwrap();

    assert!(!client.is_fallback_configured());
    assert_eq!(client.config().provider, Provider::Alchemy);
    assert!(client.config().rpc_url.contains("alchemy-key"));
}

#[test]
fn build_single_local() {
    let client = SolanaRpcBuilder::new("local", "").unwrap().build().unwrap();

    assert!(!client.is_fallback_configured());
    assert_eq!(client.config().provider, Provider::Local);
    assert_eq!(client.config().rpc_url, "http://127.0.0.1:8899");
    assert_eq!(
        client.config().ws_url.as_deref(),
        Some("ws://127.0.0.1:8900")
    );
}

// ---------------------------------------------------------------------------
// build -- with secondary (fallback)
// ---------------------------------------------------------------------------

#[test]
fn build_with_fallback() {
    let client = SolanaRpcBuilder::new("helius", "h-key")
        .unwrap()
        .with_secondary("alchemy", "a-key")
        .unwrap()
        .build()
        .unwrap();

    assert!(client.is_fallback_configured());
    assert!(matches!(client, SolanaRpcClient::Fallback(_)));
    // Primary config should be Helius
    assert_eq!(client.config().provider, Provider::Helius);
}

// ---------------------------------------------------------------------------
// build -- config overrides applied to both providers
// ---------------------------------------------------------------------------

#[test]
fn build_overrides_applied_single() {
    let client = SolanaRpcBuilder::new("helius", "key")
        .unwrap()
        .with_commitment(CommitmentConfig::finalized())
        .with_timeout(Duration::from_secs(99))
        .with_rate_limit(42)
        .with_retry(RetryConfig {
            max_retries: 7,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            max_total_time: Duration::from_secs(10),
        })
        .build()
        .unwrap();

    let cfg = client.config();
    assert_eq!(cfg.commitment, CommitmentConfig::finalized());
    assert_eq!(cfg.timeout, Duration::from_secs(99));
    assert_eq!(cfg.rate_limit.requests_per_second, 42);
    assert_eq!(cfg.retry.max_retries, 7);
}

#[test]
fn build_overrides_applied_fallback() {
    let client = SolanaRpcBuilder::new("helius", "h-key")
        .unwrap()
        .with_secondary("alchemy", "a-key")
        .unwrap()
        .with_commitment(CommitmentConfig::finalized())
        .with_timeout(Duration::from_secs(45))
        .with_rate_limit(100)
        .build()
        .unwrap();

    // Verify primary config
    let primary_cfg = client.config();
    assert_eq!(primary_cfg.commitment, CommitmentConfig::finalized());
    assert_eq!(primary_cfg.timeout, Duration::from_secs(45));
    assert_eq!(primary_cfg.rate_limit.requests_per_second, 100);

    // Verify secondary config via FallbackRpc accessor
    if let SolanaRpcClient::Fallback(ref rpc) = client {
        let secondary_cfg = rpc.secondary_config();
        assert_eq!(secondary_cfg.commitment, CommitmentConfig::finalized());
        assert_eq!(secondary_cfg.timeout, Duration::from_secs(45));
        assert_eq!(secondary_cfg.rate_limit.requests_per_second, 100);
        assert_eq!(secondary_cfg.provider, Provider::Alchemy);
    } else {
        panic!("Expected Fallback variant");
    }
}

// ---------------------------------------------------------------------------
// build -- failover-specific options
// ---------------------------------------------------------------------------

#[test]
fn build_fallback_threshold_and_cooldown() {
    let client = SolanaRpcBuilder::new("helius", "h-key")
        .unwrap()
        .with_secondary("alchemy", "a-key")
        .unwrap()
        .with_failure_threshold(10)
        .with_primary_cooldown(Duration::from_secs(300))
        .build()
        .unwrap();

    assert!(client.is_fallback_configured());
}

// ---------------------------------------------------------------------------
// from_env
// ---------------------------------------------------------------------------

#[test]
fn from_env_primary_only() {
    unsafe {
        set_env("PRIMARY_PROVIDER", "helius");
        set_env("HELIUS_API_KEY", "env-helius-key");
        remove_env("SECONDARY_PROVIDER");
    }

    let client = SolanaRpcBuilder::from_env().unwrap().build().unwrap();

    assert!(!client.is_fallback_configured());
    assert_eq!(client.config().provider, Provider::Helius);
    assert!(client.config().rpc_url.contains("env-helius-key"));

    unsafe {
        remove_env("PRIMARY_PROVIDER");
        remove_env("HELIUS_API_KEY");
    }
}

#[test]
fn from_env_with_secondary() {
    unsafe {
        set_env("PRIMARY_PROVIDER", "alchemy");
        set_env("ALCHEMY_API_KEY", "env-alchemy-key");
        set_env("SECONDARY_PROVIDER", "helius");
        set_env("HELIUS_API_KEY", "env-helius-key");
    }

    let client = SolanaRpcBuilder::from_env().unwrap().build().unwrap();

    assert!(client.is_fallback_configured());
    assert_eq!(client.config().provider, Provider::Alchemy);

    unsafe {
        remove_env("PRIMARY_PROVIDER");
        remove_env("ALCHEMY_API_KEY");
        remove_env("SECONDARY_PROVIDER");
        remove_env("HELIUS_API_KEY");
    }
}

#[test]
fn from_env_missing_primary_provider() {
    unsafe {
        remove_env("PRIMARY_PROVIDER");
    }

    let err = SolanaRpcBuilder::from_env().unwrap_err();
    assert!(err.to_string().contains("PRIMARY_PROVIDER"), "got: {err}");
}

#[test]
fn from_env_missing_primary_api_key() {
    unsafe {
        set_env("PRIMARY_PROVIDER", "helius");
        remove_env("HELIUS_API_KEY");
    }

    let err = SolanaRpcBuilder::from_env().unwrap_err();
    assert!(err.to_string().contains("HELIUS_API_KEY"), "got: {err}");

    unsafe {
        remove_env("PRIMARY_PROVIDER");
    }
}

#[test]
fn from_env_secondary_provider_without_key() {
    unsafe {
        set_env("PRIMARY_PROVIDER", "helius");
        set_env("HELIUS_API_KEY", "key");
        set_env("SECONDARY_PROVIDER", "alchemy");
        remove_env("ALCHEMY_API_KEY");
    }

    let err = SolanaRpcBuilder::from_env().unwrap_err();
    assert!(err.to_string().contains("ALCHEMY_API_KEY"), "got: {err}");

    unsafe {
        remove_env("PRIMARY_PROVIDER");
        remove_env("HELIUS_API_KEY");
        remove_env("SECONDARY_PROVIDER");
    }
}

// ---------------------------------------------------------------------------
// inner() and config() accessors
// ---------------------------------------------------------------------------

#[test]
fn accessors_single() {
    let client = SolanaRpcBuilder::new("helius", "key")
        .unwrap()
        .build()
        .unwrap();

    let _inner = client.inner();
    let cfg = client.config();
    assert_eq!(cfg.provider, Provider::Helius);
}

#[test]
fn accessors_fallback() {
    let client = SolanaRpcBuilder::new("helius", "h-key")
        .unwrap()
        .with_secondary("alchemy", "a-key")
        .unwrap()
        .build()
        .unwrap();

    let _inner = client.inner();
    assert_eq!(client.config().provider, Provider::Helius);
}
