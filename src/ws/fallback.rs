use crate::config::RpcConfig;
use crate::ws::client::WsClient;
use crate::ws::types::SubscriptionHandle;
use anyhow::Result;
use solana_sdk::commitment_config::CommitmentConfig;
use tracing::warn;

/// WebSocket client with automatic failover to a secondary provider.
///
/// Per-subscription failover: each subscribe call tries the primary first;
/// on error, it retries on the secondary (if configured and connected).
pub struct FallbackWsClient {
    primary: WsClient,
    secondary: Option<WsClient>,
}

impl FallbackWsClient {
    /// Connect to primary (required) and secondary (best-effort) WebSocket endpoints.
    ///
    /// The primary connection must succeed or this returns an error.
    /// The secondary connection is best-effort: a failure is logged as a warning
    /// and the client continues with `secondary = None`.
    pub async fn connect(
        primary_config: &RpcConfig,
        secondary_config: Option<&RpcConfig>,
    ) -> Result<Self> {
        let primary = WsClient::connect(primary_config).await?;

        let secondary = match secondary_config {
            Some(config) => match WsClient::connect(config).await {
                Ok(ws) => Some(ws),
                Err(e) => {
                    warn!(
                        error = %e,
                        "Failed to connect secondary WebSocket; continuing without fallback"
                    );
                    None
                }
            },
            None => None,
        };

        Ok(Self { primary, secondary })
    }

    /// Subscribe to log messages mentioning specified program IDs.
    ///
    /// Tries the primary connection first. On failure, falls back to the secondary.
    pub async fn logs_subscribe(
        &self,
        mentions: Vec<String>,
        commitment: Option<CommitmentConfig>,
    ) -> Result<SubscriptionHandle> {
        match self
            .primary
            .logs_subscribe(mentions.clone(), commitment)
            .await
        {
            Ok(handle) => Ok(handle),
            Err(primary_err) => {
                if let Some(ref secondary) = self.secondary {
                    warn!(
                        error = %primary_err,
                        "Primary WS logs_subscribe failed, trying secondary"
                    );
                    secondary.logs_subscribe(mentions, commitment).await
                } else {
                    Err(primary_err)
                }
            }
        }
    }

    /// Subscribe to changes on a specific account.
    ///
    /// Tries the primary connection first. On failure, falls back to the secondary.
    pub async fn account_subscribe(
        &self,
        pubkey: &str,
        commitment: Option<CommitmentConfig>,
    ) -> Result<SubscriptionHandle> {
        match self.primary.account_subscribe(pubkey, commitment).await {
            Ok(handle) => Ok(handle),
            Err(primary_err) => {
                if let Some(ref secondary) = self.secondary {
                    warn!(
                        error = %primary_err,
                        "Primary WS account_subscribe failed, trying secondary"
                    );
                    secondary.account_subscribe(pubkey, commitment).await
                } else {
                    Err(primary_err)
                }
            }
        }
    }

    /// Subscribe to signature confirmation status.
    ///
    /// Tries the primary connection first. On failure, falls back to the secondary.
    pub async fn signature_subscribe(
        &self,
        signature: &str,
        commitment: Option<CommitmentConfig>,
    ) -> Result<SubscriptionHandle> {
        match self
            .primary
            .signature_subscribe(signature, commitment)
            .await
        {
            Ok(handle) => Ok(handle),
            Err(primary_err) => {
                if let Some(ref secondary) = self.secondary {
                    warn!(
                        error = %primary_err,
                        "Primary WS signature_subscribe failed, trying secondary"
                    );
                    secondary.signature_subscribe(signature, commitment).await
                } else {
                    Err(primary_err)
                }
            }
        }
    }

    /// Returns `true` if either the primary or secondary client is connected.
    pub fn is_connected(&self) -> bool {
        self.primary.is_connected() || self.secondary.as_ref().map_or(false, |s| s.is_connected())
    }

    /// Returns `true` if a secondary WebSocket client is configured.
    pub fn is_fallback_configured(&self) -> bool {
        self.secondary.is_some()
    }
}
