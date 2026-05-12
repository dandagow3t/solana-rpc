/// Example: Test WebSocket connections for specific providers
///
/// This example demonstrates how to:
/// 1. Connect to WebSocket for a specific provider (Helius, Alchemy, etc.)
/// 2. Subscribe to different types of events (logs, account changes, signatures)
/// 3. Use fallback mode with primary and secondary providers
/// 4. Handle notifications from subscriptions
///
/// Usage:
/// ```bash
/// # Test single provider
/// PRIMARY_PROVIDER=helius HELIUS_API_KEY=your-key cargo run --example websocket_provider_test --features websocket
///
/// # Test with fallback
/// WS_PRIMARY_PROVIDER=helius HELIUS_API_KEY=key1 \
/// WS_SECONDARY_PROVIDER=alchemy ALCHEMY_API_KEY=key2 \
/// cargo run --example websocket_provider_test --features websocket -- --fallback
/// ```
#[cfg(feature = "websocket")]
use aignt_solana_rpc::rpc_client::SolanaRpcBuilder;
#[cfg(feature = "websocket")]
use solana_sdk::commitment_config::CommitmentConfig;
#[cfg(feature = "websocket")]
use std::time::Duration;
#[cfg(feature = "websocket")]
use tokio::time::timeout;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(feature = "websocket"))]
    {
        eprintln!("Error: This example requires the 'websocket' feature.");
        eprintln!("Run with: cargo run --example websocket_provider_test --features websocket");
        return Err("websocket feature required".into());
    }

    #[cfg(feature = "websocket")]
    {
        // Initialize logging
        tracing_subscriber::fmt::init();

        dotenvy::dotenv().ok();

        let args: Vec<String> = std::env::args().collect();
        let use_fallback = args.iter().any(|arg| arg == "--fallback");

        println!("╔═══════════════════════════════════════════════════════════╗");
        println!("║     WebSocket Provider Testing Example                   ║");
        println!("╚═══════════════════════════════════════════════════════════╝\n");

        if use_fallback {
            test_fallback_websocket_builder().await?;
        } else {
            test_single_provider_websocket().await?;
        }
    }

    Ok(())
}

/// Test WebSocket connection with a single provider
#[cfg(feature = "websocket")]
async fn test_single_provider_websocket() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Testing Single Provider WebSocket ===\n");

    // Build RPC client from environment
    let rpc_client = SolanaRpcBuilder::from_env()?.build()?;

    println!("✓ RPC Client initialized");
    println!("  Primary provider: {:?}", rpc_client.primary().provider());
    println!(
        "  Fallback configured: {}",
        rpc_client.is_fallback_configured()
    );

    // Access WebSocket through the RPC client
    println!("\n📡 Connecting to WebSocket...");
    let ws = rpc_client.ws().await?;

    if !ws.is_connected() {
        eprintln!("✗ WebSocket is not connected");
        return Err("WebSocket connection failed".into());
    }

    println!("✓ WebSocket connected!");

    // Test 1: Subscribe to program logs
    test_logs_subscription(&ws).await?;

    // Test 2: Subscribe to account updates
    test_account_subscription(&ws).await?;

    // Test 3: Subscribe to signature notifications
    test_signature_subscription(&ws).await?;

    println!("\n✅ All tests completed successfully!");

    Ok(())
}

/// Test WebSocket with fallback configuration (primary + secondary providers)
#[cfg(feature = "websocket")]
async fn test_fallback_websocket_builder() -> Result<(), Box<dyn std::error::Error>> {
    use aignt_solana_rpc::WsClientBuilder;

    println!("=== Testing Fallback WebSocket Configuration ===\n");

    // Build WebSocket client with fallback from environment
    println!("📡 Building WebSocket with fallback from environment...");
    let ws = WsClientBuilder::from_env()?.build().await?;

    println!("✓ WebSocket client built!");
    println!("  Connected: {}", ws.is_connected());
    println!("  Fallback configured: {}", ws.is_fallback_configured());

    if !ws.is_fallback_configured() {
        println!("\n⚠️  Warning: Fallback not configured!");
        println!("  Set WS_SECONDARY_PROVIDER env var to enable fallback");
        println!("\n  Example:");
        println!("    WS_PRIMARY_PROVIDER=helius HELIUS_API_KEY=key1 \\");
        println!("    WS_SECONDARY_PROVIDER=alchemy ALCHEMY_API_KEY=key2 \\");
        println!(
            "    cargo run --example websocket_provider_test --features websocket -- --fallback"
        );
    } else {
        println!("\n✓ Fallback configured correctly!");
    }

    // Test subscriptions with fallback
    test_logs_subscription(&ws).await?;

    println!("\n✅ Fallback test completed!");

    Ok(())
}

/// Test logs subscription (program logs)
#[cfg(feature = "websocket")]
async fn test_logs_subscription(ws: &impl WsSubscriber) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Test: Logs Subscription ---");

    // Subscribe to Jupiter Aggregator v6 program logs
    let jupiter_program = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
    println!("📝 Subscribing to logs for program: {}", jupiter_program);

    let mut handle = ws
        .logs_subscribe(
            vec![jupiter_program.to_string()],
            Some(CommitmentConfig::confirmed()),
        )
        .await?;

    println!("✓ Subscribed! Subscription ID: {}", handle.subscription_id);
    println!("  Waiting for notifications (10 seconds)...");

    // Wait for notifications with timeout
    let wait_result = timeout(Duration::from_secs(10), async {
        let mut count = 0;
        while let Some(notification) = handle.receiver.recv().await {
            count += 1;
            println!("\n  📬 Notification #{} received:", count);
            println!("     Subscription: {}", notification.subscription);
            println!(
                "     Data: {}",
                serde_json::to_string_pretty(&notification.result)?
            );

            if count >= 3 {
                break; // Stop after 3 notifications
            }
        }
        Ok::<_, Box<dyn std::error::Error>>(count)
    })
    .await;

    match wait_result {
        Ok(Ok(count)) if count > 0 => {
            println!("\n  ✓ Received {} notification(s)", count);
        }
        Ok(Ok(_)) => {
            println!("\n  ℹ️  No notifications received (network might be idle)");
        }
        Ok(Err(e)) => {
            println!("\n  ✗ Error processing notifications: {}", e);
        }
        Err(_) => {
            println!(
                "\n  ⏱️  Timeout - no notifications in 10 seconds (this is normal if network is idle)"
            );
        }
    }

    Ok(())
}

/// Test account subscription (account data changes)
#[cfg(feature = "websocket")]
async fn test_account_subscription(
    ws: &impl WsSubscriber,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Test: Account Subscription ---");

    // Subscribe to USDC mint account (very active account)
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    println!("📝 Subscribing to account: {}", usdc_mint);

    let mut handle = ws
        .account_subscribe(usdc_mint, Some(CommitmentConfig::confirmed()))
        .await?;

    println!("✓ Subscribed! Subscription ID: {}", handle.subscription_id);
    println!("  Waiting for account updates (5 seconds)...");

    // Wait for notifications with shorter timeout
    let wait_result = timeout(Duration::from_secs(5), async {
        if let Some(notification) = handle.receiver.recv().await {
            println!("\n  📬 Account update received:");
            println!("     Subscription: {}", notification.subscription);
            println!(
                "     Data: {}",
                serde_json::to_string_pretty(&notification.result)?
            );
            Ok::<_, Box<dyn std::error::Error>>(true)
        } else {
            Ok(false)
        }
    })
    .await;

    match wait_result {
        Ok(Ok(true)) => {
            println!("\n  ✓ Account update received!");
        }
        Ok(Ok(false)) => {
            println!("\n  ℹ️  No account updates received");
        }
        Ok(Err(e)) => {
            println!("\n  ✗ Error processing notification: {}", e);
        }
        Err(_) => {
            println!(
                "\n  ⏱️  Timeout - no updates in 5 seconds (this is expected for stable accounts)"
            );
        }
    }

    Ok(())
}

/// Test signature subscription (transaction confirmation)
#[cfg(feature = "websocket")]
async fn test_signature_subscription(
    ws: &impl WsSubscriber,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Test: Signature Subscription ---");

    // Note: You'd typically get this from a recently sent transaction
    // Using a placeholder here since we're not sending actual transactions
    let example_signature =
        "5j7s6NiJS3JAkvgkoc18WVAsiSaci2pxB2A6ueCJP4tprA2TFg9wSyTLeYouxPBJEMzJinENTkpA52YStRW5Dia7";

    println!("📝 Subscribing to signature: {}", example_signature);
    println!("  (Note: Using example signature - will timeout unless this specific tx is recent)");

    let mut handle = ws
        .signature_subscribe(example_signature, Some(CommitmentConfig::confirmed()))
        .await?;

    println!("✓ Subscribed! Subscription ID: {}", handle.subscription_id);
    println!("  Waiting for confirmation (5 seconds)...");

    let wait_result = timeout(Duration::from_secs(5), async {
        if let Some(notification) = handle.receiver.recv().await {
            println!("\n  📬 Signature notification received:");
            println!("     Subscription: {}", notification.subscription);
            println!(
                "     Data: {}",
                serde_json::to_string_pretty(&notification.result)?
            );
            Ok::<_, Box<dyn std::error::Error>>(true)
        } else {
            Ok(false)
        }
    })
    .await;

    match wait_result {
        Ok(Ok(true)) => {
            println!("\n  ✓ Signature confirmation received!");
        }
        Ok(Ok(false)) => {
            println!("\n  ℹ️  No confirmation received");
        }
        Ok(Err(e)) => {
            println!("\n  ✗ Error processing notification: {}", e);
        }
        Err(_) => {
            println!("\n  ⏱️  Timeout - this is expected for example signatures");
        }
    }

    Ok(())
}

/// Trait to abstract over different WebSocket client types for testing
#[cfg(feature = "websocket")]
trait WsSubscriber {
    async fn logs_subscribe(
        &self,
        mentions: Vec<String>,
        commitment: Option<CommitmentConfig>,
    ) -> Result<aignt_solana_rpc::ws::SubscriptionHandle, anyhow::Error>;

    async fn account_subscribe(
        &self,
        pubkey: &str,
        commitment: Option<CommitmentConfig>,
    ) -> Result<aignt_solana_rpc::ws::SubscriptionHandle, anyhow::Error>;

    async fn signature_subscribe(
        &self,
        signature: &str,
        commitment: Option<CommitmentConfig>,
    ) -> Result<aignt_solana_rpc::ws::SubscriptionHandle, anyhow::Error>;
}

// Import necessary types - these are behind the websocket feature
#[cfg(feature = "websocket")]
use aignt_solana_rpc::ws::{WsClient, WsConnection};

// Implement for WsClient (accessed through RPC client)
#[cfg(feature = "websocket")]
impl<'a> WsSubscriber for &'a WsClient {
    async fn logs_subscribe(
        &self,
        mentions: Vec<String>,
        commitment: Option<CommitmentConfig>,
    ) -> Result<aignt_solana_rpc::ws::SubscriptionHandle, anyhow::Error> {
        WsClient::logs_subscribe(self, mentions, commitment).await
    }

    async fn account_subscribe(
        &self,
        pubkey: &str,
        commitment: Option<CommitmentConfig>,
    ) -> Result<aignt_solana_rpc::ws::SubscriptionHandle, anyhow::Error> {
        WsClient::account_subscribe(self, pubkey, commitment).await
    }

    async fn signature_subscribe(
        &self,
        signature: &str,
        commitment: Option<CommitmentConfig>,
    ) -> Result<aignt_solana_rpc::ws::SubscriptionHandle, anyhow::Error> {
        WsClient::signature_subscribe(self, signature, commitment).await
    }
}

// Implement for WsConnection (standalone builder)
#[cfg(feature = "websocket")]
impl WsSubscriber for WsConnection {
    async fn logs_subscribe(
        &self,
        mentions: Vec<String>,
        commitment: Option<CommitmentConfig>,
    ) -> Result<aignt_solana_rpc::ws::SubscriptionHandle, anyhow::Error> {
        WsConnection::logs_subscribe(self, mentions, commitment).await
    }

    async fn account_subscribe(
        &self,
        pubkey: &str,
        commitment: Option<CommitmentConfig>,
    ) -> Result<aignt_solana_rpc::ws::SubscriptionHandle, anyhow::Error> {
        WsConnection::account_subscribe(self, pubkey, commitment).await
    }

    async fn signature_subscribe(
        &self,
        signature: &str,
        commitment: Option<CommitmentConfig>,
    ) -> Result<aignt_solana_rpc::ws::SubscriptionHandle, anyhow::Error> {
        WsConnection::signature_subscribe(self, signature, commitment).await
    }
}

#[cfg(feature = "websocket")]
impl<'a> WsSubscriber for &'a WsConnection {
    async fn logs_subscribe(
        &self,
        mentions: Vec<String>,
        commitment: Option<CommitmentConfig>,
    ) -> Result<aignt_solana_rpc::ws::SubscriptionHandle, anyhow::Error> {
        (*self).logs_subscribe(mentions, commitment).await
    }

    async fn account_subscribe(
        &self,
        pubkey: &str,
        commitment: Option<CommitmentConfig>,
    ) -> Result<aignt_solana_rpc::ws::SubscriptionHandle, anyhow::Error> {
        (*self).account_subscribe(pubkey, commitment).await
    }

    async fn signature_subscribe(
        &self,
        signature: &str,
        commitment: Option<CommitmentConfig>,
    ) -> Result<aignt_solana_rpc::ws::SubscriptionHandle, anyhow::Error> {
        (*self).signature_subscribe(signature, commitment).await
    }
}
