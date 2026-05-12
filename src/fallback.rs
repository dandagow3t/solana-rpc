use crate::client::SolanaRpc;
use crate::config::RpcConfig;
use crate::constants::{DEFAULT_FAILURE_THRESHOLD, DEFAULT_PRIMARY_COOLDOWN};
use crate::errors::RpcError;
use crate::types::{
    DasAsset, EnhancedTransaction, GetAssetsByOwnerRequest, GetAssetsByOwnerResponse,
    PriorityFeeEstimate, PriorityFeeRequest,
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
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::Duration;
use tracing::{info, warn};

/// A failover RPC client wrapping a primary and secondary `SolanaRpc`.
///
/// Routes requests to the primary by default, automatically failing over to
/// the secondary on retryable errors. After a configurable number of consecutive
/// primary failures, all traffic is routed to the secondary until a cooldown
/// period elapses and the primary is probed again.
#[derive(Clone)]
pub struct FallbackRpc {
    primary: SolanaRpc,
    secondary: SolanaRpc,
    primary_failures: Arc<AtomicU32>,
    failure_threshold: u32,
    primary_cooldown: Duration,
    /// Stores unhealthy timestamp as milliseconds since UNIX_EPOCH, or 0 if healthy
    primary_unhealthy_since_ms: Arc<AtomicU64>,
    /// Ensures only one task probes the primary after cooldown
    probe_lock: Arc<AtomicBool>,
}

/// Generates fallback methods that try primary, then secondary on failover-eligible
/// errors. Uses `Clone::clone(&$param)` for the primary call so that reference params
/// produce a reference copy while owned params produce a deep clone. Originals are
/// preserved for the secondary attempt.
macro_rules! fallback_rpc {
    ($($(#[$meta:meta])* fn $method:ident($($param:ident : $ty:ty),*) -> $ret:ty;)*) => {
        $(
            $(#[$meta])*
            pub async fn $method(&self, $($param: $ty),*) -> $ret {
                if self.should_use_primary() {
                    match self.primary.$method($(Clone::clone(&$param)),*).await {
                        Ok(result) => {
                            self.record_primary_success();
                            return Ok(result);
                        }
                        Err(err) => {
                            if !err.should_failover() { return Err(err); }
                            warn!(method = stringify!($method), error = %err, "Primary failed, trying secondary");
                            self.record_primary_failure();
                        }
                    }
                }
                self.secondary.$method($($param),*).await
            }
        )*
    };
}

impl FallbackRpc {
    pub fn new(primary: SolanaRpc, secondary: SolanaRpc) -> Self {
        Self {
            primary,
            secondary,
            primary_failures: Arc::new(AtomicU32::new(0)),
            failure_threshold: DEFAULT_FAILURE_THRESHOLD,
            primary_cooldown: DEFAULT_PRIMARY_COOLDOWN,
            primary_unhealthy_since_ms: Arc::new(AtomicU64::new(0)),
            probe_lock: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = threshold;
        self
    }

    pub fn with_primary_cooldown(mut self, cooldown: Duration) -> Self {
        self.primary_cooldown = cooldown;
        self
    }

    /// Get reference to the primary client's underlying RPC client.
    pub fn primary_inner(&self) -> &RpcClient {
        self.primary.inner()
    }

    /// Get reference to the secondary client's underlying RPC client.
    pub fn secondary_inner(&self) -> &RpcClient {
        self.secondary.inner()
    }

    /// Get the primary client config.
    pub fn primary_config(&self) -> &RpcConfig {
        self.primary.config()
    }

    /// Get the secondary client config.
    pub fn secondary_config(&self) -> &RpcConfig {
        self.secondary.config()
    }

    /// Get reference to the primary `SolanaRpc` client.
    pub fn primary(&self) -> &SolanaRpc {
        &self.primary
    }

    /// Get reference to the secondary `SolanaRpc` client.
    pub fn secondary(&self) -> &SolanaRpc {
        &self.secondary
    }

    /// Get WebSocket client from a specific provider in the fallback setup.
    ///
    /// Returns the WebSocket client from whichever RPC client (primary or secondary)
    /// matches the specified provider.
    ///
    /// # Arguments
    /// * `provider` - The provider to get the WebSocket client for
    ///
    /// # Example
    /// ```rust,no_run
    /// # use aignt_solana_rpc::{SolanaRpc, RpcConfig, Provider};
    /// # use aignt_solana_rpc::fallback::FallbackRpc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Setup: Alchemy (primary) + Helius (secondary) for HTTP
    /// let primary = SolanaRpc::new(RpcConfig::alchemy("alchemy-key"))?;
    /// let secondary = SolanaRpc::new(RpcConfig::helius("helius-key"))?;
    /// let fallback = FallbackRpc::new(primary, secondary);
    ///
    /// // Use Helius WebSocket (because Alchemy doesn't support it well)
    /// let ws = fallback.ws_by_provider(Provider::Helius).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "websocket")]
    pub async fn ws_by_provider(
        &self,
        provider: crate::types::Provider,
    ) -> Result<&crate::ws::WsClient, RpcError> {
        if self.primary.provider() == provider {
            self.primary.ws().await
        } else if self.secondary.provider() == provider {
            self.secondary.ws().await
        } else {
            Err(RpcError::WebSocketError {
                message: format!(
                    "Provider {:?} not configured. Available: {:?} (primary), {:?} (secondary)",
                    provider,
                    self.primary.provider(),
                    self.secondary.provider()
                ),
            })
        }
    }

    /// Check if the primary is currently marked unhealthy.
    pub fn is_primary_unhealthy(&self) -> bool {
        self.primary_unhealthy_since_ms.load(Ordering::Relaxed) != 0
    }

    /// Determine whether to use primary (single probe after cooldown if unhealthy).
    fn should_use_primary(&self) -> bool {
        let unhealthy_since_ms = self.primary_unhealthy_since_ms.load(Ordering::Acquire);

        if unhealthy_since_ms == 0 {
            // Primary is healthy
            return true;
        }

        // Primary is unhealthy, check if cooldown elapsed
        let now = std::time::SystemTime::now();
        let now_ms = now
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis() as u64;
        let elapsed_ms = now_ms.saturating_sub(unhealthy_since_ms);

        if Duration::from_millis(elapsed_ms) >= self.primary_cooldown {
            // Cooldown elapsed - try to acquire probe lock
            if self
                .probe_lock
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                // This task won the race to probe
                info!("Primary cooldown elapsed, probing primary (single probe)");
                true
            } else {
                // Another task is already probing, use secondary
                false
            }
        } else {
            // Still in cooldown
            false
        }
    }

    /// Record a primary success: reset failure counter and mark healthy.
    fn record_primary_success(&self) {
        let prev = self.primary_failures.swap(0, Ordering::SeqCst);
        if prev > 0 {
            info!(
                previous_failures = prev,
                "Primary recovered, resetting failure counter"
            );
        }

        // Mark healthy and release probe lock if held
        self.primary_unhealthy_since_ms.store(0, Ordering::Release);
        self.probe_lock.store(false, Ordering::Release);
    }

    /// Record a primary failure: increment counter, potentially mark unhealthy.
    fn record_primary_failure(&self) {
        let failures = self.primary_failures.fetch_add(1, Ordering::SeqCst) + 1;

        // Release probe lock if we were probing
        if self
            .probe_lock
            .compare_exchange(true, false, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
        {
            // We were probing and failed - extend cooldown
            let now = std::time::SystemTime::now();
            let now_ms = now
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_millis() as u64;

            warn!(failures, "Primary probe failed, extending cooldown");

            self.primary_unhealthy_since_ms
                .store(now_ms, Ordering::Release);
            return;
        }

        // Not probing - check if we hit threshold
        if failures >= self.failure_threshold {
            let current = self.primary_unhealthy_since_ms.load(Ordering::Relaxed);
            if current == 0 {
                let now = std::time::SystemTime::now();
                let now_ms = now
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or(Duration::ZERO)
                    .as_millis() as u64;

                warn!(
                    failures,
                    threshold = self.failure_threshold,
                    "Primary exceeded failure threshold, routing to secondary"
                );

                self.primary_unhealthy_since_ms
                    .store(now_ms, Ordering::Release);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Standard RPC methods (generated via macro)
    // -----------------------------------------------------------------------

    fallback_rpc! {
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

    // -----------------------------------------------------------------------
    // Transaction methods (kept manual — `impl Trait` params can't be
    // captured by macro_rules)
    // -----------------------------------------------------------------------

    pub async fn send_transaction(
        &self,
        transaction: &(impl SerializableTransaction + Clone),
    ) -> Result<Signature, RpcError> {
        if self.should_use_primary() {
            match self.primary.send_transaction(transaction).await {
                Ok(result) => {
                    self.record_primary_success();
                    return Ok(result);
                }
                Err(err) => {
                    if !err.should_failover() {
                        return Err(err);
                    }
                    warn!(method = "send_transaction", error = %err, "Primary failed, trying secondary");
                    self.record_primary_failure();
                }
            }
        }
        self.secondary.send_transaction(transaction).await
    }

    pub async fn send_transaction_with_config(
        &self,
        transaction: &(impl SerializableTransaction + Clone),
        send_config: RpcSendTransactionConfig,
    ) -> Result<Signature, RpcError> {
        if self.should_use_primary() {
            match self
                .primary
                .send_transaction_with_config(transaction, send_config)
                .await
            {
                Ok(result) => {
                    self.record_primary_success();
                    return Ok(result);
                }
                Err(err) => {
                    if !err.should_failover() {
                        return Err(err);
                    }
                    warn!(method = "send_transaction_with_config", error = %err, "Primary failed, trying secondary");
                    self.record_primary_failure();
                }
            }
        }
        self.secondary
            .send_transaction_with_config(transaction, send_config)
            .await
    }

    pub async fn send_and_confirm_transaction(
        &self,
        transaction: &(impl SerializableTransaction + Clone),
    ) -> Result<Signature, RpcError> {
        if self.should_use_primary() {
            match self.primary.send_and_confirm_transaction(transaction).await {
                Ok(result) => {
                    self.record_primary_success();
                    return Ok(result);
                }
                Err(err) => {
                    if !err.should_failover() {
                        return Err(err);
                    }
                    warn!(method = "send_and_confirm_transaction", error = %err, "Primary failed, trying secondary");
                    self.record_primary_failure();
                }
            }
        }
        self.secondary
            .send_and_confirm_transaction(transaction)
            .await
    }

    pub async fn simulate_transaction(
        &self,
        transaction: &(impl SerializableTransaction + Clone),
    ) -> Result<RpcSimulateTransactionResult, RpcError> {
        if self.should_use_primary() {
            match self.primary.simulate_transaction(transaction).await {
                Ok(result) => {
                    self.record_primary_success();
                    return Ok(result);
                }
                Err(err) => {
                    if !err.should_failover() {
                        return Err(err);
                    }
                    warn!(method = "simulate_transaction", error = %err, "Primary failed, trying secondary");
                    self.record_primary_failure();
                }
            }
        }
        self.secondary.simulate_transaction(transaction).await
    }

    pub async fn simulate_transaction_with_config(
        &self,
        transaction: &(impl SerializableTransaction + Clone),
        sim_config: RpcSimulateTransactionConfig,
    ) -> Result<RpcSimulateTransactionResult, RpcError> {
        if self.should_use_primary() {
            match self
                .primary
                .simulate_transaction_with_config(transaction, sim_config.clone())
                .await
            {
                Ok(result) => {
                    self.record_primary_success();
                    return Ok(result);
                }
                Err(err) => {
                    if !err.should_failover() {
                        return Err(err);
                    }
                    warn!(method = "simulate_transaction_with_config", error = %err, "Primary failed, trying secondary");
                    self.record_primary_failure();
                }
            }
        }
        self.secondary
            .simulate_transaction_with_config(transaction, sim_config)
            .await
    }

    pub async fn get_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        token_account_filter: TokenAccountsFilter,
    ) -> Result<Vec<RpcKeyedAccount>, RpcError> {
        // Note: TokenAccountsFilter doesn't implement Clone, so we can't attempt
        // fallback after a failed primary call. We decide upfront which provider to use.
        if self.should_use_primary() {
            let result = self
                .primary
                .get_token_accounts_by_owner(owner, token_account_filter)
                .await;
            match &result {
                Ok(_) => self.record_primary_success(),
                Err(err) if err.should_failover() => {
                    warn!(method = "get_token_accounts_by_owner", error = %err, "Primary failed, but cannot retry on secondary (TokenAccountsFilter consumed)");
                    self.record_primary_failure();
                }
                Err(_) => {} // Non-failover error, don't record as failure
            }
            result
        } else {
            // Primary is unhealthy, use secondary
            self.secondary
                .get_token_accounts_by_owner(owner, token_account_filter)
                .await
        }
    }

    pub async fn get_signatures_for_address_with_config(
        &self,
        address: &Pubkey,
        config: GetConfirmedSignaturesForAddress2Config,
    ) -> Result<Vec<RpcConfirmedTransactionStatusWithSignature>, RpcError> {
        // Note: GetConfirmedSignaturesForAddress2Config doesn't implement Clone, so we can't attempt
        // fallback after a failed primary call. We decide upfront which provider to use.
        if self.should_use_primary() {
            let result = self
                .primary
                .get_signatures_for_address_with_config(address, config)
                .await;
            match &result {
                Ok(_) => self.record_primary_success(),
                Err(err) if err.should_failover() => {
                    warn!(method = "get_signatures_for_address_with_config", error = %err, "Primary failed, but cannot retry on secondary (GetConfirmedSignaturesForAddress2Config consumed)");
                    self.record_primary_failure();
                }
                Err(_) => {} // Non-failover error, don't record as failure
            }
            result
        } else {
            // Primary is unhealthy, use secondary
            self.secondary
                .get_signatures_for_address_with_config(address, config)
                .await
        }
    }

    // -----------------------------------------------------------------------
    // Provider-specific methods with fallback
    // -----------------------------------------------------------------------

    pub async fn get_priority_fee_estimate(
        &self,
        request: PriorityFeeRequest,
    ) -> Result<PriorityFeeEstimate, RpcError> {
        if self.should_use_primary() {
            match self.primary.priority_fees() {
                Ok(provider) => match provider.get_priority_fee_estimate(request.clone()).await {
                    Ok(result) => {
                        self.record_primary_success();
                        return Ok(result);
                    }
                    Err(err) => {
                        if !err.should_failover() {
                            return Err(err);
                        }
                        warn!(method = "get_priority_fee_estimate", error = %err, "Primary provider failed, trying secondary");
                        self.record_primary_failure();
                    }
                },
                Err(_) => {
                    self.record_primary_failure();
                }
            }
        }
        self.secondary
            .priority_fees()?
            .get_priority_fee_estimate(request)
            .await
    }

    pub async fn get_asset(&self, id: &str) -> Result<DasAsset, RpcError> {
        if self.should_use_primary() {
            match self.primary.das() {
                Ok(provider) => match provider.get_asset(id).await {
                    Ok(result) => {
                        self.record_primary_success();
                        return Ok(result);
                    }
                    Err(err) => {
                        if !err.should_failover() {
                            return Err(err);
                        }
                        warn!(method = "get_asset", error = %err, "Primary provider failed, trying secondary");
                        self.record_primary_failure();
                    }
                },
                Err(_) => {
                    self.record_primary_failure();
                }
            }
        }
        self.secondary.das()?.get_asset(id).await
    }

    pub async fn get_assets_by_owner(
        &self,
        request: GetAssetsByOwnerRequest,
    ) -> Result<GetAssetsByOwnerResponse, RpcError> {
        if self.should_use_primary() {
            match self.primary.das() {
                Ok(provider) => match provider.get_assets_by_owner(request.clone()).await {
                    Ok(result) => {
                        self.record_primary_success();
                        return Ok(result);
                    }
                    Err(err) => {
                        if !err.should_failover() {
                            return Err(err);
                        }
                        warn!(method = "get_assets_by_owner", error = %err, "Primary provider failed, trying secondary");
                        self.record_primary_failure();
                    }
                },
                Err(_) => {
                    self.record_primary_failure();
                }
            }
        }
        self.secondary.das()?.get_assets_by_owner(request).await
    }

    pub async fn get_enhanced_transaction(
        &self,
        signature: &str,
    ) -> Result<EnhancedTransaction, RpcError> {
        if self.should_use_primary() {
            match self.primary.enhanced_transactions() {
                Ok(provider) => match provider.get_enhanced_transaction(signature).await {
                    Ok(result) => {
                        self.record_primary_success();
                        return Ok(result);
                    }
                    Err(err) => {
                        if !err.should_failover() {
                            return Err(err);
                        }
                        warn!(method = "get_enhanced_transaction", error = %err, "Primary provider failed, trying secondary");
                        self.record_primary_failure();
                    }
                },
                Err(_) => {
                    self.record_primary_failure();
                }
            }
        }
        self.secondary
            .enhanced_transactions()?
            .get_enhanced_transaction(signature)
            .await
    }

    pub async fn get_enhanced_transactions(
        &self,
        signatures: &[String],
    ) -> Result<Vec<EnhancedTransaction>, RpcError> {
        if self.should_use_primary() {
            match self.primary.enhanced_transactions() {
                Ok(provider) => match provider.get_enhanced_transactions(signatures).await {
                    Ok(result) => {
                        self.record_primary_success();
                        return Ok(result);
                    }
                    Err(err) => {
                        if !err.should_failover() {
                            return Err(err);
                        }
                        warn!(method = "get_enhanced_transactions", error = %err, "Primary provider failed, trying secondary");
                        self.record_primary_failure();
                    }
                },
                Err(_) => {
                    self.record_primary_failure();
                }
            }
        }
        self.secondary
            .enhanced_transactions()?
            .get_enhanced_transactions(signatures)
            .await
    }
}

crate::traits::rpc_operations::impl_rpc_operations!(FallbackRpc);

#[cfg(test)]
#[path = "fallback_tests.rs"]
mod fallback_tests;
