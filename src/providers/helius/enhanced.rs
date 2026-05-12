use super::HeliusProvider;
use crate::errors::RpcError;
use crate::traits::EnhancedTransactionProvider;
use crate::types::EnhancedTransaction;
use async_trait::async_trait;

/// Helius enhanced transaction API endpoint path.
const ENHANCED_TX_PATH: &str = "/v0/transactions";

impl HeliusProvider {
    /// Build the enhanced transaction API URL.
    /// Helius enhanced transaction API is a REST endpoint, not JSON-RPC.
    /// URL format: `https://api.helius.xyz/v0/transactions?api-key={key}`
    /// But since we use the full RPC URL, we need to extract the API key and build
    /// the correct REST URL.
    fn enhanced_api_url(&self) -> String {
        // The RPC URL contains the API key: https://mainnet.helius-rpc.com/?api-key=XXX
        // The enhanced API base is: https://api.helius.xyz
        // Extract the api-key from the RPC URL
        if let Some(key) = self.rpc_url.split("api-key=").nth(1) {
            let key = key.split('&').next().unwrap_or(key);
            format!("https://api.helius.xyz{ENHANCED_TX_PATH}?api-key={key}")
        } else {
            // Fallback: assume rpc_url can be used directly (for testing with mock servers)
            format!("{}{ENHANCED_TX_PATH}", self.rpc_url)
        }
    }
}

#[async_trait]
impl EnhancedTransactionProvider for HeliusProvider {
    async fn get_enhanced_transaction(
        &self,
        signature: &str,
    ) -> Result<EnhancedTransaction, RpcError> {
        let transactions = self
            .get_enhanced_transactions(&[signature.to_string()])
            .await?;
        transactions
            .into_iter()
            .next()
            .ok_or_else(|| RpcError::ProviderApiError {
                provider: "Helius".to_string(),
                message: "No enhanced transaction returned for signature".to_string(),
            })
    }

    async fn get_enhanced_transactions(
        &self,
        signatures: &[String],
    ) -> Result<Vec<EnhancedTransaction>, RpcError> {
        let url = self.enhanced_api_url();

        let response = self
            .http_client
            .post(&url)
            .json(&serde_json::json!({ "transactions": signatures }))
            .send()
            .await
            .map_err(RpcError::from_reqwest)?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(RpcError::from_http_status(status.as_u16(), body));
        }

        let transactions: Vec<EnhancedTransaction> =
            response.json().await.map_err(RpcError::from_reqwest)?;

        Ok(transactions)
    }
}

#[cfg(test)]
#[path = "enhanced_tests.rs"]
mod enhanced_tests;
