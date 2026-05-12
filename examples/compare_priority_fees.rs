use aignt_solana_rpc::client::SolanaRpc;
use aignt_solana_rpc::config::RpcConfig;
use aignt_solana_rpc::types::PriorityFeeRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    // Test accounts - using some common program accounts
    let test_accounts = vec![
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string(), // Jupiter
    ];

    println!("Comparing priority fee estimates for accounts:");
    for account in &test_accounts {
        println!("  - {}", account);
    }
    println!();

    // Test with Helius
    if let Ok(helius_key) = std::env::var("HELIUS_API_KEY") {
        println!("=== HELIUS ===");
        let helius_rpc = SolanaRpc::new(RpcConfig::helius(&helius_key))?;

        let request = PriorityFeeRequest {
            account_keys: test_accounts.clone(),
            priority_level: None,
        };

        match helius_rpc
            .priority_fees()
            .unwrap()
            .get_priority_fee_estimate(request)
            .await
        {
            Ok(estimate) => {
                println!("Priority fee: {}", estimate.priority_fee);
                if let Some(levels) = &estimate.priority_fee_levels {
                    println!("Fee levels:");
                    println!("  min: {}", levels.min);
                    println!("  low: {}", levels.low);
                    println!("  medium: {}", levels.medium);
                    println!("  high: {}", levels.high);
                    println!("  very_high: {}", levels.very_high);
                    println!("  unsafe_max: {}", levels.unsafe_max);
                }
            }
            Err(e) => println!("Error: {}", e),
        }
        println!();
    } else {
        println!("HELIUS_API_KEY not set, skipping Helius test\n");
    }

    // Test with Alchemy
    #[cfg(feature = "alchemy")]
    if let Ok(alchemy_key) = std::env::var("ALCHEMY_API_KEY") {
        println!("=== ALCHEMY ===");
        let alchemy_rpc = SolanaRpc::new(RpcConfig::alchemy(&alchemy_key))?;

        let request = PriorityFeeRequest {
            account_keys: test_accounts.clone(),
            priority_level: None,
        };

        match alchemy_rpc
            .priority_fees()
            .unwrap()
            .get_priority_fee_estimate(request)
            .await
        {
            Ok(estimate) => {
                println!("Priority fee: {}", estimate.priority_fee);
                if let Some(levels) = &estimate.priority_fee_levels {
                    println!("Fee levels:");
                    println!("  min: {}", levels.min);
                    println!("  low: {}", levels.low);
                    println!("  medium: {}", levels.medium);
                    println!("  high: {}", levels.high);
                    println!("  very_high: {}", levels.very_high);
                    println!("  unsafe_max: {}", levels.unsafe_max);
                }
            }
            Err(e) => println!("Error: {}", e),
        }
        println!();
    }

    #[cfg(not(feature = "alchemy"))]
    {
        println!("Alchemy feature not enabled. Run with --features alchemy to test Alchemy.\n");
    }

    Ok(())
}
