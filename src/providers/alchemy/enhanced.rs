use super::AlchemyProvider;
use crate::errors::RpcError;
use crate::traits::EnhancedTransactionProvider;
use crate::types::{EnhancedTransaction, JsonRpcRequest, JsonRpcResponse};
use async_trait::async_trait;

/// Alchemy doesn't provide a direct enhanced/parsed transaction API like Helius.
/// This implementation uses getTransaction and returns a minimal enhanced format
/// with the fields we can extract from standard RPC.
#[async_trait]
impl EnhancedTransactionProvider for AlchemyProvider {
    async fn get_enhanced_transaction(
        &self,
        signature: &str,
    ) -> Result<EnhancedTransaction, RpcError> {
        let params = serde_json::json!([signature, {"encoding": "jsonParsed", "maxSupportedTransactionVersion": 0}]);
        let rpc_request = JsonRpcRequest::new("getTransaction", params);

        let response = self
            .http_client
            .post(&self.rpc_url)
            .json(&rpc_request)
            .send()
            .await
            .map_err(RpcError::from_reqwest)?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(RpcError::from_http_status(status.as_u16(), body));
        }

        let rpc_response: JsonRpcResponse<serde_json::Value> =
            response.json().await.map_err(RpcError::from_reqwest)?;

        if let Some(err) = rpc_response.error {
            return Err(RpcError::ProviderApiError {
                provider: "Alchemy".to_string(),
                message: err.message,
            });
        }

        let result = rpc_response
            .result
            .ok_or_else(|| RpcError::ProviderApiError {
                provider: "Alchemy".to_string(),
                message: "getTransaction response missing result".to_string(),
            })?;

        // Extract basic fields from the standard getTransaction response
        let fee = result
            .get("meta")
            .and_then(|m| m.get("fee"))
            .and_then(|f| f.as_u64());
        let slot = result.get("slot").and_then(|s| s.as_u64());
        let block_time = result.get("blockTime").and_then(|t| t.as_i64());

        Ok(EnhancedTransaction {
            description: None,
            transaction_type: None,
            source: Some("Alchemy".to_string()),
            fee,
            fee_payer: None,
            signature: signature.to_string(),
            slot,
            timestamp: block_time,
            native_transfers: None,
            token_transfers: None,
            account_data: None,
            events: None,
        })
    }

    async fn get_enhanced_transactions(
        &self,
        signatures: &[String],
    ) -> Result<Vec<EnhancedTransaction>, RpcError> {
        use futures_util::stream::{self, StreamExt, TryStreamExt};

        let futures: Vec<_> = signatures
            .iter()
            .map(|sig| self.get_enhanced_transaction(sig))
            .collect();

        stream::iter(futures).buffered(10).try_collect().await
    }
}

#[cfg(test)]
#[path = "enhanced_tests.rs"]
mod enhanced_tests;
