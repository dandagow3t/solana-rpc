use super::*;

#[test]
fn test_helius_config_url_construction() {
    let config = RpcConfig::helius("my-api-key");

    #[cfg(feature = "helius-gatekeeper")]
    {
        assert_eq!(
            config.rpc_url,
            "https://beta.helius-rpc.com/?api-key=my-api-key"
        );
        assert_eq!(
            config.ws_url.as_deref(),
            Some("wss://beta.helius-rpc.com/?api-key=my-api-key")
        );
    }

    #[cfg(not(feature = "helius-gatekeeper"))]
    {
        assert_eq!(
            config.rpc_url,
            "https://mainnet.helius-rpc.com/?api-key=my-api-key"
        );
        assert_eq!(
            config.ws_url.as_deref(),
            Some("wss://mainnet.helius-rpc.com/?api-key=my-api-key")
        );
    }

    assert_eq!(config.provider, Provider::Helius);
    assert_eq!(config.api_key.as_deref(), Some("my-api-key"));
}

#[test]
fn test_helius_gatekeeper_config_url_construction() {
    let config = RpcConfig::helius_gatekeeper("my-api-key");
    assert_eq!(
        config.rpc_url,
        "https://beta.helius-rpc.com/?api-key=my-api-key"
    );
    assert_eq!(
        config.ws_url.as_deref(),
        Some("wss://beta.helius-rpc.com/?api-key=my-api-key")
    );
    assert_eq!(config.provider, Provider::Helius);
    assert_eq!(config.api_key.as_deref(), Some("my-api-key"));
}

#[test]
fn test_alchemy_config_url_construction() {
    let config = RpcConfig::alchemy("my-alchemy-key");
    assert_eq!(
        config.rpc_url,
        "https://solana-mainnet.g.alchemy.com/v2/my-alchemy-key"
    );
    assert_eq!(
        config.ws_url.as_deref(),
        Some("wss://solana-mainnet.g.alchemy.com/v2/my-alchemy-key")
    );
    assert_eq!(config.provider, Provider::Alchemy);
    assert_eq!(config.api_key.as_deref(), Some("my-alchemy-key"));
}

#[test]
fn test_custom_config() {
    let config = RpcConfig::custom("https://my-rpc.com", Some("wss://my-rpc.com"));
    assert_eq!(config.rpc_url, "https://my-rpc.com");
    assert_eq!(config.ws_url.as_deref(), Some("wss://my-rpc.com"));
    assert_eq!(config.provider, Provider::Custom);
    assert!(config.api_key.is_none());
}

#[test]
fn test_custom_config_no_ws() {
    let config = RpcConfig::custom("https://my-rpc.com", None);
    assert!(config.ws_url.is_none());
}

#[test]
fn test_builder_overrides() {
    let config = RpcConfig::helius("key")
        .with_commitment(CommitmentConfig::processed())
        .with_timeout(Duration::from_secs(60))
        .with_retry(RetryConfig {
            max_retries: 5,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            max_total_time: Duration::from_secs(10),
        })
        .with_rate_limit(50);

    assert_eq!(config.commitment, CommitmentConfig::processed());
    assert_eq!(config.timeout, Duration::from_secs(60));
    assert_eq!(config.retry.max_retries, 5);
    assert_eq!(config.retry.base_delay, Duration::from_millis(100));
    assert_eq!(config.retry.max_delay, Duration::from_secs(5));
    assert_eq!(config.rate_limit.requests_per_second, 50);
}

#[test]
fn test_default_values() {
    let config = RpcConfig::helius("key");
    assert_eq!(config.commitment, CommitmentConfig::confirmed());
    assert_eq!(config.timeout, DEFAULT_TIMEOUT);
    assert_eq!(config.retry.max_retries, DEFAULT_MAX_RETRIES);
    assert_eq!(config.retry.base_delay, DEFAULT_RETRY_BASE_DELAY);
    assert_eq!(config.retry.max_delay, DEFAULT_RETRY_MAX_DELAY);
    assert_eq!(
        config.rate_limit.requests_per_second,
        DEFAULT_RATE_LIMIT_RPS
    );
}

#[test]
fn test_from_provider_helius() {
    let config = RpcConfig::from_provider(Provider::Helius, "hkey").unwrap();
    assert_eq!(config.provider, Provider::Helius);
    assert!(config.rpc_url.contains("hkey"));
}

#[test]
fn test_from_provider_alchemy() {
    let config = RpcConfig::from_provider(Provider::Alchemy, "akey").unwrap();
    assert_eq!(config.provider, Provider::Alchemy);
    assert!(config.rpc_url.contains("akey"));
}

#[test]
fn test_provider_from_str() {
    assert_eq!("helius".parse::<Provider>().unwrap(), Provider::Helius);
    assert_eq!("Helius".parse::<Provider>().unwrap(), Provider::Helius);
    assert_eq!("HELIUS".parse::<Provider>().unwrap(), Provider::Helius);
    assert_eq!("alchemy".parse::<Provider>().unwrap(), Provider::Alchemy);
    assert_eq!("Alchemy".parse::<Provider>().unwrap(), Provider::Alchemy);
    assert_eq!("custom".parse::<Provider>().unwrap(), Provider::Custom);
    assert!("unknown".parse::<Provider>().is_err());
}

#[test]
fn test_from_provider_custom_returns_error() {
    let result = RpcConfig::from_provider(Provider::Custom, "key");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("RpcConfig::custom()"));
}

#[test]
fn test_local_config() {
    let config = RpcConfig::local();
    assert_eq!(config.rpc_url, "http://127.0.0.1:8899");
    assert_eq!(config.ws_url.as_deref(), Some("ws://127.0.0.1:8900"));
    assert_eq!(config.provider, Provider::Local);
    assert!(config.api_key.is_none());
}

#[test]
fn test_from_provider_local() {
    let config = RpcConfig::from_provider(Provider::Local, "").unwrap();
    assert_eq!(config.provider, Provider::Local);
    assert_eq!(config.rpc_url, "http://127.0.0.1:8899");
    assert_eq!(config.ws_url.as_deref(), Some("ws://127.0.0.1:8900"));
    assert!(config.api_key.is_none());
}

#[test]
fn test_local_provider_from_str() {
    assert_eq!("local".parse::<Provider>().unwrap(), Provider::Local);
    assert_eq!("Local".parse::<Provider>().unwrap(), Provider::Local);
    assert_eq!("LOCAL".parse::<Provider>().unwrap(), Provider::Local);
}
