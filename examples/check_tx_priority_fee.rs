use solana_client::rpc_client::RpcClient;
use solana_sdk::bs58;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_transaction_status::UiTransactionEncoding;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let tx_sig =
        "2f7ATiwfVK1na5F7jYgmS5C8d1P2BYfgmgJNgkNSHa9vYw9GvRs4AVtcNJSuosdxTjCsRNY8LipkdaD1bt53s7gc";

    println!("Analyzing transaction: {}\n", tx_sig);

    // Connect to Helius
    if let Ok(helius_key) = std::env::var("HELIUS_API_KEY") {
        let url = format!("https://mainnet.helius-rpc.com/?api-key={}", helius_key);
        let client = RpcClient::new_with_commitment(url, CommitmentConfig::confirmed());

        let sig = solana_sdk::signature::Signature::from_str(tx_sig)?;

        match client.get_transaction_with_config(
            &sig,
            solana_client::rpc_config::RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::JsonParsed),
                max_supported_transaction_version: Some(0),
                ..Default::default()
            },
        ) {
            Ok(tx) => {
                println!("Transaction found!");

                if let Some(meta) = tx.transaction.meta {
                    match meta.compute_units_consumed {
                        solana_transaction_status::option_serializer::OptionSerializer::Some(
                            compute_units,
                        ) => {
                            println!("Compute units consumed: {}", compute_units);
                        }
                        _ => {}
                    }
                    println!("Fee: {} lamports", meta.fee);

                    // Try to find priority fee in instructions
                    let transaction = tx.transaction.transaction;
                    match transaction {
                        solana_transaction_status::EncodedTransaction::Json(ui_tx) => {
                            if let solana_transaction_status::UiMessage::Parsed(parsed) =
                                ui_tx.message
                            {
                                println!("\nInstructions ({} total):", parsed.instructions.len());
                                let mut priority_fee_microlamports: Option<u64> = None;
                                let mut compute_unit_limit: Option<u32> = None;

                                for (i, ix) in parsed.instructions.iter().enumerate() {
                                    match ix {
                                        solana_transaction_status::UiInstruction::Parsed(parsed_ix) => {
                                            match parsed_ix {
                                                solana_transaction_status::UiParsedInstruction::Parsed(p) => {
                                                    println!("  {}: {} - {}", i, p.program, p.parsed.get("type").unwrap_or(&serde_json::Value::Null));

                                                    if p.program == "compute budget" {
                                                        println!("     → ComputeBudget instruction");
                                                        if let Some(params) = p.parsed.get("info") {
                                                            println!("     → Info: {}", params);
                                                        }
                                                    }
                                                }
                                                solana_transaction_status::UiParsedInstruction::PartiallyDecoded(partial) => {
                                                    println!("  {}: {} (partially decoded)", i, partial.program_id);

                                                    if partial.program_id == "ComputeBudget111111111111111111111111111111" {
                                                        println!("     → ComputeBudget instruction (partially decoded)");
                                                        if let Ok(data) = bs58::decode(&partial.data).into_vec() {
                                                            if !data.is_empty() {
                                                                match data[0] {
                                                                    2 => {
                                                                        if data.len() >= 5 {
                                                                            let limit = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                                                                            compute_unit_limit = Some(limit);
                                                                            println!("     → SetComputeUnitLimit: {}", limit);
                                                                        }
                                                                    }
                                                                    3 => {
                                                                        if data.len() >= 9 {
                                                                            let price = u64::from_le_bytes([
                                                                                data[1], data[2], data[3], data[4],
                                                                                data[5], data[6], data[7], data[8]
                                                                            ]);
                                                                            priority_fee_microlamports = Some(price);
                                                                            println!("     → SetComputeUnitPrice: {} microlamports", price);
                                                                        }
                                                                    }
                                                                    _ => println!("     → Unknown ComputeBudget instruction type: {}", data[0]),
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        solana_transaction_status::UiInstruction::Compiled(compiled) => {
                                            // Get the program account from the account keys
                                            let program_id = parsed.account_keys.get(compiled.program_id_index as usize)
                                                .map(|k| k.pubkey.as_str());

                                            // Check if it's a ComputeBudget program
                                            if program_id == Some("ComputeBudget111111111111111111111111111111") {
                                                println!("  {}: ComputeBudget (compiled)", i);

                                                // Decode the instruction data (it's base58 encoded)
                                                if let Ok(data) = bs58::decode(&compiled.data).into_vec() {
                                                    if !data.is_empty() {
                                                        match data[0] {
                                                            2 => {
                                                                // SetComputeUnitLimit
                                                                if data.len() >= 5 {
                                                                    let limit = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                                                                    compute_unit_limit = Some(limit);
                                                                    println!("     → SetComputeUnitLimit: {}", limit);
                                                                }
                                                            }
                                                            3 => {
                                                                // SetComputeUnitPrice
                                                                if data.len() >= 9 {
                                                                    let price = u64::from_le_bytes([
                                                                        data[1], data[2], data[3], data[4],
                                                                        data[5], data[6], data[7], data[8]
                                                                    ]);
                                                                    priority_fee_microlamports = Some(price);
                                                                    println!("     → SetComputeUnitPrice: {} microlamports", price);
                                                                }
                                                            }
                                                            _ => println!("     → Unknown ComputeBudget instruction type: {}", data[0]),
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                println!("\n=== Priority Fee Analysis ===");
                                if let Some(price) = priority_fee_microlamports {
                                    println!("Priority fee: {} microlamports per CU", price);
                                    if let Some(limit) = compute_unit_limit {
                                        let total_priority_fee =
                                            (price as u128 * limit as u128) / 1_000_000;
                                        println!("Compute unit limit: {}", limit);
                                        println!(
                                            "Max priority fee: {} lamports",
                                            total_priority_fee
                                        );
                                    }
                                } else {
                                    println!("No priority fee set");
                                }
                            }
                        }
                        _ => println!("Transaction not in JSON format"),
                    }
                }
            }
            Err(e) => println!("Error fetching transaction: {}", e),
        }
    } else {
        println!("HELIUS_API_KEY not set");
    }

    Ok(())
}
