use crate::errors::RpcError;
use crate::types::EnhancedTransaction;
use async_trait::async_trait;

/// Provider-specific enhanced/parsed transaction API.
#[async_trait]
pub trait EnhancedTransactionProvider: Send + Sync {
    async fn get_enhanced_transaction(
        &self,
        signature: &str,
    ) -> Result<EnhancedTransaction, RpcError>;

    async fn get_enhanced_transactions(
        &self,
        signatures: &[String],
    ) -> Result<Vec<EnhancedTransaction>, RpcError>;
}
