use aignt_solana_rpc::client::SolanaRpc;
use aignt_solana_rpc::config::RpcConfig;
use aignt_solana_rpc::types::PriorityFeeRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let helius_key = std::env::var("HELIUS_API_KEY")?;
    let rpc = SolanaRpc::new(RpcConfig::helius(&helius_key))?;

    let request = PriorityFeeRequest {
        account_keys: vec!["JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string()],
        priority_level: None, // Request all levels
    };

    let estimate = rpc
        .priority_fees()?
        .get_priority_fee_estimate(request)
        .await?;

    println!("=== Client can choose from all levels ===\n");

    // Option 1: Use the recommended default (medium)
    println!(
        "Recommended default: {} micro-lamports",
        estimate.priority_fee
    );

    // Option 2: Client chooses specific level based on urgency
    if let Some(levels) = &estimate.priority_fee_levels {
        println!("\nAll available levels:");
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

        // Example: Client logic to choose based on use case
        println!("\n=== Example client decision logic ===");

        let use_case = "normal_swap";
        let chosen_fee = match use_case {
            "arbitrage" => {
                println!("Use case: Arbitrage → choosing HIGH priority");
                levels.high
            }
            "urgent_tx" => {
                println!("Use case: Urgent transaction → choosing VERY HIGH");
                levels.very_high
            }
            "normal_swap" => {
                println!("Use case: Normal swap → choosing MEDIUM");
                levels.medium
            }
            "batch_job" => {
                println!("Use case: Batch job → choosing LOW");
                levels.low
            }
            _ => levels.medium,
        };

        println!("Chosen priority fee: {} micro-lamports", chosen_fee);
    }

    Ok(())
}
