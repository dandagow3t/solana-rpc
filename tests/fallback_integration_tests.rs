mod common;

use aignt_solana_rpc::client::SolanaRpc;
use aignt_solana_rpc::config::RpcConfig;
use aignt_solana_rpc::fallback::FallbackRpc;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::test]
async fn test_fallback_get_balance() {
    let (Some(helius_key), Some(alchemy_key)) =
        (common::helius_api_key(), common::alchemy_api_key())
    else {
        eprintln!("Skipping: HELIUS_API_KEY and ALCHEMY_API_KEY must both be set");
        return;
    };

    let primary = SolanaRpc::new(RpcConfig::helius(&helius_key)).unwrap();
    let secondary = SolanaRpc::new(RpcConfig::alchemy(&alchemy_key)).unwrap();
    let fallback = FallbackRpc::new(primary, secondary);

    let pubkey = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
    let balance = fallback.get_balance(&pubkey).await;
    assert!(
        balance.is_ok(),
        "fallback get_balance failed: {:?}",
        balance.err()
    );
}

#[tokio::test]
async fn test_fallback_get_slot() {
    let (Some(helius_key), Some(alchemy_key)) =
        (common::helius_api_key(), common::alchemy_api_key())
    else {
        eprintln!("Skipping: HELIUS_API_KEY and ALCHEMY_API_KEY must both be set");
        return;
    };

    let primary = SolanaRpc::new(RpcConfig::helius(&helius_key)).unwrap();
    let secondary = SolanaRpc::new(RpcConfig::alchemy(&alchemy_key)).unwrap();
    let fallback = FallbackRpc::new(primary, secondary);

    let slot = fallback.get_slot().await;
    assert!(slot.is_ok());
    assert!(slot.unwrap() > 0);
}

#[tokio::test]
async fn test_fallback_with_bad_primary() {
    let Some(alchemy_key) = common::alchemy_api_key() else {
        eprintln!("Skipping: ALCHEMY_API_KEY not set");
        return;
    };

    // Use a bad primary key to force fallback
    let primary = SolanaRpc::new(RpcConfig::helius("invalid-key")).unwrap();
    let secondary = SolanaRpc::new(RpcConfig::alchemy(&alchemy_key)).unwrap();
    let fallback = FallbackRpc::new(primary, secondary);

    let slot = fallback.get_slot().await;
    // Should succeed via secondary even though primary has bad key
    assert!(
        slot.is_ok(),
        "fallback should work with bad primary: {:?}",
        slot.err()
    );
}
