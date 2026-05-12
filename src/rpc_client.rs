use crate::client::SolanaRpc;
use crate::config::{RetryConfig, RpcConfig};
use crate::errors::RpcError;
use crate::fallback::FallbackRpc;
use crate::types::{
    DasAsset, EnhancedTransaction, GetAssetsByOwnerRequest, GetAssetsByOwnerResponse,
    PriorityFeeEstimate, PriorityFeeRequest, Provider,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_client::{GetConfirmedSignaturesForAddress2Config, SerializableTransaction};
use solana_client::rpc_config::{
    RpcProgramAccountsConfig, RpcSendTransactionConfig, RpcSimulateTransactionConfig,
    RpcTransactionConfig,
};
use solana_client::rpc_request::TokenAccountsFilter;
use solana_client::rpc_response::{
    RpcConfirmedTransactionStatusWithSignature, RpcKeyedAccount, RpcSimulateTransactionResult,
    RpcVersionInfo,
};
use solana_sdk::account::Account;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use std::time::Duration;

/// Macro for delegating standard RPC methods to both `Single` and `Fallback` variants.
macro_rules! delegate_rpc {
    ($(
        $(#[$meta:meta])*
        fn $method:ident($($param:ident : $ty:ty),*) -> $ret:ty;
    )*) => {
        $(
            $(#[$meta])*
            pub async fn $method(&self, $($param: $ty),*) -> $ret {
                match self {
                    Self::Single(rpc) => rpc.$method($($param),*).await,
                    Self::Fallback(rpc) => rpc.$method($($param),*).await,
                }
            }
        )*
    };
}

/// A unified Solana RPC client that wraps either a single [`SolanaRpc`] or a
/// [`FallbackRpc`] (primary + secondary with automatic failover).
///
/// Use [`SolanaRpcBuilder`] to construct an instance.
///
/// # Examples
///
/// ```rust,no_run
/// use aignt_solana_rpc::rpc_client::SolanaRpcBuilder;
///
/// # fn example() -> anyhow::Result<()> {
/// // Single provider
/// let rpc = SolanaRpcBuilder::new("helius", "your-api-key")?
///     .build()?;
///
/// // With failover
/// let rpc = SolanaRpcBuilder::new("helius", "helius-key")?
///     .with_secondary("alchemy", "alchemy-key")?
///     .build()?;
///
/// // From environment variables
/// let rpc = SolanaRpcBuilder::from_env()?.build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub enum SolanaRpcClient {
    /// A single-provider client.
    Single(SolanaRpc),
    /// A primary + secondary client with automatic failover.
    Fallback(FallbackRpc),
}

impl SolanaRpcClient {
    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    /// Access the underlying `RpcClient` (primary's for fallback configurations).
    pub fn inner(&self) -> &RpcClient {
        match self {
            Self::Single(rpc) => rpc.inner(),
            Self::Fallback(rpc) => rpc.primary_inner(),
        }
    }

    /// Get the primary provider's config.
    pub fn config(&self) -> &RpcConfig {
        match self {
            Self::Single(rpc) => rpc.config(),
            Self::Fallback(rpc) => rpc.primary_config(),
        }
    }

    /// Returns `true` if a secondary provider is configured for failover.
    pub fn is_fallback_configured(&self) -> bool {
        matches!(self, Self::Fallback(_))
    }

    // -----------------------------------------------------------------------
    // Provider client accessors
    // -----------------------------------------------------------------------

    /// Get the primary/single client.
    ///
    /// For `Single` variant, returns the single client.
    /// For `Fallback` variant, returns the primary client.
    pub fn primary(&self) -> &SolanaRpc {
        match self {
            Self::Single(rpc) => rpc,
            Self::Fallback(rpc) => rpc.primary(),
        }
    }

    /// Get the secondary client if configured.
    ///
    /// Returns `None` for `Single` variant.
    /// Returns `Some(&SolanaRpc)` for `Fallback` variant.
    pub fn secondary(&self) -> Option<&SolanaRpc> {
        match self {
            Self::Single(_) => None,
            Self::Fallback(rpc) => Some(rpc.secondary()),
        }
    }

    /// Get all configured clients as a vector.
    ///
    /// Returns 1 client for `Single`, 2 clients for `Fallback`.
    pub fn all_clients(&self) -> Vec<&SolanaRpc> {
        match self {
            Self::Single(rpc) => vec![rpc],
            Self::Fallback(rpc) => vec![rpc.primary(), rpc.secondary()],
        }
    }

    /// Find a client by provider type.
    ///
    /// Returns the first client matching the provider type.
    /// Returns `None` if no client with that provider is configured.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use aignt_solana_rpc::rpc_client::SolanaRpcBuilder;
    /// use aignt_solana_rpc::types::Provider;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let client = SolanaRpcBuilder::new("alchemy", "alchemy-key")?
    ///     .with_secondary("helius", "helius-key")?
    ///     .build()?;
    ///
    /// // Explicitly access Helius provider
    /// if let Some(helius) = client.client_by_provider(Provider::Helius) {
    ///     // Use helius-specific methods
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn client_by_provider(&self, provider: Provider) -> Option<&SolanaRpc> {
        self.all_clients()
            .into_iter()
            .find(|client| client.provider() == provider)
    }

    // -----------------------------------------------------------------------
    // WebSocket accessors
    // -----------------------------------------------------------------------

    /// Get WebSocket client from the primary/single provider.
    ///
    /// For `Single` variant, returns the WebSocket client from the single provider.
    /// For `Fallback` variant, returns the WebSocket client from the primary provider.
    ///
    /// The WebSocket connection is established lazily on the first call.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use aignt_solana_rpc::rpc_client::SolanaRpcBuilder;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let rpc = SolanaRpcBuilder::new("helius", "your-api-key")?.build()?;
    /// let ws = rpc.ws().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "websocket")]
    pub async fn ws(&self) -> Result<&crate::ws::WsClient, RpcError> {
        match self {
            Self::Single(rpc) => rpc.ws().await,
            Self::Fallback(fallback) => fallback.primary().ws().await,
        }
    }

    /// Get WebSocket client from a specific provider.
    ///
    /// For single-provider setups, the provider must match the configured provider.
    /// For fallback setups, returns WebSocket from the matching provider (primary or secondary).
    ///
    /// This is particularly useful when you want to use a different provider for WebSocket
    /// than for HTTP RPC calls. For example, using Alchemy for HTTP with Helius fallback,
    /// but explicitly using Helius WebSocket (because Alchemy doesn't support WebSocket well).
    ///
    /// # Arguments
    /// * `provider` - The provider to get the WebSocket client for
    ///
    /// # Example
    /// ```rust,no_run
    /// # use aignt_solana_rpc::rpc_client::SolanaRpcBuilder;
    /// # use aignt_solana_rpc::types::Provider;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Alchemy for HTTP (with Helius fallback), Helius for WebSocket
    /// let rpc = SolanaRpcBuilder::new("alchemy", "alchemy-key")?
    ///     .with_secondary("helius", "helius-key")?
    ///     .build()?;
    ///
    /// // HTTP calls use Alchemy with automatic failover to Helius
    /// let balance = rpc.get_balance(&pubkey).await?;
    ///
    /// // WebSocket explicitly uses Helius
    /// let ws = rpc.ws_by_provider(Provider::Helius).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "websocket")]
    pub async fn ws_by_provider(
        &self,
        provider: Provider,
    ) -> Result<&crate::ws::WsClient, RpcError> {
        match self {
            Self::Single(rpc) => rpc.ws_by_provider(provider).await,
            Self::Fallback(fallback) => fallback.ws_by_provider(provider).await,
        }
    }

    // -----------------------------------------------------------------------
    // Standard RPC methods (delegated to both variants)
    // -----------------------------------------------------------------------

    delegate_rpc! {
        fn get_balance(pubkey: &Pubkey) -> Result<u64, RpcError>;
        fn get_account(pubkey: &Pubkey) -> Result<Account, RpcError>;
        fn get_multiple_accounts(pubkeys: &[Pubkey]) -> Result<Vec<Option<Account>>, RpcError>;
        fn get_latest_blockhash() -> Result<Hash, RpcError>;
        fn get_slot() -> Result<u64, RpcError>;
        fn get_version() -> Result<RpcVersionInfo, RpcError>;
        fn get_signature_statuses(signatures: &[Signature]) -> Result<Vec<Option<solana_transaction_status::TransactionStatus>>, RpcError>;
        fn get_transaction(signature: &Signature, encoding: UiTransactionEncoding) -> Result<EncodedConfirmedTransactionWithStatusMeta, RpcError>;
        fn get_program_accounts(pubkey: &Pubkey) -> Result<Vec<(Pubkey, Account)>, RpcError>;
        fn get_program_accounts_with_config(pubkey: &Pubkey, rpc_config: RpcProgramAccountsConfig) -> Result<Vec<(Pubkey, Account)>, RpcError>;
        fn get_transaction_with_config(signature: &Signature, config: RpcTransactionConfig) -> Result<EncodedConfirmedTransactionWithStatusMeta, RpcError>;
        fn get_token_largest_accounts(mint: &Pubkey) -> Result<Vec<solana_client::rpc_response::RpcTokenAccountBalance>, RpcError>;
        fn get_token_account_balance(token_account: &Pubkey) -> Result<solana_account_decoder::parse_token::UiTokenAmount, RpcError>;
    }

    // Generic transaction methods (can't use delegate_rpc! macro with impl Trait params)

    pub async fn send_transaction(
        &self,
        transaction: &(impl SerializableTransaction + Clone),
    ) -> Result<Signature, RpcError> {
        match self {
            Self::Single(rpc) => rpc.send_transaction(transaction).await,
            Self::Fallback(rpc) => rpc.send_transaction(transaction).await,
        }
    }

    pub async fn send_transaction_with_config(
        &self,
        transaction: &(impl SerializableTransaction + Clone),
        send_config: RpcSendTransactionConfig,
    ) -> Result<Signature, RpcError> {
        match self {
            Self::Single(rpc) => {
                rpc.send_transaction_with_config(transaction, send_config)
                    .await
            }
            Self::Fallback(rpc) => {
                rpc.send_transaction_with_config(transaction, send_config)
                    .await
            }
        }
    }

    pub async fn send_and_confirm_transaction(
        &self,
        transaction: &(impl SerializableTransaction + Clone),
    ) -> Result<Signature, RpcError> {
        match self {
            Self::Single(rpc) => rpc.send_and_confirm_transaction(transaction).await,
            Self::Fallback(rpc) => rpc.send_and_confirm_transaction(transaction).await,
        }
    }

    pub async fn simulate_transaction(
        &self,
        transaction: &(impl SerializableTransaction + Clone),
    ) -> Result<RpcSimulateTransactionResult, RpcError> {
        match self {
            Self::Single(rpc) => rpc.simulate_transaction(transaction).await,
            Self::Fallback(rpc) => rpc.simulate_transaction(transaction).await,
        }
    }

    pub async fn simulate_transaction_with_config(
        &self,
        transaction: &(impl SerializableTransaction + Clone),
        sim_config: RpcSimulateTransactionConfig,
    ) -> Result<RpcSimulateTransactionResult, RpcError> {
        match self {
            Self::Single(rpc) => {
                rpc.simulate_transaction_with_config(transaction, sim_config)
                    .await
            }
            Self::Fallback(rpc) => {
                rpc.simulate_transaction_with_config(transaction, sim_config)
                    .await
            }
        }
    }

    pub async fn get_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        token_account_filter: TokenAccountsFilter,
    ) -> Result<Vec<RpcKeyedAccount>, RpcError> {
        match self {
            Self::Single(rpc) => {
                rpc.get_token_accounts_by_owner(owner, token_account_filter)
                    .await
            }
            Self::Fallback(rpc) => {
                rpc.get_token_accounts_by_owner(owner, token_account_filter)
                    .await
            }
        }
    }

    pub async fn get_signatures_for_address_with_config(
        &self,
        address: &Pubkey,
        config: GetConfirmedSignaturesForAddress2Config,
    ) -> Result<Vec<RpcConfirmedTransactionStatusWithSignature>, RpcError> {
        match self {
            Self::Single(rpc) => {
                rpc.get_signatures_for_address_with_config(address, config)
                    .await
            }
            Self::Fallback(rpc) => {
                rpc.get_signatures_for_address_with_config(address, config)
                    .await
            }
        }
    }

    // -----------------------------------------------------------------------
    // Provider-specific methods
    // -----------------------------------------------------------------------

    /// Get a priority fee estimate.
    ///
    /// - `Single`: calls through the trait accessor (`rpc.priority_fees()?.method()`)
    /// - `Fallback`: delegates to `FallbackRpc` which handles failover internally
    pub async fn get_priority_fee_estimate(
        &self,
        request: PriorityFeeRequest,
    ) -> Result<PriorityFeeEstimate, RpcError> {
        match self {
            Self::Single(rpc) => {
                rpc.priority_fees()?
                    .get_priority_fee_estimate(request)
                    .await
            }
            Self::Fallback(rpc) => rpc.get_priority_fee_estimate(request).await,
        }
    }

    /// Get a DAS asset by ID.
    pub async fn get_asset(&self, id: &str) -> Result<DasAsset, RpcError> {
        match self {
            Self::Single(rpc) => rpc.das()?.get_asset(id).await,
            Self::Fallback(rpc) => rpc.get_asset(id).await,
        }
    }

    /// Get DAS assets owned by an address.
    pub async fn get_assets_by_owner(
        &self,
        request: GetAssetsByOwnerRequest,
    ) -> Result<GetAssetsByOwnerResponse, RpcError> {
        match self {
            Self::Single(rpc) => rpc.das()?.get_assets_by_owner(request).await,
            Self::Fallback(rpc) => rpc.get_assets_by_owner(request).await,
        }
    }

    /// Get an enhanced/parsed transaction by signature.
    pub async fn get_enhanced_transaction(
        &self,
        signature: &str,
    ) -> Result<EnhancedTransaction, RpcError> {
        match self {
            Self::Single(rpc) => {
                rpc.enhanced_transactions()?
                    .get_enhanced_transaction(signature)
                    .await
            }
            Self::Fallback(rpc) => rpc.get_enhanced_transaction(signature).await,
        }
    }

    /// Get enhanced/parsed transactions by signatures (batch).
    pub async fn get_enhanced_transactions(
        &self,
        signatures: &[String],
    ) -> Result<Vec<EnhancedTransaction>, RpcError> {
        match self {
            Self::Single(rpc) => {
                rpc.enhanced_transactions()?
                    .get_enhanced_transactions(signatures)
                    .await
            }
            Self::Fallback(rpc) => rpc.get_enhanced_transactions(signatures).await,
        }
    }
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Builder for constructing a [`SolanaRpcClient`].
///
/// The primary provider is required. If a secondary provider is configured via
/// [`with_secondary`](SolanaRpcBuilder::with_secondary), the resulting client
/// will automatically fail over from primary to secondary on transient errors.
///
/// Configuration overrides (commitment, timeout, retry, rate limit) are applied
/// to **both** primary and secondary providers.
///
/// # Examples
///
/// ```rust,no_run
/// use aignt_solana_rpc::rpc_client::SolanaRpcBuilder;
/// use solana_sdk::commitment_config::CommitmentConfig;
/// use std::time::Duration;
///
/// # fn example() -> anyhow::Result<()> {
/// let rpc = SolanaRpcBuilder::new("alchemy", "alchemy-key")?
///     .with_secondary("helius", "helius-key")?
///     .with_commitment(CommitmentConfig::finalized())
///     .with_timeout(Duration::from_secs(60))
///     .with_rate_limit(50)
///     .build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct SolanaRpcBuilder {
    primary_provider: Provider,
    primary_api_key: String,
    secondary: Option<(Provider, String)>,
    commitment: Option<CommitmentConfig>,
    timeout: Option<Duration>,
    retry: Option<RetryConfig>,
    rate_limit: Option<u32>,
    failure_threshold: Option<u32>,
    primary_cooldown: Option<Duration>,
}

impl SolanaRpcBuilder {
    /// Create a new builder with the required primary provider.
    ///
    /// The `provider` string is case-insensitive: `"helius"`, `"Helius"`, `"HELIUS"` all work.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider name is not recognized (expected: `helius`, `alchemy`).
    pub fn new(provider: &str, api_key: &str) -> anyhow::Result<Self> {
        let primary_provider: Provider =
            provider.parse().map_err(|e: String| anyhow::anyhow!(e))?;
        if primary_provider == Provider::Custom {
            anyhow::bail!(
                "Custom provider is not supported in the builder; use RpcConfig::custom() directly"
            );
        }
        Ok(Self {
            primary_provider,
            primary_api_key: api_key.to_string(),
            secondary: None,
            commitment: None,
            timeout: None,
            retry: None,
            rate_limit: None,
            failure_threshold: None,
            primary_cooldown: None,
        })
    }

    /// Create a builder from environment variables.
    ///
    /// Reads:
    /// - `PRIMARY_PROVIDER` (required) -- e.g. `"helius"`, `"alchemy"`, `"local"`
    /// - `SECONDARY_PROVIDER` (optional) -- e.g. `"alchemy"`, `"helius"`, `"local"`
    ///
    /// API keys are resolved automatically from the provider name:
    /// `PRIMARY_PROVIDER=helius` looks up `HELIUS_API_KEY`,
    /// `SECONDARY_PROVIDER=alchemy` looks up `ALCHEMY_API_KEY`.
    /// `Provider::Local` does not require an API key.
    ///
    /// # Errors
    ///
    /// Returns an error if required env vars are missing or provider names are invalid.
    pub fn from_env() -> anyhow::Result<Self> {
        let primary_name = std::env::var("PRIMARY_PROVIDER")
            .map_err(|_| anyhow::anyhow!("PRIMARY_PROVIDER env var is required"))?;

        let primary_provider: Provider = primary_name
            .parse()
            .map_err(|e: String| anyhow::anyhow!(e))?;
        if primary_provider == Provider::Custom {
            anyhow::bail!(
                "Custom provider is not supported in the builder; use RpcConfig::custom() directly"
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

        if let Ok(secondary_name) = std::env::var("SECONDARY_PROVIDER") {
            if !secondary_name.is_empty() {
                let secondary_provider: Provider = secondary_name
                    .parse()
                    .map_err(|e: String| anyhow::anyhow!(e))?;
                if secondary_provider == Provider::Custom {
                    anyhow::bail!(
                        "Custom provider is not supported as secondary in the builder; use FallbackRpc directly"
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

    /// Add a secondary provider for automatic failover.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider name is not recognized.
    pub fn with_secondary(mut self, provider: &str, api_key: &str) -> anyhow::Result<Self> {
        let secondary_provider: Provider =
            provider.parse().map_err(|e: String| anyhow::anyhow!(e))?;
        if secondary_provider == Provider::Custom {
            anyhow::bail!(
                "Custom provider is not supported as secondary in the builder; use FallbackRpc directly"
            );
        }
        self.secondary = Some((secondary_provider, api_key.to_string()));
        Ok(self)
    }

    /// Override the commitment level (applied to both primary and secondary).
    pub fn with_commitment(mut self, commitment: CommitmentConfig) -> Self {
        self.commitment = Some(commitment);
        self
    }

    /// Override the request timeout (applied to both primary and secondary).
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Override the retry configuration (applied to both primary and secondary).
    pub fn with_retry(mut self, retry: RetryConfig) -> Self {
        self.retry = Some(retry);
        self
    }

    /// Override the rate limit in requests per second (applied to both primary and secondary).
    pub fn with_rate_limit(mut self, rps: u32) -> Self {
        self.rate_limit = Some(rps);
        self
    }

    /// Set the failure threshold for failover (only relevant with a secondary provider).
    ///
    /// After this many consecutive primary failures, all traffic is routed to the secondary.
    pub fn with_failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = Some(threshold);
        self
    }

    /// Set the primary cooldown duration for failover (only relevant with a secondary provider).
    ///
    /// After the primary is marked unhealthy, wait this long before probing it again.
    pub fn with_primary_cooldown(mut self, cooldown: Duration) -> Self {
        self.primary_cooldown = Some(cooldown);
        self
    }

    /// Build the [`SolanaRpcClient`].
    ///
    /// If no secondary provider was configured, returns `SolanaRpcClient::Single`.
    /// If a secondary was configured, returns `SolanaRpcClient::Fallback` with automatic failover.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying `SolanaRpc` construction fails (e.g. invalid rate limit).
    pub fn build(self) -> anyhow::Result<SolanaRpcClient> {
        let primary_config = apply_overrides(
            RpcConfig::from_provider(self.primary_provider, &self.primary_api_key)
                .map_err(|e| anyhow::anyhow!(e))?,
            &self.commitment,
            &self.timeout,
            &self.retry,
            &self.rate_limit,
        );

        match self.secondary {
            None => {
                let rpc = SolanaRpc::new(primary_config)?;
                Ok(SolanaRpcClient::Single(rpc))
            }
            Some((secondary_provider, secondary_api_key)) => {
                let secondary_config = apply_overrides(
                    RpcConfig::from_provider(secondary_provider, &secondary_api_key)
                        .map_err(|e| anyhow::anyhow!(e))?,
                    &self.commitment,
                    &self.timeout,
                    &self.retry,
                    &self.rate_limit,
                );

                let primary = SolanaRpc::new(primary_config)?;
                let secondary = SolanaRpc::new(secondary_config)?;

                let mut fallback = FallbackRpc::new(primary, secondary);

                if let Some(threshold) = self.failure_threshold {
                    fallback = fallback.with_failure_threshold(threshold);
                }
                if let Some(cooldown) = self.primary_cooldown {
                    fallback = fallback.with_primary_cooldown(cooldown);
                }

                Ok(SolanaRpcClient::Fallback(fallback))
            }
        }
    }
}

/// Apply optional overrides to an `RpcConfig`.
fn apply_overrides(
    mut config: RpcConfig,
    commitment: &Option<CommitmentConfig>,
    timeout: &Option<Duration>,
    retry: &Option<RetryConfig>,
    rate_limit: &Option<u32>,
) -> RpcConfig {
    if let Some(commitment) = commitment {
        config = config.with_commitment(*commitment);
    }
    if let Some(timeout) = timeout {
        config = config.with_timeout(*timeout);
    }
    if let Some(retry) = retry {
        config = config.with_retry(retry.clone());
    }
    if let Some(rps) = rate_limit {
        config = config.with_rate_limit(*rps);
    }
    config
}

crate::traits::rpc_operations::impl_rpc_operations!(SolanaRpcClient);

#[cfg(test)]
#[path = "rpc_client_tests.rs"]
mod rpc_client_tests;
