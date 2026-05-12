use aignt_solana_rpc::client::SolanaRpc;
use aignt_solana_rpc::config::RpcConfig;
use aignt_solana_rpc::types::PriorityFeeRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file
    dotenvy::dotenv().ok();

    println!("=== API Key Verification ===\n");

    // Check Helius
    match std::env::var("HELIUS_API_KEY") {
        Ok(key) => {
            println!("✅ HELIUS_API_KEY loaded from .env");
            println!(
                "   Key starts with: {}...",
                &key.chars().take(8).collect::<String>()
            );

            println!("   Testing real API call...");
            let rpc = SolanaRpc::new(RpcConfig::helius(&key))?;
            let request = PriorityFeeRequest {
                account_keys: vec!["JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string()],
                priority_level: None,
            };

            match rpc
                .priority_fees()?
                .get_priority_fee_estimate(request)
                .await
            {
                Ok(estimate) => {
                    println!(
                        "   ✅ Helius API working! Returned: {} micro-lamports/CU",
                        estimate.priority_fee
                    );
                }
                Err(e) => {
                    println!("   ❌ Helius API error: {}", e);
                }
            }
        }
        Err(_) => println!("❌ HELIUS_API_KEY not found in .env"),
    }

    println!();

    // Check Alchemy
    #[cfg(feature = "alchemy")]
    match std::env::var("ALCHEMY_API_KEY") {
        Ok(key) => {
            println!("✅ ALCHEMY_API_KEY loaded from .env");
            println!(
                "   Key starts with: {}...",
                &key.chars().take(8).collect::<String>()
            );

            println!("   Testing real API call...");
            let rpc = SolanaRpc::new(RpcConfig::alchemy(&key))?;
            let request = PriorityFeeRequest {
                account_keys: vec!["JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string()],
                priority_level: None,
            };

            match rpc
                .priority_fees()?
                .get_priority_fee_estimate(request)
                .await
            {
                Ok(estimate) => {
                    println!(
                        "   ✅ Alchemy API working! Returned: {} micro-lamports/CU",
                        estimate.priority_fee
                    );
                }
                Err(e) => {
                    println!("   ❌ Alchemy API error: {}", e);
                }
            }
        }
        Err(_) => println!("❌ ALCHEMY_API_KEY not found in .env"),
    }

    #[cfg(not(feature = "alchemy"))]
    {
        println!("ℹ️  Alchemy feature not enabled. Run with --features alchemy");
    }

    Ok(())
}
