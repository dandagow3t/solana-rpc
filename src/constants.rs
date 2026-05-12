use std::time::Duration;

/// Default timeout for RPC requests (trading-optimized).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(3);

/// Default maximum number of retries for retryable errors (fail fast).
pub const DEFAULT_MAX_RETRIES: u32 = 2;

/// Default base delay between retries (exponential backoff).
pub const DEFAULT_RETRY_BASE_DELAY: Duration = Duration::from_millis(100);

/// Default maximum delay between retries (capped for low latency).
pub const DEFAULT_RETRY_MAX_DELAY: Duration = Duration::from_secs(1);

/// Default maximum total time for all retry attempts.
pub const DEFAULT_RETRY_MAX_TOTAL_TIME: Duration = Duration::from_secs(5);

/// Default rate limit (requests per second - optimized for premium RPC tiers).
pub const DEFAULT_RATE_LIMIT_RPS: u32 = 100;

/// Default failure threshold before switching to secondary provider in FallbackRpc.
pub const DEFAULT_FAILURE_THRESHOLD: u32 = 2;

/// Default cooldown before retrying primary provider after it was marked unhealthy.
pub const DEFAULT_PRIMARY_COOLDOWN: Duration = Duration::from_secs(15);

/// Helius mainnet RPC base URL.
#[cfg(not(feature = "helius-gatekeeper"))]
pub const HELIUS_MAINNET_URL: &str = "https://mainnet.helius-rpc.com";

/// Helius mainnet WebSocket base URL.
#[cfg(not(feature = "helius-gatekeeper"))]
pub const HELIUS_MAINNET_WS_URL: &str = "wss://mainnet.helius-rpc.com";

/// Helius Gatekeeper RPC base URL (faster beta endpoint).
pub const HELIUS_GATEKEEPER_URL: &str = "https://beta.helius-rpc.com";

/// Helius Gatekeeper WebSocket base URL.
pub const HELIUS_GATEKEEPER_WS_URL: &str = "wss://beta.helius-rpc.com";

/// Alchemy Solana mainnet RPC base URL.
pub const ALCHEMY_MAINNET_URL: &str = "https://solana-mainnet.g.alchemy.com/v2";

/// Alchemy Solana mainnet WebSocket base URL.
pub const ALCHEMY_MAINNET_WS_URL: &str = "wss://solana-mainnet.g.alchemy.com/v2";

/// Localhost RPC URL (for local validator).
pub const LOCAL_RPC_URL: &str = "http://127.0.0.1:8899";

/// Localhost WebSocket URL (for local validator).
pub const LOCAL_WS_URL: &str = "ws://127.0.0.1:8900";
