use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    // Use the exact same test accounts
    let test_accounts = vec!["JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string()];

    println!("Testing with accounts: {:?}\n", test_accounts);

    // Test Helius raw response
    if let Ok(helius_key) = std::env::var("HELIUS_API_KEY") {
        println!("=== HELIUS RAW RESPONSE ===");
        let url = format!("https://mainnet.helius-rpc.com/?api-key={}", helius_key);

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "getPriorityFeeEstimate",
            "params": [{
                "accountKeys": test_accounts,
                "options": {
                    "includeAllPriorityFeeLevels": true
                }
            }]
        });

        let client = reqwest::Client::new();
        let response = client.post(&url).json(&request).send().await?;
        let json: Value = response.json().await?;
        println!("{}\n", serde_json::to_string_pretty(&json)?);
    }

    // Test Alchemy raw response
    if let Ok(alchemy_key) = std::env::var("ALCHEMY_API_KEY") {
        println!("=== ALCHEMY RAW RESPONSE ===");
        let url = format!("https://solana-mainnet.g.alchemy.com/v2/{}", alchemy_key);

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "getPriorityFeeEstimate",
            "params": [{
                "accountKeys": test_accounts,
                "options": {
                    "includeAllPriorityFeeLevels": true
                }
            }]
        });

        let client = reqwest::Client::new();
        let response = client.post(&url).json(&request).send().await?;
        let json: Value = response.json().await?;
        println!("{}\n", serde_json::to_string_pretty(&json)?);
    }

    // Now test through our SDK
    println!("=== SDK RESPONSES ===\n");

    use aignt_solana_rpc::client::SolanaRpc;
    use aignt_solana_rpc::config::RpcConfig;
    use aignt_solana_rpc::types::PriorityFeeRequest;

    if let Ok(helius_key) = std::env::var("HELIUS_API_KEY") {
        println!("HELIUS SDK:");
        let rpc = SolanaRpc::new(RpcConfig::helius(&helius_key))?;
        let request = PriorityFeeRequest {
            account_keys: test_accounts.clone(),
            priority_level: None,
        };

        let estimate = rpc
            .priority_fees()?
            .get_priority_fee_estimate(request)
            .await?;
        println!("  priority_fee: {}", estimate.priority_fee);
        if let Some(levels) = &estimate.priority_fee_levels {
            println!(
                "  Levels: min={}, low={}, medium={}, high={}, very_high={}, unsafe_max={}",
                levels.min,
                levels.low,
                levels.medium,
                levels.high,
                levels.very_high,
                levels.unsafe_max
            );
        }
        println!();
    }

    #[cfg(feature = "alchemy")]
    if let Ok(alchemy_key) = std::env::var("ALCHEMY_API_KEY") {
        println!("ALCHEMY SDK:");
        let rpc = SolanaRpc::new(RpcConfig::alchemy(&alchemy_key))?;
        let request = PriorityFeeRequest {
            account_keys: test_accounts.clone(),
            priority_level: None,
        };

        let estimate = rpc
            .priority_fees()?
            .get_priority_fee_estimate(request)
            .await?;
        println!("  priority_fee: {}", estimate.priority_fee);
        if let Some(levels) = &estimate.priority_fee_levels {
            println!(
                "  Levels: min={}, low={}, medium={}, high={}, very_high={}, unsafe_max={}",
                levels.min,
                levels.low,
                levels.medium,
                levels.high,
                levels.very_high,
                levels.unsafe_max
            );
        }
        println!();
    }

    Ok(())
}
