#![allow(dead_code)]

use aignt_solana_rpc::client::SolanaRpc;
use aignt_solana_rpc::config::RpcConfig;
use httpmock::prelude::*;

// ---------------------------------------------------------------------------
// API key helpers (load from .env, return None if missing/empty)
// ---------------------------------------------------------------------------

pub fn helius_api_key() -> Option<String> {
    dotenvy::dotenv().ok();
    std::env::var("HELIUS_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
}

pub fn alchemy_api_key() -> Option<String> {
    dotenvy::dotenv().ok();
    std::env::var("ALCHEMY_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
}

// ---------------------------------------------------------------------------
// Mock RPC helpers
// ---------------------------------------------------------------------------

/// Spin up a mock HTTP server that returns a valid `getBalance` JSON-RPC response.
pub fn mock_balance_server() -> MockServer {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"jsonrpc":"2.0","result":{"context":{"slot":1},"value":999},"id":1}"#);
    });
    server
}

/// Fire `total` get_balance requests as fast as possible and return elapsed time.
pub async fn fire_requests(rpc: &SolanaRpc, total: usize) -> std::time::Duration {
    let pubkey = solana_sdk::pubkey::Pubkey::default();
    let start = std::time::Instant::now();
    for _ in 0..total {
        let _ = rpc.get_balance(&pubkey).await;
    }
    start.elapsed()
}

// ---------------------------------------------------------------------------
// Convenience builders
// ---------------------------------------------------------------------------

/// Build a `SolanaRpc` client pointing at Helius, or return `None` if the key is missing.
pub fn helius_rpc() -> Option<SolanaRpc> {
    let key = helius_api_key()?;
    Some(SolanaRpc::new(RpcConfig::helius(&key)).unwrap())
}

/// Build a `SolanaRpc` client pointing at Alchemy, or return `None` if the key is missing.
pub fn alchemy_rpc() -> Option<SolanaRpc> {
    let key = alchemy_api_key()?;
    Some(SolanaRpc::new(RpcConfig::alchemy(&key)).unwrap())
}
