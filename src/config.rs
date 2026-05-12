use crate::constants::*;
#[allow(unused_imports)]
use crate::types::Provider;
use solana_sdk::commitment_config::CommitmentConfig;
use std::time::Duration;

/// Retry configuration for RPC requests.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub max_total_time: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_MAX_RETRIES,
            base_delay: DEFAULT_RETRY_BASE_DELAY,
            max_delay: DEFAULT_RETRY_MAX_DELAY,
            max_total_time: DEFAULT_RETRY_MAX_TOTAL_TIME,
        }
    }
}

/// Rate limit configuration.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_second: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: DEFAULT_RATE_LIMIT_RPS,
        }
    }
}

/// Configuration for the Solana RPC client.
#[derive(Debug, Clone)]
pub struct RpcConfig {
    /// Provider type.
    pub provider: Provider,
    /// Full RPC URL (HTTP).
    pub rpc_url: String,
    /// Full WebSocket URL (if applicable).
    pub ws_url: Option<String>,
    /// API key (if applicable).
    pub api_key: Option<String>,
    /// Commitment level for RPC requests.
    pub commitment: CommitmentConfig,
    /// Request timeout.
    pub timeout: Duration,
    /// Retry configuration.
    pub retry: RetryConfig,
    /// Rate limit configuration.
    pub rate_limit: RateLimitConfig,
}

impl RpcConfig {
    /// Create config for Helius provider.
    ///
    /// When the `helius-gatekeeper` feature is enabled, this will use the faster
    /// Gatekeeper beta endpoint (`https://beta.helius-rpc.com`).
    pub fn helius(api_key: &str) -> Self {
        #[cfg(feature = "helius-gatekeeper")]
        {
            Self {
                provider: Provider::Helius,
                rpc_url: format!("{HELIUS_GATEKEEPER_URL}/?api-key={api_key}"),
                ws_url: Some(format!("{HELIUS_GATEKEEPER_WS_URL}/?api-key={api_key}")),
                api_key: Some(api_key.to_string()),
                commitment: CommitmentConfig::confirmed(),
                timeout: DEFAULT_TIMEOUT,
                retry: RetryConfig::default(),
                rate_limit: RateLimitConfig::default(),
            }
        }
        #[cfg(not(feature = "helius-gatekeeper"))]
        {
            Self {
                provider: Provider::Helius,
                rpc_url: format!("{HELIUS_MAINNET_URL}/?api-key={api_key}"),
                ws_url: Some(format!("{HELIUS_MAINNET_WS_URL}/?api-key={api_key}")),
                api_key: Some(api_key.to_string()),
                commitment: CommitmentConfig::confirmed(),
                timeout: DEFAULT_TIMEOUT,
                retry: RetryConfig::default(),
                rate_limit: RateLimitConfig::default(),
            }
        }
    }

    /// Create config for Helius Gatekeeper provider (faster beta endpoint).
    pub fn helius_gatekeeper(api_key: &str) -> Self {
        Self {
            provider: Provider::Helius,
            rpc_url: format!("{HELIUS_GATEKEEPER_URL}/?api-key={api_key}"),
            ws_url: Some(format!("{HELIUS_GATEKEEPER_WS_URL}/?api-key={api_key}")),
            api_key: Some(api_key.to_string()),
            commitment: CommitmentConfig::confirmed(),
            timeout: DEFAULT_TIMEOUT,
            retry: RetryConfig::default(),
            rate_limit: RateLimitConfig::default(),
        }
    }

    /// Create config for Alchemy provider.
    pub fn alchemy(api_key: &str) -> Self {
        Self {
            provider: Provider::Alchemy,
            rpc_url: format!("{ALCHEMY_MAINNET_URL}/{api_key}"),
            ws_url: Some(format!("{ALCHEMY_MAINNET_WS_URL}/{api_key}")),
            api_key: Some(api_key.to_string()),
            commitment: CommitmentConfig::confirmed(),
            timeout: DEFAULT_TIMEOUT,
            retry: RetryConfig::default(),
            rate_limit: RateLimitConfig::default(),
        }
    }

    /// Create config for local validator (localhost:8899).
    pub fn local() -> Self {
        Self {
            provider: Provider::Local,
            rpc_url: LOCAL_RPC_URL.to_string(),
            ws_url: Some(LOCAL_WS_URL.to_string()),
            api_key: None,
            commitment: CommitmentConfig::confirmed(),
            timeout: DEFAULT_TIMEOUT,
            retry: RetryConfig::default(),
            rate_limit: RateLimitConfig::default(),
        }
    }

    /// Create config by provider name and API key.
    ///
    /// This enables fully config-driven provider selection -- read the provider
    /// name from an env var or config file and construct the right config
    /// without branching in application code.
    ///
    /// Accepts `Provider::Helius`, `Provider::Alchemy`, or `Provider::Local`.
    /// For custom endpoints use `RpcConfig::custom()` instead.
    ///
    /// # Errors
    /// Returns an error if called with `Provider::Custom` (use `RpcConfig::custom()` for that).
    pub fn from_provider(provider: Provider, api_key: &str) -> Result<Self, String> {
        match provider {
            Provider::Helius => Ok(Self::helius(api_key)),
            Provider::Alchemy => Ok(Self::alchemy(api_key)),
            Provider::Local => Ok(Self::local()),
            Provider::Custom => Err(
                "Use RpcConfig::custom() for custom providers -- it needs a URL, not an API key"
                    .to_string(),
            ),
        }
    }

    /// Create config for a custom RPC endpoint.
    pub fn custom(rpc_url: &str, ws_url: Option<&str>) -> Self {
        Self {
            provider: Provider::Custom,
            rpc_url: rpc_url.to_string(),
            ws_url: ws_url.map(|s| s.to_string()),
            api_key: None,
            commitment: CommitmentConfig::confirmed(),
            timeout: DEFAULT_TIMEOUT,
            retry: RetryConfig::default(),
            rate_limit: RateLimitConfig::default(),
        }
    }

    /// Override commitment level.
    pub fn with_commitment(mut self, commitment: CommitmentConfig) -> Self {
        self.commitment = commitment;
        self
    }

    /// Override request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Override retry configuration.
    pub fn with_retry(mut self, retry: RetryConfig) -> Self {
        self.retry = retry;
        self
    }

    /// Override rate limit (requests per second).
    pub fn with_rate_limit(mut self, rps: u32) -> Self {
        self.rate_limit = RateLimitConfig {
            requests_per_second: rps,
        };
        self
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
