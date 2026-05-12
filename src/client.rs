use crate::config::RpcConfig;
use crate::errors::RpcError;
use crate::retry::with_retry;
use crate::traits::{DasProvider, EnhancedTransactionProvider, PriorityFeeProvider};
use crate::types::Provider;
use anyhow::{Context, Result};
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};
use solana_account_decoder::parse_token::UiTokenAmount;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_client::{GetConfirmedSignaturesForAddress2Config, SerializableTransaction};
use solana_client::rpc_config::{
    RpcProgramAccountsConfig, RpcSendTransactionConfig, RpcSimulateTransactionConfig,
    RpcTransactionConfig,
};
use solana_client::rpc_request::TokenAccountsFilter;
use solana_client::rpc_response::{
    RpcConfirmedTransactionStatusWithSignature, RpcKeyedAccount, RpcSimulateTransactionResult,
    RpcTokenAccountBalance, RpcVersionInfo,
};
use solana_sdk::account::Account;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use std::num::NonZeroU32;
use std::sync::Arc;
use tracing::debug;

type GovernorRateLimiter = RateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// Main Solana RPC client with rate limiting, retry, and provider-specific API support.
#[derive(Clone)]
pub struct SolanaRpc {
    config: RpcConfig,
    rpc_client: Arc<RpcClient>,
    http_client: reqwest::Client,
    rate_limiter: Arc<GovernorRateLimiter>,
    priority_fee_provider: Option<Arc<dyn PriorityFeeProvider>>,
    das_provider: Option<Arc<dyn DasProvider>>,
    enhanced_tx_provider: Option<Arc<dyn EnhancedTransactionProvider>>,
    #[cfg(feature = "websocket")]
    ws_client: Arc<tokio::sync::OnceCell<crate::ws::WsClient>>,
}

impl SolanaRpc {
    /// Create a new SolanaRpc client from the given config.
    ///
    /// Automatically initializes provider-specific APIs based on the provider type
    /// and enabled feature flags.
    ///
    /// WebSocket connections are initialized lazily on first access via [`ws()`](Self::ws).
    pub fn new(config: RpcConfig) -> Result<Self> {
        let rpc_client = Arc::new(RpcClient::new_with_timeout_and_commitment(
            config.rpc_url.clone(),
            config.timeout,
            config.commitment,
        ));

        let http_client = reqwest::Client::builder()
            .timeout(config.timeout)
            .pool_max_idle_per_host(config.rate_limit.requests_per_second as usize)
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .build()
            .context("Failed to build HTTP client")?;

        let quota = Quota::per_second(
            NonZeroU32::new(config.rate_limit.requests_per_second)
                .context("Rate limit must be > 0")?,
        );
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        let (priority_fee_provider, das_provider, enhanced_tx_provider) =
            build_providers(&config, &http_client);

        Ok(Self {
            config,
            rpc_client,
            http_client,
            rate_limiter,
            priority_fee_provider,
            das_provider,
            enhanced_tx_provider,
            #[cfg(feature = "websocket")]
            ws_client: Arc::new(tokio::sync::OnceCell::new()),
        })
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    /// Access the underlying `solana_client::nonblocking::rpc_client::RpcClient`.
    pub fn inner(&self) -> &RpcClient {
        &self.rpc_client
    }

    /// Get the current config.
    pub fn config(&self) -> &RpcConfig {
        &self.config
    }

    /// Get the provider type of this client.
    pub fn provider(&self) -> Provider {
        self.config.provider
    }

    /// Get the HTTP client (for custom requests).
    pub fn http_client(&self) -> &reqwest::Client {
        &self.http_client
    }

    /// Access priority fee provider (if available for this provider).
    pub fn priority_fees(&self) -> Result<&dyn PriorityFeeProvider, RpcError> {
        self.priority_fee_provider
            .as_deref()
            .ok_or_else(|| RpcError::UnsupportedFeature {
                feature: format!("priority_fees (provider: {})", self.config.provider),
            })
    }

    /// Access DAS provider (if available for this provider).
    pub fn das(&self) -> Result<&dyn DasProvider, RpcError> {
        self.das_provider
            .as_deref()
            .ok_or_else(|| RpcError::UnsupportedFeature {
                feature: format!("das (provider: {})", self.config.provider),
            })
    }

    /// Access enhanced transaction provider (if available for this provider).
    pub fn enhanced_transactions(&self) -> Result<&dyn EnhancedTransactionProvider, RpcError> {
        self.enhanced_tx_provider
            .as_deref()
            .ok_or_else(|| RpcError::UnsupportedFeature {
                feature: format!("enhanced_transactions (provider: {})", self.config.provider),
            })
    }

    /// Access the WebSocket client with lazy initialization.
    ///
    /// The WebSocket connection is established on the first call to this method.
    /// Subsequent calls return a reference to the same connection.
    ///
    /// # Errors
    /// Returns `RpcError::WebSocketError` if:
    /// - No WebSocket URL is configured
    /// - The WebSocket connection fails to establish
    ///
    /// # Example
    /// ```rust,no_run
    /// # use aignt_solana_rpc::{RpcConfig, SolanaRpc};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let rpc = SolanaRpc::new(RpcConfig::helius("your-api-key"))?;
    /// let ws = rpc.ws().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "websocket")]
    pub async fn ws(&self) -> Result<&crate::ws::WsClient, RpcError> {
        self.ws_client
            .get_or_try_init(|| async {
                if self.config.ws_url.is_none() {
                    return Err(anyhow::anyhow!("No WebSocket URL configured"));
                }
                crate::ws::WsClient::connect(&self.config).await
            })
            .await
            .map_err(|e| RpcError::WebSocketError {
                message: format!("Failed to initialize WebSocket: {}", e),
            })
    }

    /// Get WebSocket client if this instance's provider matches the requested provider.
    ///
    /// This is useful in fallback scenarios where you want to explicitly select
    /// which provider's WebSocket to use.
    ///
    /// # Arguments
    /// * `provider` - The provider to match against
    ///
    /// # Returns
    /// Returns the WebSocket client if the provider matches, otherwise returns an error.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use aignt_solana_rpc::{RpcConfig, SolanaRpc, Provider};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let rpc = SolanaRpc::new(RpcConfig::helius("key"))?;
    /// let ws = rpc.ws_by_provider(Provider::Helius).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "websocket")]
    pub async fn ws_by_provider(
        &self,
        provider: Provider,
    ) -> Result<&crate::ws::WsClient, RpcError> {
        if self.config.provider == provider {
            self.ws().await
        } else {
            Err(RpcError::WebSocketError {
                message: format!(
                    "Provider mismatch: requested {:?}, but this client is configured for {:?}",
                    provider, self.config.provider
                ),
            })
        }
    }

    // -----------------------------------------------------------------------
    // Rate-limited helpers
    // -----------------------------------------------------------------------

    async fn wait_for_rate_limit(&self) {
        self.rate_limiter.until_ready().await;
    }

    // -----------------------------------------------------------------------
    // Standard RPC methods (wrapped with rate limiting + retry)
    // -----------------------------------------------------------------------

    pub async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64, RpcError> {
        self.wait_for_rate_limit().await;
        let client = self.rpc_client.clone();
        let pk = *pubkey;
        with_retry(&self.config.retry, || {
            let client = client.clone();
            async move {
                client
                    .get_balance(&pk)
                    .await
                    .map_err(RpcError::SolanaClient)
            }
        })
        .await
    }

    pub async fn get_account(&self, pubkey: &Pubkey) -> Result<Account, RpcError> {
        self.wait_for_rate_limit().await;
        let client = self.rpc_client.clone();
        let pk = *pubkey;
        with_retry(&self.config.retry, || {
            let client = client.clone();
            async move {
                client
                    .get_account(&pk)
                    .await
                    .map_err(RpcError::SolanaClient)
            }
        })
        .await
    }

    pub async fn get_multiple_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<Option<Account>>, RpcError> {
        self.wait_for_rate_limit().await;
        let client = self.rpc_client.clone();
        let pks = Arc::new(pubkeys.to_vec());
        with_retry(&self.config.retry, || {
            let client = client.clone();
            let pks = pks.clone();
            async move {
                client
                    .get_multiple_accounts(&pks)
                    .await
                    .map_err(RpcError::SolanaClient)
            }
        })
        .await
    }

    pub async fn get_latest_blockhash(&self) -> Result<Hash, RpcError> {
        self.wait_for_rate_limit().await;
        let client = self.rpc_client.clone();
        with_retry(&self.config.retry, || {
            let client = client.clone();
            async move {
                client
                    .get_latest_blockhash()
                    .await
                    .map_err(RpcError::SolanaClient)
            }
        })
        .await
    }

    pub async fn get_slot(&self) -> Result<u64, RpcError> {
        self.wait_for_rate_limit().await;
        let client = self.rpc_client.clone();
        with_retry(&self.config.retry, || {
            let client = client.clone();
            async move { client.get_slot().await.map_err(RpcError::SolanaClient) }
        })
        .await
    }

    pub async fn send_transaction(
        &self,
        transaction: &(impl SerializableTransaction + Clone),
    ) -> Result<Signature, RpcError> {
        self.wait_for_rate_limit().await;
        // No retry for send_transaction to prevent duplicate submissions on timeout
        self.rpc_client
            .send_transaction(transaction)
            .await
            .map_err(RpcError::SolanaClient)
    }

    pub async fn send_transaction_with_config(
        &self,
        transaction: &(impl SerializableTransaction + Clone),
        send_config: RpcSendTransactionConfig,
    ) -> Result<Signature, RpcError> {
        self.wait_for_rate_limit().await;
        // No retry for send_transaction to prevent duplicate submissions on timeout
        self.rpc_client
            .send_transaction_with_config(transaction, send_config)
            .await
            .map_err(RpcError::SolanaClient)
    }

    pub async fn send_and_confirm_transaction(
        &self,
        transaction: &(impl SerializableTransaction + Clone),
    ) -> Result<Signature, RpcError> {
        self.wait_for_rate_limit().await;
        // No retry for send_and_confirm — it has its own internal retry logic.
        self.rpc_client
            .send_and_confirm_transaction(transaction)
            .await
            .map_err(RpcError::SolanaClient)
    }

    pub async fn simulate_transaction(
        &self,
        transaction: &(impl SerializableTransaction + Clone),
    ) -> Result<RpcSimulateTransactionResult, RpcError> {
        self.wait_for_rate_limit().await;
        let client = self.rpc_client.clone();
        let tx = transaction.clone();
        with_retry(&self.config.retry, || {
            let client = client.clone();
            let tx = tx.clone();
            async move {
                let response = client
                    .simulate_transaction(&tx)
                    .await
                    .map_err(RpcError::SolanaClient)?;
                Ok(response.value)
            }
        })
        .await
    }

    pub async fn simulate_transaction_with_config(
        &self,
        transaction: &(impl SerializableTransaction + Clone),
        sim_config: RpcSimulateTransactionConfig,
    ) -> Result<RpcSimulateTransactionResult, RpcError> {
        self.wait_for_rate_limit().await;
        let client = self.rpc_client.clone();
        let tx = transaction.clone();
        with_retry(&self.config.retry, || {
            let client = client.clone();
            let tx = tx.clone();
            let cfg = sim_config.clone();
            async move {
                let response = client
                    .simulate_transaction_with_config(&tx, cfg)
                    .await
                    .map_err(RpcError::SolanaClient)?;
                Ok(response.value)
            }
        })
        .await
    }

    pub async fn get_signature_statuses(
        &self,
        signatures: &[Signature],
    ) -> Result<Vec<Option<solana_transaction_status::TransactionStatus>>, RpcError> {
        self.wait_for_rate_limit().await;
        let client = self.rpc_client.clone();
        let sigs = Arc::new(signatures.to_vec());
        with_retry(&self.config.retry, || {
            let client = client.clone();
            let sigs = sigs.clone();
            async move {
                let response = client
                    .get_signature_statuses(&sigs)
                    .await
                    .map_err(RpcError::SolanaClient)?;
                Ok(response.value)
            }
        })
        .await
    }

    pub async fn get_transaction(
        &self,
        signature: &Signature,
        encoding: UiTransactionEncoding,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta, RpcError> {
        self.wait_for_rate_limit().await;
        let client = self.rpc_client.clone();
        let sig = *signature;
        with_retry(&self.config.retry, || {
            let client = client.clone();
            async move {
                client
                    .get_transaction(&sig, encoding)
                    .await
                    .map_err(RpcError::SolanaClient)
            }
        })
        .await
    }

    pub async fn get_program_accounts(
        &self,
        pubkey: &Pubkey,
    ) -> Result<Vec<(Pubkey, Account)>, RpcError> {
        self.wait_for_rate_limit().await;
        let client = self.rpc_client.clone();
        let pk = *pubkey;
        with_retry(&self.config.retry, || {
            let client = client.clone();
            async move {
                client
                    .get_program_accounts(&pk)
                    .await
                    .map_err(RpcError::SolanaClient)
            }
        })
        .await
    }

    pub async fn get_program_accounts_with_config(
        &self,
        pubkey: &Pubkey,
        rpc_config: RpcProgramAccountsConfig,
    ) -> Result<Vec<(Pubkey, Account)>, RpcError> {
        self.wait_for_rate_limit().await;
        let client = self.rpc_client.clone();
        let pk = *pubkey;
        with_retry(&self.config.retry, || {
            let client = client.clone();
            let cfg = rpc_config.clone();
            async move {
                client
                    .get_program_accounts_with_config(&pk, cfg)
                    .await
                    .map_err(RpcError::SolanaClient)
            }
        })
        .await
    }

    pub async fn get_version(&self) -> Result<RpcVersionInfo, RpcError> {
        self.wait_for_rate_limit().await;
        let client = self.rpc_client.clone();
        with_retry(&self.config.retry, || {
            let client = client.clone();
            async move { client.get_version().await.map_err(RpcError::SolanaClient) }
        })
        .await
    }

    pub async fn get_transaction_with_config(
        &self,
        signature: &Signature,
        config: RpcTransactionConfig,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta, RpcError> {
        self.wait_for_rate_limit().await;
        let client = self.rpc_client.clone();
        let sig = *signature;
        with_retry(&self.config.retry, || {
            let client = client.clone();
            let cfg = config;
            async move {
                client
                    .get_transaction_with_config(&sig, cfg)
                    .await
                    .map_err(RpcError::SolanaClient)
            }
        })
        .await
    }

    pub async fn get_token_largest_accounts(
        &self,
        mint: &Pubkey,
    ) -> Result<Vec<RpcTokenAccountBalance>, RpcError> {
        self.wait_for_rate_limit().await;
        let client = self.rpc_client.clone();
        let mint = *mint;
        with_retry(&self.config.retry, || {
            let client = client.clone();
            async move {
                client
                    .get_token_largest_accounts(&mint)
                    .await
                    .map_err(RpcError::SolanaClient)
            }
        })
        .await
    }

    pub async fn get_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        token_account_filter: TokenAccountsFilter,
    ) -> Result<Vec<RpcKeyedAccount>, RpcError> {
        self.wait_for_rate_limit().await;
        // Since TokenAccountsFilter doesn't implement Clone, we can't use with_retry
        // Call the RPC client directly
        self.rpc_client
            .get_token_accounts_by_owner(owner, token_account_filter)
            .await
            .map_err(RpcError::SolanaClient)
    }

    pub async fn get_token_account_balance(
        &self,
        token_account: &Pubkey,
    ) -> Result<UiTokenAmount, RpcError> {
        self.wait_for_rate_limit().await;
        let client = self.rpc_client.clone();
        let account = *token_account;
        with_retry(&self.config.retry, || {
            let client = client.clone();
            async move {
                client
                    .get_token_account_balance(&account)
                    .await
                    .map_err(RpcError::SolanaClient)
            }
        })
        .await
    }

    pub async fn get_signatures_for_address_with_config(
        &self,
        address: &Pubkey,
        config: GetConfirmedSignaturesForAddress2Config,
    ) -> Result<Vec<RpcConfirmedTransactionStatusWithSignature>, RpcError> {
        self.wait_for_rate_limit().await;
        // GetConfirmedSignaturesForAddress2Config doesn't implement Clone, so we can't use with_retry
        // Call the RPC client directly
        self.rpc_client
            .get_signatures_for_address_with_config(address, config)
            .await
            .map_err(RpcError::SolanaClient)
    }
}

/// Build provider-specific trait implementations based on config and feature flags.
fn build_providers(
    config: &RpcConfig,
    _http_client: &reqwest::Client,
) -> (
    Option<Arc<dyn PriorityFeeProvider>>,
    Option<Arc<dyn DasProvider>>,
    Option<Arc<dyn EnhancedTransactionProvider>>,
) {
    match config.provider {
        #[cfg(feature = "helius")]
        Provider::Helius => {
            let provider = Arc::new(crate::providers::helius::HeliusProvider::new(
                config.rpc_url.clone(),
                _http_client.clone(),
            ));
            (
                Some(provider.clone() as Arc<dyn PriorityFeeProvider>),
                Some(provider.clone() as Arc<dyn DasProvider>),
                Some(provider as Arc<dyn EnhancedTransactionProvider>),
            )
        }
        #[cfg(feature = "alchemy")]
        Provider::Alchemy => {
            let provider = Arc::new(crate::providers::alchemy::AlchemyProvider::new(
                config.rpc_url.clone(),
                _http_client.clone(),
            ));
            (
                Some(provider.clone() as Arc<dyn PriorityFeeProvider>),
                Some(provider.clone() as Arc<dyn DasProvider>),
                Some(provider as Arc<dyn EnhancedTransactionProvider>),
            )
        }
        _ => {
            debug!(
                provider = %config.provider,
                "No provider-specific APIs available"
            );
            (None, None, None)
        }
    }
}

crate::traits::rpc_operations::impl_rpc_operations!(SolanaRpc);

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;
