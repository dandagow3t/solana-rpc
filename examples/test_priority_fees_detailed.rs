use aignt_solana_rpc::client::SolanaRpc;
use aignt_solana_rpc::config::RpcConfig;
use aignt_solana_rpc::types::PriorityFeeRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let test_accounts = vec![
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string(), // Jupiter
    ];

    println!("Testing priority fees for: {:?}\n", test_accounts);

    // Test Helius
    if let Ok(helius_key) = std::env::var("HELIUS_API_KEY") {
        println!("╔════════════════════════════════════════╗");
        println!("║            HELIUS                      ║");
        println!("╚════════════════════════════════════════╝");

        let rpc = SolanaRpc::new(RpcConfig::helius(&helius_key))?;
        let request = PriorityFeeRequest {
            account_keys: test_accounts.clone(),
            priority_level: None,
        };

        match rpc
            .priority_fees()?
            .get_priority_fee_estimate(request)
            .await
        {
            Ok(estimate) => {
                println!(
                    "✓ Recommended priority_fee: {} micro-lamports",
                    estimate.priority_fee
                );

                if let Some(levels) = &estimate.priority_fee_levels {
                    println!("\nAll levels returned:");
                    println!("  min:        {:>12} micro-lamports", levels.min);
                    println!("  low:        {:>12} micro-lamports", levels.low);
                    println!(
                        "  medium:     {:>12} micro-lamports ← USED AS DEFAULT",
                        levels.medium
                    );
                    println!("  high:       {:>12} micro-lamports", levels.high);
                    println!("  very_high:  {:>12} micro-lamports", levels.very_high);
                    println!("  unsafe_max: {:>12} micro-lamports", levels.unsafe_max);

                    // Verify we're using medium
                    assert_eq!(
                        estimate.priority_fee, levels.medium,
                        "priority_fee should equal medium level"
                    );
                    println!("\n✓ Verified: priority_fee matches medium level");
                } else {
                    println!("⚠ No fee levels returned");
                }
            }
            Err(e) => println!("✗ Error: {}", e),
        }
        println!();
    }

    // Test Alchemy
    #[cfg(feature = "alchemy")]
    if let Ok(alchemy_key) = std::env::var("ALCHEMY_API_KEY") {
        println!("╔════════════════════════════════════════╗");
        println!("║            ALCHEMY                     ║");
        println!("╚════════════════════════════════════════╝");

        let rpc = SolanaRpc::new(RpcConfig::alchemy(&alchemy_key))?;
        let request = PriorityFeeRequest {
            account_keys: test_accounts.clone(),
            priority_level: None,
        };

        match rpc
            .priority_fees()?
            .get_priority_fee_estimate(request)
            .await
        {
            Ok(estimate) => {
                println!(
                    "✓ Recommended priority_fee: {} micro-lamports",
                    estimate.priority_fee
                );

                if let Some(levels) = &estimate.priority_fee_levels {
                    println!("\nAll levels returned:");
                    println!("  min:        {:>12} micro-lamports", levels.min);
                    println!("  low:        {:>12} micro-lamports", levels.low);
                    println!(
                        "  medium:     {:>12} micro-lamports ← USED AS DEFAULT",
                        levels.medium
                    );
                    println!("  high:       {:>12} micro-lamports", levels.high);
                    println!("  very_high:  {:>12} micro-lamports", levels.very_high);
                    println!("  unsafe_max: {:>12} micro-lamports", levels.unsafe_max);

                    // Verify we're using medium
                    assert_eq!(
                        estimate.priority_fee, levels.medium,
                        "priority_fee should equal medium level"
                    );
                    println!("\n✓ Verified: priority_fee matches medium level");
                } else {
                    println!("⚠ No fee levels returned");
                }
            }
            Err(e) => println!("✗ Error: {}", e),
        }
        println!();
    }

    #[cfg(not(feature = "alchemy"))]
    println!("ℹ Alchemy feature not enabled. Run with --features alchemy to test.");

    Ok(())
}
