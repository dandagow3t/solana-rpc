use aignt_solana_rpc::rpc_client::SolanaRpcBuilder;
use aignt_solana_rpc::types::{PriorityFeeRequest, Provider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    // Build a fallback client with Alchemy primary and Helius secondary
    let client = SolanaRpcBuilder::from_env()?.build()?;

    let request = PriorityFeeRequest {
        account_keys: vec!["JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string()],
        priority_level: None,
    };

    println!("=== Explicit Provider Access Demo ===\n");

    // Method 1: Access by position (primary/secondary)
    println!("Method 1: Access by position");
    println!("Primary provider: {}", client.primary().config().provider);
    let primary_estimate = client
        .primary()
        .priority_fees()?
        .get_priority_fee_estimate(request.clone())
        .await?;
    println!(
        "Primary estimate: {} micro-lamports\n",
        primary_estimate.priority_fee
    );

    if let Some(secondary) = client.secondary() {
        println!("Secondary provider: {}", secondary.config().provider);
        let secondary_estimate = secondary
            .priority_fees()?
            .get_priority_fee_estimate(request.clone())
            .await?;
        println!(
            "Secondary estimate: {} micro-lamports\n",
            secondary_estimate.priority_fee
        );
    }

    // Method 2: Access by provider type (explicitly use Helius)
    println!("Method 2: Access by provider type");
    if let Some(helius) = client.client_by_provider(Provider::Helius) {
        println!("Found Helius provider!");
        let helius_estimate = helius
            .priority_fees()?
            .get_priority_fee_estimate(request.clone())
            .await?;

        println!(
            "Helius estimate: {} micro-lamports",
            helius_estimate.priority_fee
        );

        if let Some(levels) = &helius_estimate.priority_fee_levels {
            println!("\nHelius priority fee levels:");
            println!("  Min:        {} micro-lamports", levels.min);
            println!("  Low:        {} micro-lamports", levels.low);
            println!("  Medium:     {} micro-lamports", levels.medium);
            println!("  High:       {} micro-lamports", levels.high);
            println!("  Very High:  {} micro-lamports", levels.very_high);
            println!("  Unsafe Max: {} micro-lamports", levels.unsafe_max);
        }
    } else {
        println!("Helius provider not configured");
    }

    // Method 3: Iterate over all clients
    println!("\n\nMethod 3: Compare all providers");
    for (i, rpc) in client.all_clients().iter().enumerate() {
        let provider = rpc.provider();
        println!("\nProvider {}: {}", i + 1, provider);

        if let Ok(priority_fees) = rpc.priority_fees() {
            match priority_fees
                .get_priority_fee_estimate(request.clone())
                .await
            {
                Ok(estimate) => {
                    println!("  Estimate: {} micro-lamports", estimate.priority_fee);
                }
                Err(e) => {
                    println!("  Error: {}", e);
                }
            }
        } else {
            println!("  Priority fees not supported for this provider");
        }
    }

    Ok(())
}
