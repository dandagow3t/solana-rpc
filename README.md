# aignt-solana-rpc

A unified Solana RPC client library with provider abstraction, automatic failover, retry with exponential backoff, rate limiting, and WebSocket subscriptions.

Instead of scattering raw `reqwest` calls and hardcoded Helius/Alchemy URLs across every service, this crate gives you a single `SolanaRpcClient` that wraps provider-specific APIs (priority fees, DAS, enhanced transactions) behind trait-based abstractions. Use `SolanaRpcBuilder` to construct a client with one or two providers -- failover is handled automatically when a secondary is configured.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Feature Flags](#feature-flags)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
  - [SolanaRpcBuilder (Recommended)](#solanaRpcBuilder-recommended)
  - [Provider Configuration](#provider-configuration)
  - [Retry Configuration](#retry-configuration)
  - [Rate Limiting](#rate-limiting)
  - [Commitment Level](#commitment-level)
  - [Timeout](#timeout)
- [Standard RPC Methods](#standard-rpc-methods)
- [Provider-Specific APIs](#provider-specific-apis)
  - [Priority Fees](#priority-fees)
  - [DAS (Digital Asset Standard)](#das-digital-asset-standard)
  - [Enhanced Transactions](#enhanced-transactions)
- [Failover with SolanaRpcBuilder](#failover-with-solanaRpcBuilder)
  - [How Failover Works](#how-failover-works)
  - [Configuring Failover](#configuring-failover)
  - [Config-Driven Provider Selection](#config-driven-provider-selection)
- [Advanced: Manual Construction](#advanced-manual-construction)
- [WebSocket Subscriptions](#websocket-subscriptions)
- [Error Handling](#error-handling)
- [Escape Hatch](#escape-hatch)
- [Building](#building)
- [Testing](#testing)
- [Environment Variables](#environment-variables)
- [Architecture](#architecture)

## Features

- **Provider abstraction** -- Helius and Alchemy behind unified traits; swap providers without changing application code.
- **Automatic failover** -- `FallbackRpc` routes to a secondary provider on transient errors, with health tracking and cooldown-based recovery.
- **Retry with exponential backoff** -- configurable max retries, base delay, and max delay. Only retryable errors are retried.
- **Rate limiting** -- token-bucket rate limiter (via `governor`) applied to every RPC call.
- **Provider-specific APIs** -- priority fee estimation, DAS (Digital Asset Standard), and enhanced/parsed transactions exposed through traits.
- **WebSocket subscriptions** -- `logsSubscribe`, `accountSubscribe`, `signatureSubscribe` with auto-reconnect, ping/pong keepalive, and automatic resubscription on reconnect.
- **Feature flags** -- compile only the providers you need. WebSocket support is opt-in.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
# Default: Helius provider only
aignt-solana-rpc = "0.1"

# Or pick specific providers
aignt-solana-rpc = { version = "0.1", features = ["all-providers"] }

# With WebSocket support
aignt-solana-rpc = { version = "0.1", features = ["all-providers", "websocket"] }
```

## Feature Flags

| Flag | Description | Default |
|------|-------------|---------|
| `helius` | Enables the Helius provider (priority fees, DAS, enhanced transactions) | Yes |
| `helius-gatekeeper` | Use Helius Gatekeeper (faster beta endpoint) for all Helius configurations | No |
| `alchemy` | Enables the Alchemy provider (priority fees, DAS, enhanced transactions) | No |
| `all-providers` | Enables both `helius` and `alchemy` | No |
| `websocket` | Enables WebSocket client with auto-reconnect subscriptions | No |

### Feature Combinations Reference

| Features | Result |
|----------|--------|
| `["all-providers"]` | Helius (standard) + Alchemy |
| `["helius-gatekeeper"]` | Helius Gatekeeper only |
| `["all-providers", "helius-gatekeeper"]` | **Helius Gatekeeper + Alchemy** ✨ |
| `["all-providers", "helius-gatekeeper", "websocket"]` | **Full stack with Gatekeeper** 🚀 |
| `["helius"]` | Helius (standard) only |
| `["helius", "websocket"]` | Helius (standard) with WebSocket |
| `["helius-gatekeeper", "websocket"]` | Helius Gatekeeper with WebSocket |
| `["alchemy"]` | Alchemy only |
| `["alchemy", "websocket"]` | Alchemy with WebSocket |

```toml
# Helius only (default)
aignt-solana-rpc = "0.1"

# Helius with Gatekeeper (faster beta endpoint)
aignt-solana-rpc = { version = "0.1", features = ["helius-gatekeeper"] }

# Alchemy only
aignt-solana-rpc = { version = "0.1", default-features = false, features = ["alchemy"] }

# Both providers (standard Helius + Alchemy)
aignt-solana-rpc = { version = "0.1", features = ["all-providers"] }

# Both providers with Helius Gatekeeper (faster)
aignt-solana-rpc = { version = "0.1", features = ["all-providers", "helius-gatekeeper"] }

# Both providers with Helius Gatekeeper + WebSocket (recommended for production)
aignt-solana-rpc = { version = "0.1", features = ["all-providers", "helius-gatekeeper", "websocket"] }

# Everything (standard endpoints)
aignt-solana-rpc = { version = "0.1", features = ["all-providers", "websocket"] }

# No providers (standard RPC only, no provider-specific APIs)
aignt-solana-rpc = { version = "0.1", default-features = false }
```

## Quick Start

```rust
use aignt_solana_rpc::SolanaRpcBuilder;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Single provider
    let rpc = SolanaRpcBuilder::new("helius", "your-api-key")?
        .build()?;

    // Standard RPC -- rate-limited + retried automatically
    let pubkey = Pubkey::from_str("So11111111111111111111111111111111111111112")?;
    let balance = rpc.get_balance(&pubkey).await?;
    println!("Balance: {} lamports", balance);

    // Provider-specific: priority fee estimation
    let estimate = rpc
        .get_priority_fee_estimate(Default::default())
        .await?;
    println!("Recommended fee: {} micro-lamports/CU", estimate.priority_fee);

    Ok(())
}
```

### Local Development (Localhost Validator)

For local development with `solana-test-validator`, use the `local` provider:

```rust
use aignt_solana_rpc::SolanaRpcBuilder;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Connect to local validator (http://127.0.0.1:8899)
    let rpc = SolanaRpcBuilder::new("local", "")?
        .build()?;

    let pubkey = Pubkey::from_str("So11111111111111111111111111111111111111112")?;
    let balance = rpc.get_balance(&pubkey).await?;
    println!("Balance: {} lamports", balance);

    Ok(())
}
```

The `local` provider automatically connects to `http://127.0.0.1:8899` (HTTP) and `ws://127.0.0.1:8900` (WebSocket) without requiring an API key.

### Using Helius Gatekeeper (Faster Performance)

For maximum performance, enable the `helius-gatekeeper` feature flag to automatically use the Helius Gatekeeper beta endpoint (faster):

#### Single Provider (Helius Only)
**In your Cargo.toml:**
```toml
[dependencies]
aignt-solana-rpc = { version = "0.1", features = ["helius-gatekeeper"] }
```

#### Multiple Providers (Helius Gatekeeper + Alchemy)
**In your Cargo.toml:**
```toml
[dependencies]
# Helius uses Gatekeeper, Alchemy uses standard endpoint
aignt-solana-rpc = { version = "0.1", features = ["all-providers", "helius-gatekeeper"] }
```

**No code changes needed** - all Helius configurations will automatically use the Gatekeeper endpoint:

```rust
use aignt_solana_rpc::SolanaRpcBuilder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // When helius-gatekeeper feature is enabled, Helius uses the faster Gatekeeper endpoint
    // Primary: Helius Gatekeeper (beta.helius-rpc.com)
    // Secondary: Alchemy (standard endpoint)
    let rpc = SolanaRpcBuilder::new("helius", "helius-api-key")?
        .with_secondary("alchemy", "alchemy-api-key")?
        .build()?;

    // All methods work the same, but with improved latency on Helius
    let balance = rpc.get_balance(&pubkey).await?;

    Ok(())
}
```

**Alternative:** Use `RpcConfig::helius_gatekeeper()` directly without the feature flag:
```rust
use aignt_solana_rpc::{SolanaRpc, RpcConfig};

let config = RpcConfig::helius_gatekeeper("your-api-key");
let rpc = SolanaRpc::new(config)?;
```

### With automatic failover

```rust
use aignt_solana_rpc::SolanaRpcBuilder;
use solana_sdk::commitment_config::CommitmentConfig;
use std::time::Duration;

let rpc = SolanaRpcBuilder::new("helius", "helius-key")?
    .with_secondary("alchemy", "alchemy-key")?
    .with_commitment(CommitmentConfig::finalized())
    .with_timeout(Duration::from_secs(60))
    .with_rate_limit(50)
    .build()?;

// Same API regardless of single or fallback -- failover is transparent
let balance = rpc.get_balance(&pubkey).await?;
let estimate = rpc.get_priority_fee_estimate(Default::default()).await?;
```

### From environment variables

```rust
use aignt_solana_rpc::SolanaRpcBuilder;

// Reads PRIMARY_PROVIDER (and optionally SECONDARY_PROVIDER),
// then resolves API keys from {PROVIDER}_API_KEY env vars
let rpc = SolanaRpcBuilder::from_env()?.build()?;
```

Set in your `.env`:

```bash
# Set API keys once
HELIUS_API_KEY=your-helius-key
ALCHEMY_API_KEY=your-alchemy-key

# Pick primary/secondary by name -- keys are resolved automatically
PRIMARY_PROVIDER=helius
SECONDARY_PROVIDER=alchemy
```

## Configuration

### SolanaRpcBuilder (Recommended)

`SolanaRpcBuilder` is the recommended way to construct a client. It handles provider selection, config overrides, and optional failover in a single fluent API:

```rust
use aignt_solana_rpc::{SolanaRpcBuilder, config::RetryConfig};
use solana_sdk::commitment_config::CommitmentConfig;
use std::time::Duration;

let rpc = SolanaRpcBuilder::new("helius", "your-key")?
    .with_secondary("alchemy", "backup-key")?     // optional failover
    .with_commitment(CommitmentConfig::finalized())
    .with_timeout(Duration::from_secs(60))
    .with_rate_limit(50)
    .with_retry(RetryConfig {
        max_retries: 5,
        base_delay: Duration::from_millis(250),
        max_delay: Duration::from_secs(30),
    })
    .with_failure_threshold(5)                     // failover-specific
    .with_primary_cooldown(Duration::from_secs(120))
    .build()?;
```

All overrides (commitment, timeout, retry, rate_limit) are applied to **both** primary and secondary providers. Failover-specific options (`failure_threshold`, `primary_cooldown`) only apply when a secondary is configured.

### Provider Configuration (Advanced)

Three ways to create a provider config:

**Direct constructors** -- when you know the provider at compile time:

```rust
use solana_rpc::RpcConfig;

// Helius -- constructs URL: https://mainnet.helius-rpc.com/?api-key=<key>
//           WebSocket URL:  wss://mainnet.helius-rpc.com/?api-key=<key>
//           With helius-gatekeeper feature: https://beta.helius-rpc.com/?api-key=<key>
let config = RpcConfig::helius("your-helius-api-key");

// Helius Gatekeeper (faster beta endpoint) -- always uses Gatekeeper regardless of feature flag
//                                             Constructs URL: https://beta.helius-rpc.com/?api-key=<key>
//                                             WebSocket URL:  wss://beta.helius-rpc.com/?api-key=<key>
let config = RpcConfig::helius_gatekeeper("your-helius-api-key");

// Alchemy -- constructs URL: https://solana-mainnet.g.alchemy.com/v2/<key>
//            WebSocket URL:  wss://solana-mainnet.g.alchemy.com/v2/<key>
let config = RpcConfig::alchemy("your-alchemy-api-key");

// Local validator -- constructs URL: http://127.0.0.1:8899
//                    WebSocket URL:  ws://127.0.0.1:8900
let config = RpcConfig::local();

// Custom endpoint (e.g. QuickNode, Triton, etc.)
let config = RpcConfig::custom(
    "https://my-rpc.example.com",
    Some("wss://my-rpc.example.com/ws"),
);

// Custom endpoint without WebSocket
let config = RpcConfig::custom("http://localhost:8899", None);
```

**`from_provider`** -- when the provider is chosen at runtime (env var, config file, CLI arg):

```rust
use solana_rpc::{RpcConfig, Provider};

// Provider::from_str is case-insensitive: "helius", "Helius", "HELIUS" all work
// Supported providers: "helius", "alchemy", "local"
let provider: Provider = "alchemy".parse().expect("invalid provider");
let config = RpcConfig::from_provider(provider, "your-api-key");

// For local provider, API key is ignored
let provider: Provider = "local".parse()?;
let config = RpcConfig::from_provider(provider, "");
```

This is the recommended approach for applications that need to switch providers without code changes. See [Config-Driven Provider Selection](#config-driven-provider-selection) for a full example.

### Retry Configuration

Retry uses exponential backoff: `base_delay * 2^attempt`, capped at `max_delay`. Only errors classified as retryable are retried (rate limits, timeouts, connection errors, 5xx). Non-retryable errors (simulation failures, invalid transactions, 401/403) return immediately.

```rust
use solana_rpc::{RpcConfig, config::RetryConfig};
use std::time::Duration;

let config = RpcConfig::helius("key")
    .with_retry(RetryConfig {
        max_retries: 5,                              // default: 3
        base_delay: Duration::from_millis(250),      // default: 500ms
        max_delay: Duration::from_secs(30),          // default: 10s
    });
```

**Defaults:**

| Parameter | Default | Description |
|-----------|---------|-------------|
| `max_retries` | `3` | Maximum retry attempts after initial failure |
| `base_delay` | `500ms` | Initial delay before first retry |
| `max_delay` | `10s` | Maximum delay between retries |

**Backoff schedule with defaults:** 500ms -> 1s -> 2s -> (give up)

### Rate Limiting

A token-bucket rate limiter (via the `governor` crate) is applied to every RPC call. Calls that exceed the limit are delayed until a token is available -- they are never rejected.

```rust
let config = RpcConfig::helius("key")
    .with_rate_limit(50);  // 50 requests per second (default: 20)
```

| Parameter | Default | Description |
|-----------|---------|-------------|
| `requests_per_second` | `20` | Maximum RPC requests per second |

### Commitment Level

```rust
use solana_sdk::commitment_config::CommitmentConfig;

let config = RpcConfig::helius("key")
    .with_commitment(CommitmentConfig::finalized());  // default: confirmed
```

### Timeout

```rust
use std::time::Duration;

let config = RpcConfig::helius("key")
    .with_timeout(Duration::from_secs(60));  // default: 30s
```

### Chaining All Overrides

```rust
use solana_rpc::{RpcConfig, config::RetryConfig};
use solana_sdk::commitment_config::CommitmentConfig;
use std::time::Duration;

let config = RpcConfig::helius("key")
    .with_commitment(CommitmentConfig::finalized())
    .with_timeout(Duration::from_secs(60))
    .with_rate_limit(50)
    .with_retry(RetryConfig {
        max_retries: 5,
        base_delay: Duration::from_millis(200),
        max_delay: Duration::from_secs(15),
    });

let rpc = SolanaRpc::new(config)?;
```

## Standard RPC Methods

All standard methods are rate-limited and retried automatically. They wrap `solana_client::nonblocking::rpc_client::RpcClient` and return `Result<T, RpcError>`.

```rust
use solana_rpc::{SolanaRpc, RpcConfig};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_transaction_status::UiTransactionEncoding;

let rpc = SolanaRpc::new(RpcConfig::helius("key"))?;

// Account queries
let balance = rpc.get_balance(&pubkey).await?;
let account = rpc.get_account(&pubkey).await?;
let accounts = rpc.get_multiple_accounts(&[pubkey1, pubkey2]).await?;

// Blockchain state
let blockhash = rpc.get_latest_blockhash().await?;
let slot = rpc.get_slot().await?;
let version = rpc.get_version().await?;

// Transactions
let sig = rpc.send_transaction(&transaction).await?;
let sig = rpc.send_transaction_with_config(&transaction, send_config).await?;
let sig = rpc.send_and_confirm_transaction(&transaction).await?;
let result = rpc.simulate_transaction(&transaction).await?;
let result = rpc.simulate_transaction_with_config(&transaction, sim_config).await?;

// Transaction history
let statuses = rpc.get_signature_statuses(&[signature]).await?;
let tx = rpc.get_transaction(&signature, UiTransactionEncoding::Json).await?;
let signatures = rpc.get_signatures_for_address_with_config(&address, config).await?;

// Program accounts
let accounts = rpc.get_program_accounts(&program_id).await?;
let accounts = rpc.get_program_accounts_with_config(&program_id, rpc_config).await?;

// Token accounts
let largest_accounts = rpc.get_token_largest_accounts(&mint_pubkey).await?;
let token_accounts = rpc.get_token_accounts_by_owner(&owner_pubkey, token_account_filter).await?;
let token_balance = rpc.get_token_account_balance(&token_account_pubkey).await?;
```

### Getting Signatures for an Address

To fetch transaction signatures for a specific address with configuration options:

```rust
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

let address = Pubkey::from_str("YourAddressHere")?;

let config = GetConfirmedSignaturesForAddress2Config {
    before: None,                    // Start from most recent
    until: None,                     // No end limit
    limit: Some(10),                 // Fetch up to 10 signatures
    commitment: None,                // Use client's default commitment
};

let signatures = rpc.get_signatures_for_address_with_config(&address, config).await?;

for sig_status in signatures {
    println!("Signature: {}, Slot: {}", sig_status.signature, sig_status.slot);
    if let Some(err) = sig_status.err {
        println!("  Error: {:?}", err);
    }
}
```

## Provider-Specific APIs

Provider-specific capabilities are accessed through trait accessors on `SolanaRpc`. If the provider doesn't support a feature (or the feature flag isn't enabled), the accessor returns `Err(RpcError::UnsupportedFeature { .. })`.

### Priority Fees

```rust
use solana_rpc::types::{PriorityFeeRequest, PriorityLevel};

let rpc = SolanaRpc::new(RpcConfig::helius("key"))?;

// Basic estimate
let estimate = rpc
    .priority_fees()?
    .get_priority_fee_estimate(PriorityFeeRequest {
        account_keys: vec!["JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string()],
        priority_level: Some(PriorityLevel::High),
    })
    .await?;

println!("Recommended fee: {} micro-lamports/CU", estimate.priority_fee);

// Access per-level breakdown (if available)
if let Some(levels) = &estimate.priority_fee_levels {
    println!("Min: {}, Low: {}, Medium: {}, High: {}, VeryHigh: {}, UnsafeMax: {}",
        levels.min, levels.low, levels.medium,
        levels.high, levels.very_high, levels.unsafe_max);
}
```

### DAS (Digital Asset Standard)

```rust
use solana_rpc::types::GetAssetsByOwnerRequest;

let rpc = SolanaRpc::new(RpcConfig::helius("key"))?;

// Get a single asset by its mint/ID
let asset = rpc.das()?.get_asset("asset-id-here").await?;
println!("Asset: {}", asset.id);

if let Some(content) = &asset.content {
    if let Some(metadata) = &content.metadata {
        println!("Name: {:?}", metadata.name);
    }
}

// Get all assets owned by a wallet
let response = rpc
    .das()?
    .get_assets_by_owner(GetAssetsByOwnerRequest {
        owner_address: "WaLLeT...".to_string(),
        page: 1,
        limit: 100,
        ..Default::default()
    })
    .await?;

println!("Found {} assets (total: {})", response.items.len(), response.total);
```

### Enhanced Transactions

```rust
let rpc = SolanaRpc::new(RpcConfig::helius("key"))?;

// Single transaction
let tx = rpc
    .enhanced_transactions()?
    .get_enhanced_transaction("5abc...signature")
    .await?;

println!("Type: {:?}, Fee: {:?}, Source: {:?}", tx.transaction_type, tx.fee, tx.source);

if let Some(transfers) = &tx.native_transfers {
    for t in transfers {
        println!("  {} -> {}: {} lamports", t.from_user_account, t.to_user_account, t.amount);
    }
}

// Batch
let txs = rpc
    .enhanced_transactions()?
    .get_enhanced_transactions(&["sig1".into(), "sig2".into()])
    .await?;
```

> **Note:** Helius provides rich enhanced transaction data (human-readable descriptions, typed events, full transfer breakdowns). Alchemy's implementation uses `getTransaction` with `jsonParsed` encoding and returns basic fields (fee, slot, timestamp). The same interface works for both -- the richness of the data depends on the provider.

## Failover with SolanaRpcBuilder

The simplest way to get automatic failover is via the builder. Just add `.with_secondary()`:

```rust
use aignt_solana_rpc::SolanaRpcBuilder;
use std::time::Duration;

let rpc = SolanaRpcBuilder::new("helius", "helius-key")?
    .with_secondary("alchemy", "alchemy-key")?
    .with_failure_threshold(3)                       // default: 3
    .with_primary_cooldown(Duration::from_secs(60))  // default: 60s
    .build()?;

// Same API regardless of single or fallback -- failover is transparent
let balance = rpc.get_balance(&pubkey).await?;
let slot = rpc.get_slot().await?;

// Provider-specific APIs also fail over
let estimate = rpc.get_priority_fee_estimate(Default::default()).await?;
let asset = rpc.get_asset("asset-id").await?;
let tx = rpc.get_enhanced_transaction("sig").await?;

// Check if fallback is configured
assert!(rpc.is_fallback_configured());
```

### How Failover Works

1. **All requests go to the primary** by default.
2. On a retryable error (rate limit, timeout, connection error, 5xx), the request is **immediately retried on the secondary** (single-request failover).
3. After `failure_threshold` consecutive primary failures (default: 3), **all traffic is routed to the secondary**.
4. After `primary_cooldown` elapses (default: 60s), the next request **probes the primary** again.
5. If the primary probe succeeds, the failure counter resets and traffic resumes on the primary.
6. **Non-retryable errors** (simulation failures, invalid transactions, 401/403) are **never** failed over -- they return immediately.

```
  Request
    │
    ▼
┌─────────────┐     healthy?      ┌─────────────┐
│   Primary    │◄─────yes─────────│  Health      │
│   Provider   │                  │  Tracker     │
└──────┬──────┘                  └──────┬──────┘
       │                                │
   success?                        cooldown
       │                          elapsed?
    ┌──┴──┐                         │
   yes    no (retryable)        ┌──┴──┐
    │      │                   yes    no
    │      ▼                    │     │
    │  ┌─────────────┐         │     ▼
    │  │  Secondary   │◄───────┘  [stay on
    │  │  Provider    │           secondary]
    │  └─────────────┘
    ▼
  Return
```

### Configuring Failover

| Parameter | Default | Description |
|-----------|---------|-------------|
| `failure_threshold` | `3` | Consecutive primary failures before routing all traffic to secondary |
| `primary_cooldown` | `60s` | Time to wait before probing primary again after marking it unhealthy |

```rust
// Aggressive failover: switch after 1 failure, retry primary after 30s
let rpc = SolanaRpcBuilder::new("helius", "h-key")?
    .with_secondary("alchemy", "a-key")?
    .with_failure_threshold(1)
    .with_primary_cooldown(Duration::from_secs(30))
    .build()?;

// Conservative failover: tolerate 10 failures, wait 5 minutes before retrying
let rpc = SolanaRpcBuilder::new("helius", "h-key")?
    .with_secondary("alchemy", "a-key")?
    .with_failure_threshold(10)
    .with_primary_cooldown(Duration::from_secs(300))
    .build()?;
```

### Inspecting Client State

```rust
// Check if fallback is configured
let has_fallback = rpc.is_fallback_configured();

// Access underlying solana RpcClient (primary's for fallback)
let inner = rpc.inner();

// Access primary config
let config = rpc.config();
println!("Using provider: {}", config.provider);
```

### Config-Driven Provider Selection

Use `SolanaRpcBuilder::from_env()` to make the primary/secondary choice entirely config-driven -- no code changes needed to swap providers.

```rust
use aignt_solana_rpc::SolanaRpcBuilder;

// Reads PRIMARY_PROVIDER, PRIMARY_API_KEY, and optionally
// SECONDARY_PROVIDER + SECONDARY_API_KEY from env vars
let rpc = SolanaRpcBuilder::from_env()?.build()?;

// Use rpc as normal -- provider choice is transparent
let balance = rpc.get_balance(&pubkey).await?;
```

Then in your `.env` or deployment config:

```bash
HELIUS_API_KEY=your-helius-key
ALCHEMY_API_KEY=your-alchemy-key

# Alchemy as primary, Helius as fallback
PRIMARY_PROVIDER=alchemy
SECONDARY_PROVIDER=helius

# Or use local validator for development
PRIMARY_PROVIDER=local
```

To flip the primary, just swap the provider names -- no code changes, no key duplication:

```bash
# Helius as primary, Alchemy as fallback
PRIMARY_PROVIDER=helius
SECONDARY_PROVIDER=alchemy
```

Provider names are case-insensitive, so `"helius"`, `"Helius"`, and `"HELIUS"` all work.

## Advanced: Manual Construction

For full control, you can still construct `SolanaRpc` and `FallbackRpc` directly:

```rust
use aignt_solana_rpc::{SolanaRpc, RpcConfig, FallbackRpc};
use std::time::Duration;

let primary = SolanaRpc::new(RpcConfig::helius("helius-key"))?;
let secondary = SolanaRpc::new(RpcConfig::alchemy("alchemy-key"))?;

let rpc = FallbackRpc::new(primary, secondary)
    .with_failure_threshold(3)
    .with_primary_cooldown(Duration::from_secs(60));

let balance = rpc.get_balance(&pubkey).await?;
```

## WebSocket Subscriptions

WebSocket support is integrated directly into the RPC client with **lazy initialization** - connections are established only when first accessed, with zero overhead for non-WebSocket users.

```toml
aignt-solana-rpc = { version = "0.1", features = ["helius", "websocket"] }
```

### Integrated WebSocket Access (Recommended)

Access WebSocket directly through your RPC client - no separate construction needed:

```rust
use aignt_solana_rpc::{SolanaRpcBuilder, Provider};
use solana_sdk::pubkey::Pubkey;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create RPC client (synchronous, no WebSocket yet)
    let rpc = SolanaRpcBuilder::new("helius", "your-api-key")?
        .build()?;

    // HTTP RPC works immediately
    let balance = rpc.get_balance(&pubkey).await?;

    // WebSocket initialized on first access (lazy)
    let ws = rpc.ws().await?;

    // Subscribe to account updates
    let mut sub = ws.subscribe_account(&pubkey).await?;
    while let Some(notification) = sub.receiver.recv().await {
        println!("Account updated: {:?}", notification);
    }

    Ok(())
}
```

### Mixed Provider Strategy (Alchemy HTTP + Helius WebSocket)

Use different providers for HTTP and WebSocket - perfect for when one provider has better HTTP performance and another has better WebSocket support:

```rust
use aignt_solana_rpc::{SolanaRpcBuilder, Provider};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Alchemy for HTTP (with Helius fallback), Helius for WebSocket
    let rpc = SolanaRpcBuilder::new("alchemy", "alchemy-key")?
        .with_secondary("helius", "helius-key")?
        .build()?;  // ✅ Synchronous!

    // HTTP calls use Alchemy with automatic failover to Helius
    let balance = rpc.get_balance(&pubkey).await?;
    println!("Balance: {}", balance);

    // WebSocket explicitly uses Helius (because Alchemy doesn't support it well)
    let ws = rpc.ws_by_provider(Provider::Helius).await?;

    // Subscribe to account updates
    let mut sub = ws.subscribe_account(&pubkey).await?;
    while let Some(notification) = sub.receiver.recv().await {
        println!("Account updated: {:?}", notification);
    }

    Ok(())
}
```

### Key Features

- **✅ No Breaking Changes** - `new()` and `build()` remain synchronous
- **✅ Lazy Initialization** - WebSocket connects only on first `ws()` call
- **✅ Zero Overhead** - Non-WebSocket users pay no cost
- **✅ Provider Selection** - Explicitly choose which provider's WebSocket to use
- **✅ Unified Interface** - HTTP and WebSocket through one client

### WebSocket API Methods

#### `rpc.ws()` - Get WebSocket from Primary Provider

```rust
let rpc = SolanaRpcBuilder::new("helius", "key")?.build()?;
let ws = rpc.ws().await?;  // Uses primary/single provider
```

#### `rpc.ws_by_provider()` - Select WebSocket by Provider

```rust
let rpc = SolanaRpcBuilder::new("alchemy", "key1")?
    .with_secondary("helius", "key2")?
    .build()?;

// Explicitly use Helius WebSocket
let ws = rpc.ws_by_provider(Provider::Helius).await?;
```

### Available Subscriptions

```rust
let ws = rpc.ws().await?;

// Account -- subscribe to changes on a specific account
let mut sub = ws.subscribe_account(&pubkey).await?;
while let Some(notification) = sub.receiver.recv().await {
    println!("Account updated: {:?}", notification.result);
}

// Slot -- subscribe to slot changes
let mut sub = ws.subscribe_slot().await?;

// Signature -- subscribe to transaction confirmation
let mut sub = ws.subscribe_signature(&signature).await?;

// Program -- subscribe to all accounts owned by a program
let mut sub = ws.subscribe_program(&program_id).await?;
```

### Auto-Reconnect Behavior

Each WebSocket client automatically handles disconnections:

- On disconnect, reconnects with exponential backoff (500ms base, 30s max)
- On reconnect, all active subscriptions are automatically re-established
- Ping/pong keepalive runs every 30 seconds to detect stale connections
- Thread-safe lazy initialization ensures only one connection per client
- Check connection status with `ws.is_connected()`

### Error Handling

```rust
match rpc.ws().await {
    Ok(ws) => {
        // Use WebSocket
        let sub = ws.subscribe_account(&pubkey).await?;
    }
    Err(RpcError::WebSocketError { message }) => {
        eprintln!("WebSocket unavailable: {}", message);
        // Fall back to HTTP-only mode
    }
    Err(e) => return Err(e.into()),
}
```

WebSocket initialization may fail for:
- **No WebSocket URL configured** - Provider doesn't have a `ws_url`
- **Connection failure** - Cannot reach WebSocket endpoint
- **Feature not enabled** - `websocket` feature not enabled in Cargo.toml

### Standalone WebSocket Client (Advanced)

For advanced use cases requiring independent WebSocket management:

#### WsClientBuilder

`WsClientBuilder` constructs a `WsConnection` with optional per-subscription failover:

```rust
use aignt_solana_rpc::WsClientBuilder;
use solana_sdk::commitment_config::CommitmentConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Single provider
    let ws = WsClientBuilder::new("helius", "your-api-key")?
        .build()
        .await?;

    // With failover
    let ws = WsClientBuilder::new("alchemy", "ws-alchemy-key")?
        .with_secondary("helius", "ws-helius-key")?
        .build()
        .await?;

    // From environment variables
    let ws = WsClientBuilder::from_env()?.build().await?;

    Ok(())
}
```

#### Manual WsClient Construction

For full control (custom endpoints):

```rust
use aignt_solana_rpc::{RpcConfig, ws::WsClient};

let config = RpcConfig::helius("your-api-key");
let ws = WsClient::connect(&config).await?;
```

## Error Handling

All errors flow through the `RpcError` enum. Errors are classified as retryable or non-retryable, which drives both the retry logic and the failover behavior.

```rust
use solana_rpc::RpcError;

match rpc.get_balance(&pubkey).await {
    Ok(balance) => println!("Balance: {balance}"),
    Err(RpcError::RateLimited { message }) => {
        // Retryable -- already retried max_retries times before reaching here
        eprintln!("Rate limited after retries: {message}");
    }
    Err(RpcError::Timeout { message }) => {
        // Retryable
        eprintln!("Timed out after retries: {message}");
    }
    Err(RpcError::Unauthorized { message }) => {
        // Non-retryable -- bad API key, check config
        eprintln!("Auth failed: {message}");
    }
    Err(RpcError::UnsupportedFeature { feature }) => {
        // Provider doesn't support this feature
        eprintln!("Not available: {feature}");
    }
    Err(e) => eprintln!("Error: {e}"),
}
```

### Retryable vs Failoverable

`is_retryable()` controls retry on the **same** provider. `should_failover()` controls whether a different provider might succeed.

| Error Variant | Retryable | Triggers Failover |
|---------------|-----------|-------------------|
| `RateLimited` | Yes | Yes |
| `Timeout` | Yes | Yes |
| `ConnectionError` | Yes | Yes |
| `TemporaryServerError` (5xx) | Yes | Yes |
| `Unauthorized` (401/403) | No | Yes |
| `ProviderApiError` | No | Yes |
| `WebSocketError` | No | Yes |
| `SolanaClient` | Inspected | Inspected |
| `Http` | No | Yes |
| `SimulationFailed` | No | No |
| `InvalidTransaction` | No | No |
| `UnsupportedFeature` | No | No |
| `Serialization` | No | No |

`SolanaClient` errors (from `solana_client::RpcClient`) are inspected for the inner cause -- a 429 or 5xx inside is retryable, a 401 triggers failover, a transaction error does not.

```rust
// Check programmatically
if err.is_retryable() {
    // Transient -- already retried max_retries times before reaching your code
}
if err.should_failover() {
    // Another provider might succeed (superset of is_retryable)
}
```

## Escape Hatch

For any RPC method not directly wrapped by `SolanaRpc`, you can access the underlying `solana_client::nonblocking::rpc_client::RpcClient` directly:

```rust
let rpc = SolanaRpc::new(RpcConfig::helius("key"))?;

// Access the inner solana RpcClient for any method
let inner = rpc.inner();
let health = inner.get_health().await?;
let supply = inner.supply().await?;

// Access the HTTP client for custom requests
let http = rpc.http_client();
let response = http.get("https://some-api.com").send().await?;

// Access the config
let config = rpc.config();
println!("Using provider: {}", config.provider);
println!("RPC URL: {}", config.rpc_url);
```

> **Note:** Calls made directly through `inner()` bypass rate limiting and retry. Use the wrapped methods when you want those guarantees.

## Building

```bash
# Default features (helius only)
cargo build

# All providers
cargo build --features all-providers

# All providers + WebSocket
cargo build --features all-providers,websocket

# No providers (standard RPC only)
cargo build --no-default-features

# Alchemy only
cargo build --no-default-features --features alchemy

# Release build
cargo build --release --features all-providers,websocket
```

## Testing

### Unit Tests

Unit tests use `httpmock` for provider API testing and don't require any API keys.

```bash
# Run all unit tests (all providers + websocket)
cargo test --lib --features all-providers,websocket

# Run tests for a specific module
cargo test --lib --features all-providers config::config_tests
cargo test --lib --features all-providers errors::errors_tests
cargo test --lib --features all-providers retry::retry_tests
cargo test --lib --features all-providers client::client_tests
cargo test --lib --features all-providers fallback::fallback_tests

# Provider-specific tests
cargo test --lib --features helius providers::helius
cargo test --lib --features alchemy providers::alchemy

# WebSocket tests
cargo test --lib --features websocket ws::client::client_tests
cargo test --lib --features all-providers,websocket ws::ws_connection::ws_connection_tests
```

### Integration Tests

Integration tests hit real RPC endpoints and require API keys set as environment variables.

```bash
# Set up environment
cp .env.example .env
# Edit .env with your actual API keys

# Run integration tests
HELIUS_API_KEY=your-key cargo test --tests --features helius
ALCHEMY_API_KEY=your-key cargo test --tests --features alchemy

# Run all integration tests
HELIUS_API_KEY=xxx ALCHEMY_API_KEY=xxx cargo test --tests --features all-providers,websocket
```

### Compile Checks

Verify all feature flag combinations compile:

```bash
cargo check                                         # default (helius)
cargo check --no-default-features                   # no providers
cargo check --features alchemy                      # alchemy + helius (default)
cargo check --no-default-features --features alchemy # alchemy only
cargo check --features all-providers                # both providers
cargo check --features all-providers,websocket      # everything
```

## Environment Variables

### Integration Tests

| Variable | Description | Required For |
|----------|-------------|--------------|
| `HELIUS_API_KEY` | Helius API key from [helius.dev](https://helius.dev) | Integration tests with `helius` feature |
| `ALCHEMY_API_KEY` | Alchemy API key from [alchemy.com](https://www.alchemy.com) | Integration tests with `alchemy` feature |

Copy `.env.example` to `.env` and fill in your keys:

```bash
cp .env.example .env
```

### Config-Driven Provider Selection (Application)

These are not used by the crate itself -- they're a recommended convention for applications using `SolanaRpcBuilder::from_env()` and `WsClientBuilder::from_env()`.

**API keys** (set once, shared across RPC and WS):

| Variable | Description |
|----------|-------------|
| `HELIUS_API_KEY` | Helius API key |
| `ALCHEMY_API_KEY` | Alchemy API key |

**RPC provider selection** (`SolanaRpcBuilder::from_env()`):

| Variable | Required | Description |
|----------|----------|-------------|
| `PRIMARY_PROVIDER` | Yes | Primary RPC provider name (`helius`, `alchemy`) |
| `SECONDARY_PROVIDER` | No | Fallback RPC provider name |

**WebSocket provider selection** (`WsClientBuilder::from_env()`):

| Variable | Required | Description |
|----------|----------|-------------|
| `WS_PRIMARY_PROVIDER` | Yes | Primary WS provider name (`helius`, `alchemy`) |
| `WS_SECONDARY_PROVIDER` | No | Secondary WS provider for failover |

API keys are resolved automatically: `PRIMARY_PROVIDER=helius` looks up `HELIUS_API_KEY`. WS providers are configured independently from RPC providers and can be in a different order.

## Architecture

```
solana-rpc/src/
├── lib.rs                              # Module exports, re-exports
├── constants.rs                        # Default timeout, retry, rate limit, provider URLs
├── types.rs                            # Provider enum, priority fee/DAS/enhanced tx types, JSON-RPC wrappers
├── config.rs                           # RpcConfig (builder), RetryConfig, RateLimitConfig
├── errors.rs                           # RpcError enum (thiserror), is_retryable(), from_reqwest()
├── retry.rs                            # with_retry() generic async retry with exponential backoff
├── client.rs                           # SolanaRpc -- main client, wraps RpcClient + reqwest + governor
├── fallback.rs                         # FallbackRpc -- primary/secondary with health tracking
├── rpc_client.rs                       # SolanaRpcClient (unified enum) + SolanaRpcBuilder
├── traits/
│   ├── mod.rs                          # Trait re-exports
│   ├── priority_fees.rs                # PriorityFeeProvider trait
│   ├── das.rs                          # DasProvider trait
│   └── enhanced_transactions.rs        # EnhancedTransactionProvider trait
├── providers/
│   ├── mod.rs                          # Feature-gated provider modules
│   ├── helius/
│   │   ├── mod.rs                      # HeliusProvider struct
│   │   ├── types.rs                    # Helius-specific request/response types
│   │   ├── priority_fees.rs            # PriorityFeeProvider impl for Helius
│   │   ├── das.rs                      # DasProvider impl for Helius
│   │   └── enhanced.rs                 # EnhancedTransactionProvider impl for Helius
│   └── alchemy/
│       ├── mod.rs                      # AlchemyProvider struct
│       ├── types.rs                    # Alchemy-specific request/response types
│       ├── priority_fees.rs            # PriorityFeeProvider impl for Alchemy
│       ├── das.rs                      # DasProvider impl for Alchemy
│       └── enhanced.rs                 # EnhancedTransactionProvider impl for Alchemy
└── ws/                                 # (behind "websocket" feature)
    ├── mod.rs
    ├── client.rs                       # WsClient with auto-reconnect, ping/pong
    ├── types.rs                        # SubscriptionType, WsNotification, SubscriptionHandle
    ├── subscriptions.rs                # logsSubscribe, accountSubscribe, signatureSubscribe
    ├── fallback.rs                     # FallbackWsClient -- per-subscription failover
    └── ws_connection.rs                # WsConnection (unified enum) + WsClientBuilder
```

### Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Standard RPC | Wrap `solana_client::nonblocking::RpcClient` | Reuse the official Solana client instead of reimplementing JSON-RPC |
| Provider APIs | Custom `reqwest` layer | Provider-specific endpoints (priority fees, DAS) aren't in the standard client |
| Provider dispatch | `async-trait` + `dyn` trait objects | Enables runtime polymorphism needed for failover between providers |
| Rate limiting | `governor` crate (token bucket) | Non-blocking, efficient, well-tested |
| Retry | Generic `with_retry()` function | Keeps retry logic in one place, each RPC method just wraps its call |
| Failover | Direct method delegation | Avoids complex generic closures; explicit and debuggable |
| WebSocket | `tokio-tungstenite` + `Notify`-based reconnect | Async-native, `Send`-safe architecture for auto-reconnect |
| Feature flags | Cargo features per provider | Compile only what you need; no dead code for unused providers |

## License

MIT
