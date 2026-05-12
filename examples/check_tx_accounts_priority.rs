use aignt_solana_rpc::client::SolanaRpc;
use aignt_solana_rpc::config::RpcConfig;
use aignt_solana_rpc::types::PriorityFeeRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    // TX: 2f7ATiwfVK1na5F7jYgmS5C8d1P2BYfgmgJNgkNSHa9vYw9GvRs4AVtcNJSuosdxTjCsRNY8LipkdaD1bt53s7gc
    println!("╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║  Priority Fee Estimation Comparison                                      ║");
    println!("║  TX: 2f7ATiwfVK1na5F7jYgmS5C8d1P2BYfgmgJNgkNSHa9vYw9GvRs4...      ║");
    println!("╚══════════════════════════════════════════════════════════════════════════╝");
    println!("\n💡 Actual TX priority fee: 2,000,000 micro-lamports/CU");
    println!("   Compute units consumed: 117,272 CU");
    println!("   Total priority fee paid: 254,924 lamports\n");

    // Test 1: Just one account (Jupiter program)
    let single_account = vec!["JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string()];

    println!("┌──────────────────────────────────────────────────────────────────────────┐");
    println!("│ TEST 1: Querying with SINGLE account (JUP6L...)                          │");
    println!("└──────────────────────────────────────────────────────────────────────────┘");
    println!("Accounts sent to providers:");
    for acc in &single_account {
        println!("  • {}", acc);
    }
    println!();

    if let Ok(helius_key) = std::env::var("HELIUS_API_KEY") {
        println!("HELIUS (1 account):");
        let rpc = SolanaRpc::new(RpcConfig::helius(&helius_key))?;
        let request = PriorityFeeRequest {
            account_keys: single_account.clone(),
            priority_level: None,
        };

        match rpc
            .priority_fees()?
            .get_priority_fee_estimate(request)
            .await
        {
            Ok(estimate) => {
                println!("  Recommended: {} micro-lamports/CU", estimate.priority_fee);
                if let Some(levels) = &estimate.priority_fee_levels {
                    println!(
                        "  Levels: min={}, low={}, medium={}, high={}, very_high={}",
                        levels.min, levels.low, levels.medium, levels.high, levels.very_high
                    );
                }
            }
            Err(e) => println!("  Error: {}", e),
        }
        println!();
    }

    #[cfg(feature = "alchemy")]
    if let Ok(alchemy_key) = std::env::var("ALCHEMY_API_KEY") {
        println!("ALCHEMY (1 account):");
        let rpc = SolanaRpc::new(RpcConfig::alchemy(&alchemy_key))?;
        let request = PriorityFeeRequest {
            account_keys: single_account.clone(),
            priority_level: None,
        };

        match rpc
            .priority_fees()?
            .get_priority_fee_estimate(request)
            .await
        {
            Ok(estimate) => {
                println!("  Recommended: {} micro-lamports/CU", estimate.priority_fee);
                if let Some(levels) = &estimate.priority_fee_levels {
                    println!(
                        "  Levels: min={}, low={}, medium={}, high={}, very_high={}",
                        levels.min, levels.low, levels.medium, levels.high, levels.very_high
                    );
                }
            }
            Err(e) => println!("  Error: {}", e),
        }
        println!();
    }

    // Test 2: All writable accounts from the actual transaction
    let all_accounts = vec![
        // Writable accounts from message
        "HkVCdKCGAhLDxdGRUuAi7q5nqCdZ885T3CozCGDAjeNe".to_string(), // Signer
        "3Rky4sjYECzyCHcq38h3suyrb6ucDF99Abq9VbDuqhTW".to_string(),
        "3THFzMzuaNj3yftKA3A8bsNuys91rhHsusB2suVun42d".to_string(),
        "4Exh7VtJFe77wHq2fbXr8QRWEYVm5GdJvj1oC46nRhga".to_string(),
        "58oV4GY4yFddvj6DJWp8mkwZaKpL2pGazNnv1U9DQ5bL".to_string(),
        "8UsPFFXSKtEVXEjhzgMyNLL3YCRSqtSJSGkX5CYzXCcN".to_string(),
        "9K8MtW9G2AuLCT3miRKqbbdmBoTD3TT6zorTYgr97e87".to_string(),
        "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh".to_string(),
        "EehuYM7vxkB6XfWyMKjyEJcmnj1BN6KUx4XzDytkjWMF".to_string(),
        "F9mJDgbmcsJ2tNG4JsVTSnGrqrzU9Xr3xPqxsKSrpfp9".to_string(),
        // Writable accounts from address lookup tables
        "7CtQqDqNinWTUNwTGhau9tC4y2PSZLEJuAJMQPDUhNm2".to_string(),
        "81wVSFrdPdxJ5GoXmTeRMV8gyDbhnkNszh7zKiAkbb9G".to_string(),
        "AgtH7EzrbFnYDs148YMgJ9Qv5s66RcGCWcm2Y9LeDWWw".to_string(),
        "GoXrsVz1mxEj7qqdfniLsdBy9D9AMQ8EHDiEpq1Wghnj".to_string(),
    ];

    println!("┌──────────────────────────────────────────────────────────────────────────┐");
    println!(
        "│ TEST 2: Querying with ALL {} WRITABLE accounts from TX                 │",
        all_accounts.len()
    );
    println!("└──────────────────────────────────────────────────────────────────────────┘");
    println!("Accounts sent to providers:");
    for (i, acc) in all_accounts.iter().enumerate() {
        println!("  {:2}. {}", i + 1, acc);
    }
    println!();

    if let Ok(helius_key) = std::env::var("HELIUS_API_KEY") {
        println!("HELIUS (14 accounts):");
        let rpc = SolanaRpc::new(RpcConfig::helius(&helius_key))?;
        let request = PriorityFeeRequest {
            account_keys: all_accounts.clone(),
            priority_level: None,
        };

        match rpc
            .priority_fees()?
            .get_priority_fee_estimate(request)
            .await
        {
            Ok(estimate) => {
                println!("  Recommended: {} micro-lamports/CU", estimate.priority_fee);
                if let Some(levels) = &estimate.priority_fee_levels {
                    println!(
                        "  Levels: min={}, low={}, medium={}, high={}, very_high={}",
                        levels.min, levels.low, levels.medium, levels.high, levels.very_high
                    );
                }
            }
            Err(e) => println!("  Error: {}", e),
        }
        println!();
    }

    #[cfg(feature = "alchemy")]
    if let Ok(alchemy_key) = std::env::var("ALCHEMY_API_KEY") {
        println!("ALCHEMY (14 accounts):");
        let rpc = SolanaRpc::new(RpcConfig::alchemy(&alchemy_key))?;
        let request = PriorityFeeRequest {
            account_keys: all_accounts.clone(),
            priority_level: None,
        };

        match rpc
            .priority_fees()?
            .get_priority_fee_estimate(request)
            .await
        {
            Ok(estimate) => {
                println!("  Recommended: {} micro-lamports/CU", estimate.priority_fee);
                if let Some(levels) = &estimate.priority_fee_levels {
                    println!(
                        "  Levels: min={}, low={}, medium={}, high={}, very_high={}",
                        levels.min, levels.low, levels.medium, levels.high, levels.very_high
                    );
                }
            }
            Err(e) => println!("  Error: {}", e),
        }
        println!();
    }

    Ok(())
}
