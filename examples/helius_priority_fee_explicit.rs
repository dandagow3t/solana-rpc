/// Example: Explicitly use Helius for priority fee estimation
///
/// This example shows how to explicitly call Helius for priority fees,
/// regardless of whether it's configured as primary or secondary provider.
/// This is useful when you want to ensure you're using Helius specifically,
/// not just falling back to it on error.
use aignt_solana_rpc::rpc_client::SolanaRpcBuilder;
use aignt_solana_rpc::types::{PriorityFeeRequest, Provider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    // Build client from environment (could be any configuration)
    let client = SolanaRpcBuilder::from_env()?.build()?;

    let request = PriorityFeeRequest {
        account_keys: vec!["JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string()],
        priority_level: None, // Get all levels
    };

    println!("=== Explicitly Using Helius for Priority Fees ===\n");

    // Find and use Helius provider explicitly
    match client.client_by_provider(Provider::Helius) {
        Some(helius_client) => {
            println!("✓ Found Helius provider, fetching priority fee estimate...\n");

            let estimate = helius_client
                .priority_fees()?
                .get_priority_fee_estimate(request)
                .await?;

            println!("Helius Priority Fee Estimate:");
            println!(
                "  Recommended (medium): {} micro-lamports\n",
                estimate.priority_fee
            );

            if let Some(levels) = estimate.priority_fee_levels {
                println!("All priority fee levels from Helius:");
                println!("  Min (cheapest):        {} micro-lamports", levels.min);
                println!("  Low (slow):            {} micro-lamports", levels.low);
                println!("  Medium (balanced):     {} micro-lamports", levels.medium);
                println!("  High (fast):           {} micro-lamports", levels.high);
                println!(
                    "  Very High (urgent):    {} micro-lamports",
                    levels.very_high
                );
                println!(
                    "  Unsafe Max (critical): {} micro-lamports",
                    levels.unsafe_max
                );
            }
        }
        None => {
            println!("✗ Helius provider not configured in this client");
            println!("\nConfigured providers:");
            for rpc in client.all_clients() {
                println!("  - {}", rpc.provider());
            }
            println!("\nTo use Helius, set:");
            println!("  PRIMARY_PROVIDER=helius HELIUS_API_KEY=your-key");
            println!("  or");
            println!("  SECONDARY_PROVIDER=helius HELIUS_API_KEY=your-key");
        }
    }

    Ok(())
}
