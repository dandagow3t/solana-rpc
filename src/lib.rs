pub mod client;
pub mod config;
pub(crate) mod constants;
pub mod errors;
pub mod fallback;
pub(crate) mod providers;
pub(crate) mod retry;
pub mod rpc_client;
pub mod traits;
pub mod types;

#[cfg(feature = "websocket")]
pub mod ws;

// Re-exports for convenience
pub use client::SolanaRpc;
pub use config::RpcConfig;
pub use errors::RpcError;
pub use fallback::FallbackRpc;
pub use rpc_client::{SolanaRpcBuilder, SolanaRpcClient};
pub use traits::{
    DasProvider, EnhancedTransactionProvider, PriorityFeeProvider, SolanaRpcOperations,
};
pub use types::Provider;

// WebSocket re-exports
#[cfg(feature = "websocket")]
pub use ws::{FallbackWsClient, WsClientBuilder, WsConnection};
