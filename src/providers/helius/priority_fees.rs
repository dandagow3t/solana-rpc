use super::HeliusProvider;
use super::types::{HeliusPriorityFeeOptions, HeliusPriorityFeeParams};
use crate::errors::RpcError;
use crate::traits::PriorityFeeProvider;
use crate::types::{
    JsonRpcRequest, JsonRpcResponse, PriorityFeeEstimate, PriorityFeeLevels, PriorityFeeRequest,
    PriorityLevel,
};
use async_trait::async_trait;

#[async_trait]
impl PriorityFeeProvider for HeliusProvider {
    async fn get_priority_fee_estimate(
        &self,
        request: PriorityFeeRequest,
    ) -> Result<PriorityFeeEstimate, RpcError> {
        let priority_level_str = request.priority_level.as_ref().map(|l| match l {
            PriorityLevel::Min => "Min",
            PriorityLevel::Low => "Low",
            PriorityLevel::Medium => "Medium",
            PriorityLevel::High => "High",
            PriorityLevel::VeryHigh => "VeryHigh",
            PriorityLevel::UnsafeMax => "UnsafeMax",
        });

        let include_all_levels = request.priority_level.is_none();

        let params = HeliusPriorityFeeParams {
            account_keys: request.account_keys,
            options: Some(HeliusPriorityFeeOptions {
                priority_level: priority_level_str.map(|s| s.to_string()),
                include_all_priority_fee_levels: Some(include_all_levels),
            }),
        };

        let rpc_request = JsonRpcRequest::new("getPriorityFeeEstimate", [params]);

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

        let rpc_response: JsonRpcResponse<super::types::HeliusPriorityFeeResult> =
            response.json().await.map_err(RpcError::from_reqwest)?;

        if let Some(err) = rpc_response.error {
            return Err(RpcError::ProviderApiError {
                provider: "Helius".to_string(),
                message: err.message,
            });
        }

        let result = rpc_response
            .result
            .ok_or_else(|| RpcError::ProviderApiError {
                provider: "Helius".to_string(),
                message: "priority fee response missing result".to_string(),
            })?;

        // When includeAllPriorityFeeLevels is true, Helius returns only priorityFeeLevels
        // and not a single estimate. We use the medium level as the recommended estimate.
        // When a specific priority level is requested, Helius returns a single priorityFeeEstimate value.
        let (priority_fee, priority_fee_levels) = if let Some(levels) = result.priority_fee_levels {
            let fee_levels = PriorityFeeLevels {
                min: levels.min,
                low: levels.low,
                medium: levels.medium,
                high: levels.high,
                very_high: levels.very_high,
                unsafe_max: levels.unsafe_max,
            };
            // Use medium as the default recommendation
            (levels.medium, Some(fee_levels))
        } else {
            // Single estimate was returned (when priorityLevel was specified)
            (result.priority_fee_estimate.unwrap_or(0.0), None)
        };

        Ok(PriorityFeeEstimate {
            priority_fee,
            priority_fee_levels,
        })
    }
}

#[cfg(test)]
#[path = "priority_fees_tests.rs"]
mod priority_fees_tests;
