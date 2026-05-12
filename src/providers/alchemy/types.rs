use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Priority fees - Alchemy provides getPriorityFeeEstimate endpoint
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlchemyPriorityFeeParams {
    pub account_keys: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<AlchemyPriorityFeeOptions>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlchemyPriorityFeeOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_all_priority_fee_levels: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlchemyPriorityFeeResult {
    /// The estimated optimal priority fee in microlamports
    /// Alchemy returns this field as "priorityFee" when a specific level is requested
    /// or "priorityFeeEstimate" in some cases
    #[serde(alias = "priorityFee")]
    pub priority_fee_estimate: Option<f64>,
    /// Priority fee levels when includeAllPriorityFeeLevels is true
    #[serde(default)]
    pub priority_fee_levels: Option<AlchemyPriorityFeeLevels>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlchemyPriorityFeeLevels {
    #[serde(default)]
    pub min: f64,
    #[serde(default)]
    pub low: f64,
    #[serde(default)]
    pub medium: f64,
    #[serde(default)]
    pub high: f64,
    #[serde(default)]
    pub very_high: f64,
    #[serde(default)]
    pub unsafe_max: f64,
}

// ---------------------------------------------------------------------------
// DAS - Alchemy supports the standard Metaplex DAS API
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct AlchemyGetAssetParams {
    pub id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlchemyGetAssetsByOwnerParams {
    pub owner_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

// ---------------------------------------------------------------------------
// Enhanced transactions - Alchemy Solana doesn't have a direct equivalent
// to Helius enhanced transactions, so we provide a basic implementation
// using standard getTransaction with parsing
// ---------------------------------------------------------------------------

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct AlchemyTransactionResponse {
    #[serde(default)]
    pub result: Option<serde_json::Value>,
}
