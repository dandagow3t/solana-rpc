pub mod das;
pub mod enhanced;
pub mod priority_fees;
pub mod types;

/// Alchemy provider implementing all provider-specific traits.
#[derive(Clone)]
pub struct AlchemyProvider {
    /// The full RPC URL (including API key).
    pub(crate) rpc_url: String,
    /// HTTP client for JSON-RPC calls.
    pub(crate) http_client: reqwest::Client,
}

impl AlchemyProvider {
    pub fn new(rpc_url: String, http_client: reqwest::Client) -> Self {
        Self {
            rpc_url,
            http_client,
        }
    }
}
