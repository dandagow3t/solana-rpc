use crate::errors::RpcError;
use crate::types::{DasAsset, GetAssetsByOwnerRequest, GetAssetsByOwnerResponse};
use async_trait::async_trait;

/// Provider-specific Digital Asset Standard (DAS) API.
#[async_trait]
pub trait DasProvider: Send + Sync {
    async fn get_asset(&self, id: &str) -> Result<DasAsset, RpcError>;

    async fn get_assets_by_owner(
        &self,
        request: GetAssetsByOwnerRequest,
    ) -> Result<GetAssetsByOwnerResponse, RpcError>;
}
