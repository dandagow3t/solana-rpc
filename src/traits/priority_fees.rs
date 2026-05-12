use crate::errors::RpcError;
use crate::types::{PriorityFeeEstimate, PriorityFeeRequest};
use async_trait::async_trait;

/// Provider-specific priority fee estimation.
#[async_trait]
pub trait PriorityFeeProvider: Send + Sync {
    async fn get_priority_fee_estimate(
        &self,
        request: PriorityFeeRequest,
    ) -> Result<PriorityFeeEstimate, RpcError>;
}
