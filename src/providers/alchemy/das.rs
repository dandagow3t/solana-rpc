use super::AlchemyProvider;
use super::types::{AlchemyGetAssetParams, AlchemyGetAssetsByOwnerParams};
use crate::errors::RpcError;
use crate::traits::DasProvider;
use crate::types::{
    DasAsset, GetAssetsByOwnerRequest, GetAssetsByOwnerResponse, JsonRpcRequest, JsonRpcResponse,
};
use async_trait::async_trait;

#[async_trait]
impl DasProvider for AlchemyProvider {
    async fn get_asset(&self, id: &str) -> Result<DasAsset, RpcError> {
        let params = AlchemyGetAssetParams { id: id.to_string() };
        let rpc_request = JsonRpcRequest::new("getAsset", params);

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

        let rpc_response: JsonRpcResponse<DasAsset> =
            response.json().await.map_err(RpcError::from_reqwest)?;

        if let Some(err) = rpc_response.error {
            return Err(RpcError::ProviderApiError {
                provider: "Alchemy".to_string(),
                message: err.message,
            });
        }

        rpc_response
            .result
            .ok_or_else(|| RpcError::ProviderApiError {
                provider: "Alchemy".to_string(),
                message: "getAsset response missing result".to_string(),
            })
    }

    async fn get_assets_by_owner(
        &self,
        request: GetAssetsByOwnerRequest,
    ) -> Result<GetAssetsByOwnerResponse, RpcError> {
        let params = AlchemyGetAssetsByOwnerParams {
            owner_address: request.owner_address,
            page: Some(request.page),
            limit: Some(request.limit),
            before: request.before,
            after: request.after,
        };
        let rpc_request = JsonRpcRequest::new("getAssetsByOwner", params);

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

        let rpc_response: JsonRpcResponse<GetAssetsByOwnerResponse> =
            response.json().await.map_err(RpcError::from_reqwest)?;

        if let Some(err) = rpc_response.error {
            return Err(RpcError::ProviderApiError {
                provider: "Alchemy".to_string(),
                message: err.message,
            });
        }

        rpc_response
            .result
            .ok_or_else(|| RpcError::ProviderApiError {
                provider: "Alchemy".to_string(),
                message: "getAssetsByOwner response missing result".to_string(),
            })
    }
}

#[cfg(test)]
#[path = "das_tests.rs"]
mod das_tests;
