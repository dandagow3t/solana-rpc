use aignt_solana_rpc::SolanaRpcBuilder;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Example: Connect to a local Solana validator
///
/// Prerequisites:
/// 1. Start a local validator:
///    ```
///    solana-test-validator
///    ```
///
/// 2. Run this example:
///    ```
///    cargo run --example local_validator --features all-providers
///    ```
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Connect to local validator at http://127.0.0.1:8899
    let rpc = SolanaRpcBuilder::new("local", "")?.build()?;

    println!("✅ Connected to local validator");
    println!("   RPC URL: {}", rpc.config().rpc_url);
    println!("   Provider: {}", rpc.config().provider);

    // Get version info
    let version = rpc.get_version().await?;
    println!("\n📊 Version Info:");
    println!("   Solana Core: {}", version.solana_core);
    if let Some(feature_set) = version.feature_set {
        println!("   Feature Set: {}", feature_set);
    }

    // Get current slot
    let slot = rpc.get_slot().await?;
    println!("\n⏰ Current Slot: {}", slot);

    // Get latest blockhash
    let blockhash = rpc.get_latest_blockhash().await?;
    println!("🔗 Latest Blockhash: {}", blockhash);

    // Check balance of the native token program (SOL)
    let native_mint = Pubkey::from_str("So11111111111111111111111111111111111111112")?;
    let balance = rpc.get_balance(&native_mint).await?;
    println!("\n💰 Balance of native mint: {} lamports", balance);

    // Note: Provider-specific APIs (priority fees, DAS, enhanced transactions)
    // are not available for the local provider
    if let Err(e) = rpc.get_priority_fee_estimate(Default::default()).await {
        println!("\n⚠️  Provider-specific APIs not available:");
        println!("   {}", e);
    }

    println!("\n✅ Example completed successfully!");

    Ok(())
}
