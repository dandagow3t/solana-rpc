use crate::errors::RpcError;
use async_trait::async_trait;
use solana_account_decoder::parse_token::UiTokenAmount;
use solana_client::rpc_config::{RpcProgramAccountsConfig, RpcTransactionConfig};
use solana_client::rpc_response::{RpcTokenAccountBalance, RpcVersionInfo};
use solana_sdk::account::Account;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, TransactionStatus, UiTransactionEncoding,
};

/// Object-safe trait for standard Solana RPC read operations.
///
/// Implemented by [`SolanaRpc`](crate::client::SolanaRpc),
/// [`FallbackRpc`](crate::fallback::FallbackRpc), and
/// [`SolanaRpcClient`](crate::rpc_client::SolanaRpcClient),
/// enabling downstream code to accept `&dyn SolanaRpcOperations`.
///
/// Transaction-sending methods (`send_transaction`, `simulate_transaction`, etc.)
/// are excluded because they use `impl Trait` parameters, which are not object-safe.
#[async_trait]
pub trait SolanaRpcOperations: Send + Sync {
    async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64, RpcError>;
    async fn get_account(&self, pubkey: &Pubkey) -> Result<Account, RpcError>;
    async fn get_multiple_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<Option<Account>>, RpcError>;
    async fn get_latest_blockhash(&self) -> Result<Hash, RpcError>;
    async fn get_slot(&self) -> Result<u64, RpcError>;
    async fn get_version(&self) -> Result<RpcVersionInfo, RpcError>;
    async fn get_signature_statuses(
        &self,
        signatures: &[Signature],
    ) -> Result<Vec<Option<TransactionStatus>>, RpcError>;
    async fn get_transaction(
        &self,
        signature: &Signature,
        encoding: UiTransactionEncoding,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta, RpcError>;
    async fn get_program_accounts(
        &self,
        pubkey: &Pubkey,
    ) -> Result<Vec<(Pubkey, Account)>, RpcError>;
    async fn get_program_accounts_with_config(
        &self,
        pubkey: &Pubkey,
        config: RpcProgramAccountsConfig,
    ) -> Result<Vec<(Pubkey, Account)>, RpcError>;
    async fn get_transaction_with_config(
        &self,
        signature: &Signature,
        config: RpcTransactionConfig,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta, RpcError>;
    async fn get_token_largest_accounts(
        &self,
        mint: &Pubkey,
    ) -> Result<Vec<RpcTokenAccountBalance>, RpcError>;
    async fn get_token_account_balance(
        &self,
        token_account: &Pubkey,
    ) -> Result<UiTokenAmount, RpcError>;
}

/// Implements `SolanaRpcOperations` for a type by delegating each method to
/// the identically-named inherent method on `self`.
macro_rules! impl_rpc_operations {
    ($ty:ty) => {
        #[async_trait::async_trait]
        impl crate::traits::SolanaRpcOperations for $ty {
            async fn get_balance(
                &self,
                pubkey: &solana_sdk::pubkey::Pubkey,
            ) -> Result<u64, crate::errors::RpcError> {
                self.get_balance(pubkey).await
            }
            async fn get_account(
                &self,
                pubkey: &solana_sdk::pubkey::Pubkey,
            ) -> Result<solana_sdk::account::Account, crate::errors::RpcError> {
                self.get_account(pubkey).await
            }
            async fn get_multiple_accounts(
                &self,
                pubkeys: &[solana_sdk::pubkey::Pubkey],
            ) -> Result<Vec<Option<solana_sdk::account::Account>>, crate::errors::RpcError> {
                self.get_multiple_accounts(pubkeys).await
            }
            async fn get_latest_blockhash(
                &self,
            ) -> Result<solana_sdk::hash::Hash, crate::errors::RpcError> {
                self.get_latest_blockhash().await
            }
            async fn get_slot(&self) -> Result<u64, crate::errors::RpcError> {
                self.get_slot().await
            }
            async fn get_version(
                &self,
            ) -> Result<solana_client::rpc_response::RpcVersionInfo, crate::errors::RpcError> {
                self.get_version().await
            }
            async fn get_signature_statuses(
                &self,
                signatures: &[solana_sdk::signature::Signature],
            ) -> Result<
                Vec<Option<solana_transaction_status::TransactionStatus>>,
                crate::errors::RpcError,
            > {
                self.get_signature_statuses(signatures).await
            }
            async fn get_transaction(
                &self,
                signature: &solana_sdk::signature::Signature,
                encoding: solana_transaction_status::UiTransactionEncoding,
            ) -> Result<
                solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta,
                crate::errors::RpcError,
            > {
                self.get_transaction(signature, encoding).await
            }
            async fn get_program_accounts(
                &self,
                pubkey: &solana_sdk::pubkey::Pubkey,
            ) -> Result<
                Vec<(solana_sdk::pubkey::Pubkey, solana_sdk::account::Account)>,
                crate::errors::RpcError,
            > {
                self.get_program_accounts(pubkey).await
            }
            async fn get_program_accounts_with_config(
                &self,
                pubkey: &solana_sdk::pubkey::Pubkey,
                config: solana_client::rpc_config::RpcProgramAccountsConfig,
            ) -> Result<
                Vec<(solana_sdk::pubkey::Pubkey, solana_sdk::account::Account)>,
                crate::errors::RpcError,
            > {
                self.get_program_accounts_with_config(pubkey, config).await
            }
            async fn get_transaction_with_config(
                &self,
                signature: &solana_sdk::signature::Signature,
                config: solana_client::rpc_config::RpcTransactionConfig,
            ) -> Result<
                solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta,
                crate::errors::RpcError,
            > {
                self.get_transaction_with_config(signature, config).await
            }
            async fn get_token_largest_accounts(
                &self,
                mint: &solana_sdk::pubkey::Pubkey,
            ) -> Result<
                Vec<solana_client::rpc_response::RpcTokenAccountBalance>,
                crate::errors::RpcError,
            > {
                self.get_token_largest_accounts(mint).await
            }
            async fn get_token_account_balance(
                &self,
                token_account: &solana_sdk::pubkey::Pubkey,
            ) -> Result<solana_account_decoder::parse_token::UiTokenAmount, crate::errors::RpcError>
            {
                self.get_token_account_balance(token_account).await
            }
        }
    };
}

pub(crate) use impl_rpc_operations;
