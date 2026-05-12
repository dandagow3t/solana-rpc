use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Priority fees
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeliusPriorityFeeParams {
    pub account_keys: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<HeliusPriorityFeeOptions>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeliusPriorityFeeOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_all_priority_fee_levels: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeliusPriorityFeeResult {
    pub priority_fee_estimate: Option<f64>,
    #[serde(default)]
    pub priority_fee_levels: Option<HeliusPriorityFeeLevels>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeliusPriorityFeeLevels {
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
// DAS
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct HeliusGetAssetParams {
    pub id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeliusGetAssetsByOwnerParams {
    pub owner_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<serde_json::Value>,
}
