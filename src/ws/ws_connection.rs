use crate::config::RpcConfig;
use crate::types::Provider;
use crate::ws::client::WsClient;
use crate::ws::fallback::FallbackWsClient;
use crate::ws::types::SubscriptionHandle;
use anyhow::Result;
use solana_sdk::commitment_config::CommitmentConfig;

/// A unified WebSocket client that wraps either a single [`WsClient`] or a
/// [`FallbackWsClient`] (primary + secondary with per-subscription failover).
///
/// Use [`WsClientBuilder`] to construct an instance.
///
/// # Examples
///
/// ```rust,no_run
/// use aignt_solana_rpc::ws::ws_connection::WsClientBuilder;
///
/// # async fn example() -> anyhow::Result<()> {
/// // Single provider
/// let ws = WsClientBuilder::new("helius", "your-api-key")?
///     .build()
///     .await?;
///
/// // With failover (independent from RPC config)
/// let ws = WsClientBuilder::new("alchemy", "ws-alchemy-key")?
///     .with_secondary("helius", "ws-helius-key")?
///     .build()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub enum WsConnection {
    /// A single-provider WebSocket client.
    Single(WsClient),
    /// A primary + secondary WebSocket client with per-subscription failover.
    Fallback(FallbackWsClient),
}

impl WsConnection {
    /// Subscribe to log messages mentioning specified program IDs.
    pub async fn logs_subscribe(
        &self,
        mentions: Vec<String>,
        commitment: Option<CommitmentConfig>,
    ) -> Result<SubscriptionHandle> {
        match self {
            Self::Single(ws) => ws.logs_subscribe(mentions, commitment).await,
            Self::Fallback(ws) => ws.logs_subscribe(mentions, commitment).await,
        }
    }

    /// Subscribe to changes on a specific account.
    pub async fn account_subscribe(
        &self,
        pubkey: &str,
        commitment: Option<CommitmentConfig>,
    ) -> Result<SubscriptionHandle> {
        match self {
            Self::Single(ws) => ws.account_subscribe(pubkey, commitment).await,
            Self::Fallback(ws) => ws.account_subscribe(pubkey, commitment).await,
        }
    }

    /// Subscribe to signature confirmation status.
    pub async fn signature_subscribe(
        &self,
        signature: &str,
        commitment: Option<CommitmentConfig>,
    ) -> Result<SubscriptionHandle> {
        match self {
            Self::Single(ws) => ws.signature_subscribe(signature, commitment).await,
            Self::Fallback(ws) => ws.signature_subscribe(signature, commitment).await,
        }
    }

    /// Returns `true` if the underlying client is connected.
    pub fn is_connected(&self) -> bool {
        match self {
            Self::Single(ws) => ws.is_connected(),
            Self::Fallback(ws) => ws.is_connected(),
        }
    }

    /// Returns `true` if a secondary WebSocket provider is configured for failover.
    pub fn is_fallback_configured(&self) -> bool {
        matches!(self, Self::Fallback(ws) if ws.is_fallback_configured())
    }
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Builder for constructing a [`WsConnection`].
///
/// WebSocket providers are configured **independently** from RPC providers.
/// This allows the WS primary/secondary to be in a different order than the
/// RPC primary/secondary (e.g. RPC primary=Helius but WS primary=Alchemy).
///
/// # Examples
///
/// ```rust,no_run
/// use aignt_solana_rpc::ws::ws_connection::WsClientBuilder;
///
/// # async fn example() -> anyhow::Result<()> {
/// // Independent from RPC config
/// let ws = WsClientBuilder::new("alchemy", "ws-alchemy-key")?
///     .with_secondary("helius", "ws-helius-key")?
///     .build()
///     .await?;
///
/// // From environment variables
/// let ws = WsClientBuilder::from_env()?.build().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct WsClientBuilder {
    primary_provider: Provider,
    primary_api_key: String,
    secondary: Option<(Provider, String)>,
}

impl WsClientBuilder {
    /// Create a new builder with the required primary WebSocket provider.
    ///
    /// The `provider` string is case-insensitive: `"helius"`, `"Helius"`, `"HELIUS"` all work.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider name is not recognized or is `"custom"`.
    pub fn new(provider: &str, api_key: &str) -> Result<Self> {
        let primary_provider: Provider =
            provider.parse().map_err(|e: String| anyhow::anyhow!(e))?;
        if primary_provider == Provider::Custom {
            anyhow::bail!(
                "Custom provider is not supported in WsClientBuilder; use WsClient::connect() directly"
            );
        }
        Ok(Self {
            primary_provider,
            primary_api_key: api_key.to_string(),
            secondary: None,
        })
    }

    /// Create a builder from environment variables.
    ///
    /// Reads (separate from RPC provider selection):
    /// - `WS_PRIMARY_PROVIDER` (required) -- e.g. `"helius"`, `"alchemy"`, `"local"`
    /// - `WS_SECONDARY_PROVIDER` (optional) -- e.g. `"alchemy"`, `"helius"`, `"local"`
    ///
    /// API keys are resolved automatically from the provider name:
    /// `WS_PRIMARY_PROVIDER=alchemy` looks up `ALCHEMY_API_KEY`,
    /// `WS_SECONDARY_PROVIDER=helius` looks up `HELIUS_API_KEY`.
    /// `Provider::Local` does not require an API key.
    ///
    /// # Errors
    ///
    /// Returns an error if required env vars are missing or provider names are invalid.
    pub fn from_env() -> Result<Self> {
        let primary_name = std::env::var("WS_PRIMARY_PROVIDER")
            .map_err(|_| anyhow::anyhow!("WS_PRIMARY_PROVIDER env var is required"))?;

        let primary_provider: Provider = primary_name
            .parse()
            .map_err(|e: String| anyhow::anyhow!(e))?;
        if primary_provider == Provider::Custom {
            anyhow::bail!(
                "Custom provider is not supported in WsClientBuilder; use WsClient::connect() directly"
            );
        }

        let primary_key = if let Some(key_var) = primary_provider.api_key_env_var() {
            std::env::var(key_var)
                .ok()
                .filter(|k| !k.is_empty())
                .ok_or_else(|| {
                    anyhow::anyhow!("{key_var} env var is required for provider '{primary_name}'")
                })?
        } else {
            String::new()
        };

        let mut builder = Self::new(&primary_name, &primary_key)?;

        if let Ok(secondary_name) = std::env::var("WS_SECONDARY_PROVIDER") {
            if !secondary_name.is_empty() {
                let secondary_provider: Provider = secondary_name
                    .parse()
                    .map_err(|e: String| anyhow::anyhow!(e))?;
                if secondary_provider == Provider::Custom {
                    anyhow::bail!(
                        "Custom provider is not supported as secondary in WsClientBuilder; use FallbackWsClient directly"
                    );
                }

                let secondary_key = if let Some(key_var) = secondary_provider.api_key_env_var() {
                    std::env::var(key_var)
                        .ok()
                        .filter(|k| !k.is_empty())
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "{key_var} env var is required for provider '{secondary_name}'"
                            )
                        })?
                } else {
                    String::new()
                };

                builder = builder.with_secondary(&secondary_name, &secondary_key)?;
            }
        }

        Ok(builder)
    }

    /// Add a secondary WebSocket provider for per-subscription failover.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider name is not recognized or is `"custom"`.
    pub fn with_secondary(mut self, provider: &str, api_key: &str) -> Result<Self> {
        let secondary_provider: Provider =
            provider.parse().map_err(|e: String| anyhow::anyhow!(e))?;
        if secondary_provider == Provider::Custom {
            anyhow::bail!(
                "Custom provider is not supported as secondary in WsClientBuilder; use FallbackWsClient directly"
            );
        }
        self.secondary = Some((secondary_provider, api_key.to_string()));
        Ok(self)
    }

    /// Build the [`WsConnection`] by establishing WebSocket connections.
    ///
    /// If no secondary provider was configured, returns `WsConnection::Single`.
    /// If a secondary was configured, returns `WsConnection::Fallback` with
    /// per-subscription failover.
    ///
    /// # Errors
    ///
    /// Returns an error if the primary WebSocket connection fails.
    /// Secondary connection failure is logged as a warning (best-effort).
    pub async fn build(self) -> Result<WsConnection> {
        let primary_config = RpcConfig::from_provider(self.primary_provider, &self.primary_api_key)
            .map_err(|e| anyhow::anyhow!(e))?;

        match self.secondary {
            None => {
                let ws = WsClient::connect(&primary_config).await?;
                Ok(WsConnection::Single(ws))
            }
            Some((secondary_provider, secondary_api_key)) => {
                let secondary_config =
                    RpcConfig::from_provider(secondary_provider, &secondary_api_key)
                        .map_err(|e| anyhow::anyhow!(e))?;
                let ws =
                    FallbackWsClient::connect(&primary_config, Some(&secondary_config)).await?;
                Ok(WsConnection::Fallback(ws))
            }
        }
    }
}

#[cfg(test)]
#[path = "ws_connection_tests.rs"]
mod ws_connection_tests;
