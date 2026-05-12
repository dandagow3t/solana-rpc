use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported RPC provider types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Helius,
    Alchemy,
    Local,
    Custom,
}

impl Provider {
    /// Returns the environment variable name that holds this provider's API key,
    /// or `None` for `Provider::Local` or `Provider::Custom`.
    ///
    /// - `Helius`  → `Some("HELIUS_API_KEY")`
    /// - `Alchemy` → `Some("ALCHEMY_API_KEY")`
    /// - `Local`   → `None`
    /// - `Custom`  → `None`
    pub fn api_key_env_var(&self) -> Option<&'static str> {
        match self {
            Provider::Helius => Some("HELIUS_API_KEY"),
            Provider::Alchemy => Some("ALCHEMY_API_KEY"),
            Provider::Local => None,
            Provider::Custom => None,
        }
    }
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::Helius => write!(f, "Helius"),
            Provider::Alchemy => write!(f, "Alchemy"),
            Provider::Local => write!(f, "Local"),
            Provider::Custom => write!(f, "Custom"),
        }
    }
}

impl std::str::FromStr for Provider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "helius" => Ok(Provider::Helius),
            "alchemy" => Ok(Provider::Alchemy),
            "local" => Ok(Provider::Local),
            "custom" => Ok(Provider::Custom),
            _ => Err(format!(
                "Unknown provider '{s}'. Expected one of: helius, alchemy, local, custom"
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// Priority Fee types
// ---------------------------------------------------------------------------

/// Priority level hint for fee estimation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum PriorityLevel {
    Min,
    Low,
    #[default]
    Medium,
    High,
    VeryHigh,
    UnsafeMax,
}

/// Request for priority fee estimation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PriorityFeeRequest {
    /// Account keys to consider for fee estimation.
    pub account_keys: Vec<String>,
    /// Optional priority level hint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_level: Option<PriorityLevel>,
}

/// Response with priority fee estimates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityFeeEstimate {
    /// Recommended priority fee in micro-lamports per compute unit.
    pub priority_fee: f64,
    /// Per-level estimates if available.
    #[serde(default)]
    pub priority_fee_levels: Option<PriorityFeeLevels>,
}

/// Priority fee estimates broken down by level.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriorityFeeLevels {
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
// DAS (Digital Asset Standard) types
// ---------------------------------------------------------------------------

/// A digital asset returned by DAS API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DasAsset {
    pub id: String,
    #[serde(default)]
    pub interface: Option<String>,
    #[serde(default)]
    pub content: Option<DasContent>,
    #[serde(default)]
    pub authorities: Option<Vec<DasAuthority>>,
    #[serde(default)]
    pub compression: Option<DasCompression>,
    #[serde(default)]
    pub grouping: Option<Vec<DasGrouping>>,
    #[serde(default)]
    pub ownership: Option<DasOwnership>,
    #[serde(default)]
    pub supply: Option<DasSupply>,
    #[serde(default)]
    pub mutable: Option<bool>,
    #[serde(default)]
    pub burnt: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DasContent {
    #[serde(rename = "$schema", default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub json_uri: Option<String>,
    #[serde(default)]
    pub metadata: Option<DasMetadata>,
    #[serde(default)]
    pub files: Option<Vec<DasFile>>,
    #[serde(default)]
    pub links: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DasMetadata {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub attributes: Option<Vec<DasAttribute>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DasAttribute {
    #[serde(default)]
    pub trait_type: Option<String>,
    #[serde(default)]
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DasFile {
    #[serde(default)]
    pub uri: Option<String>,
    #[serde(default)]
    pub cdn_uri: Option<String>,
    #[serde(default)]
    pub mime: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DasAuthority {
    pub address: String,
    #[serde(default)]
    pub scopes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DasCompression {
    #[serde(default)]
    pub eligible: bool,
    #[serde(default)]
    pub compressed: bool,
    #[serde(default)]
    pub data_hash: Option<String>,
    #[serde(default)]
    pub creator_hash: Option<String>,
    #[serde(default)]
    pub asset_hash: Option<String>,
    #[serde(default)]
    pub tree: Option<String>,
    #[serde(default)]
    pub seq: Option<u64>,
    #[serde(default)]
    pub leaf_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DasGrouping {
    #[serde(default)]
    pub group_key: Option<String>,
    #[serde(default)]
    pub group_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DasOwnership {
    #[serde(default)]
    pub frozen: bool,
    #[serde(default)]
    pub delegated: bool,
    #[serde(default)]
    pub delegate: Option<String>,
    #[serde(default)]
    pub ownership_model: Option<String>,
    pub owner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DasSupply {
    #[serde(default)]
    pub print_max_supply: Option<u64>,
    #[serde(default)]
    pub print_current_supply: Option<u64>,
    #[serde(default)]
    pub edition_nonce: Option<u64>,
}

/// Request for get_assets_by_owner.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GetAssetsByOwnerRequest {
    pub owner_address: String,
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<DasSortBy>,
}

fn default_page() -> u32 {
    1
}
fn default_limit() -> u32 {
    1000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DasSortBy {
    pub sort_by: String,
    pub sort_direction: String,
}

/// Response from get_assets_by_owner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetAssetsByOwnerResponse {
    #[serde(default)]
    pub total: u32,
    #[serde(default)]
    pub limit: u32,
    #[serde(default)]
    pub page: u32,
    #[serde(default)]
    pub items: Vec<DasAsset>,
}

// ---------------------------------------------------------------------------
// Enhanced transaction types
// ---------------------------------------------------------------------------

/// An enhanced/parsed transaction from provider APIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnhancedTransaction {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "type", default)]
    pub transaction_type: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub fee: Option<u64>,
    #[serde(default)]
    pub fee_payer: Option<String>,
    pub signature: String,
    #[serde(default)]
    pub slot: Option<u64>,
    #[serde(default)]
    pub timestamp: Option<i64>,
    #[serde(default)]
    pub native_transfers: Option<Vec<NativeTransfer>>,
    #[serde(default)]
    pub token_transfers: Option<Vec<TokenTransfer>>,
    #[serde(default)]
    pub account_data: Option<Vec<AccountData>>,
    #[serde(default)]
    pub events: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTransfer {
    pub from_user_account: String,
    pub to_user_account: String,
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenTransfer {
    #[serde(default)]
    pub from_user_account: Option<String>,
    #[serde(default)]
    pub to_user_account: Option<String>,
    #[serde(default)]
    pub from_token_account: Option<String>,
    #[serde(default)]
    pub to_token_account: Option<String>,
    #[serde(default)]
    pub token_amount: Option<f64>,
    #[serde(default)]
    pub mint: Option<String>,
    #[serde(default)]
    pub token_standard: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountData {
    pub account: String,
    #[serde(default)]
    pub native_balance_change: Option<i64>,
    #[serde(default)]
    pub token_balance_changes: Option<Vec<TokenBalanceChange>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenBalanceChange {
    #[serde(default)]
    pub user_account: Option<String>,
    #[serde(default)]
    pub token_account: Option<String>,
    #[serde(default)]
    pub raw_token_amount: Option<RawTokenAmount>,
    #[serde(default)]
    pub mint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawTokenAmount {
    #[serde(default)]
    pub token_amount: Option<String>,
    #[serde(default)]
    pub decimals: Option<u8>,
}

// ---------------------------------------------------------------------------
// JSON-RPC wrapper types (used by provider impls)
// ---------------------------------------------------------------------------

/// Generic JSON-RPC 2.0 request.
#[derive(Debug, Serialize)]
pub struct JsonRpcRequest<T: Serialize> {
    pub jsonrpc: &'static str,
    pub id: &'static str,
    pub method: &'static str,
    pub params: T,
}

impl<T: Serialize> JsonRpcRequest<T> {
    pub fn new(method: &'static str, params: T) -> Self {
        Self {
            jsonrpc: "2.0",
            id: "1",
            method,
            params,
        }
    }
}

/// Generic JSON-RPC 2.0 response.
#[derive(Debug, Deserialize)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: Option<String>,
    pub id: Option<serde_json::Value>,
    pub result: Option<T>,
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC error object.
#[derive(Debug, Deserialize, Serialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}
