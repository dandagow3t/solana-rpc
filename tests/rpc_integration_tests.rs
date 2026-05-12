mod common;

use aignt_solana_rpc::types::PriorityFeeRequest;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Helius integration tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_helius_get_balance() {
    let Some(rpc) = common::helius_rpc() else {
        eprintln!("Skipping: HELIUS_API_KEY not set");
        return;
    };

    let pubkey = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
    let balance = rpc.get_balance(&pubkey).await;
    assert!(balance.is_ok(), "get_balance failed: {:?}", balance.err());
}

#[tokio::test]
async fn test_helius_get_slot() {
    let Some(rpc) = common::helius_rpc() else {
        eprintln!("Skipping: HELIUS_API_KEY not set");
        return;
    };

    let slot = rpc.get_slot().await;
    assert!(slot.is_ok());
    assert!(slot.unwrap() > 0);
}

#[tokio::test]
async fn test_helius_get_latest_blockhash() {
    let Some(rpc) = common::helius_rpc() else {
        eprintln!("Skipping: HELIUS_API_KEY not set");
        return;
    };

    let blockhash = rpc.get_latest_blockhash().await;
    assert!(blockhash.is_ok());
}

#[tokio::test]
async fn test_helius_priority_fee_estimate() {
    let Some(rpc) = common::helius_rpc() else {
        eprintln!("Skipping: HELIUS_API_KEY not set");
        return;
    };

    let request = PriorityFeeRequest {
        account_keys: vec!["JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string()],
        priority_level: None,
    };

    let result = rpc
        .priority_fees()
        .unwrap()
        .get_priority_fee_estimate(request)
        .await;
    assert!(
        result.is_ok(),
        "priority fee estimate failed: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_helius_get_version() {
    let Some(rpc) = common::helius_rpc() else {
        eprintln!("Skipping: HELIUS_API_KEY not set");
        return;
    };

    let version = rpc.get_version().await;
    assert!(version.is_ok());
}

#[tokio::test]
async fn test_helius_get_token_account_balance() {
    let Some(rpc) = common::helius_rpc() else {
        eprintln!("Skipping: HELIUS_API_KEY not set");
        return;
    };

    // Use a well-known USDC token account address
    // This is Jupiter's USDC token account (known to have balance)
    let token_account = Pubkey::from_str("4wBqpZM9xaSheVZLYRA9MpKPZmTqKq9m9s7bSXmf52Ws").unwrap();

    let balance = rpc.get_token_account_balance(&token_account).await;
    assert!(
        balance.is_ok(),
        "get_token_account_balance failed: {:?}",
        balance.err()
    );

    let balance = balance.unwrap();
    // Verify the response has the expected fields
    assert!(balance.amount.parse::<u64>().is_ok());
    assert!(balance.decimals > 0);
}

#[tokio::test]
async fn test_helius_get_signatures_for_address_with_config() {
    let Some(rpc) = common::helius_rpc() else {
        eprintln!("Skipping: HELIUS_API_KEY not set");
        return;
    };

    // Use a well-known address with transaction history (Jupiter program)
    let address = Pubkey::from_str("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap();

    let config = GetConfirmedSignaturesForAddress2Config {
        before: None,
        until: None,
        limit: Some(5), // Only fetch 5 signatures for testing
        commitment: None,
    };

    let signatures = rpc
        .get_signatures_for_address_with_config(&address, config)
        .await;
    assert!(
        signatures.is_ok(),
        "get_signatures_for_address_with_config failed: {:?}",
        signatures.err()
    );

    let signatures = signatures.unwrap();
    assert!(!signatures.is_empty(), "Expected at least one signature");
    assert!(signatures.len() <= 5, "Expected at most 5 signatures");

    // Verify the response structure
    for sig in signatures {
        assert!(sig.signature.len() > 0, "Signature should not be empty");
        assert!(sig.slot > 0, "Slot should be greater than 0");
    }
}

// ---------------------------------------------------------------------------
// Alchemy integration tests
// ---------------------------------------------------------------------------

#[cfg(feature = "alchemy")]
#[tokio::test]
async fn test_alchemy_get_balance() {
    let Some(rpc) = common::alchemy_rpc() else {
        eprintln!("Skipping: ALCHEMY_API_KEY not set");
        return;
    };

    let pubkey = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
    let balance = rpc.get_balance(&pubkey).await;
    assert!(balance.is_ok(), "get_balance failed: {:?}", balance.err());
}

#[cfg(feature = "alchemy")]
#[tokio::test]
async fn test_alchemy_get_slot() {
    let Some(rpc) = common::alchemy_rpc() else {
        eprintln!("Skipping: ALCHEMY_API_KEY not set");
        return;
    };

    let slot = rpc.get_slot().await;
    assert!(slot.is_ok());
    assert!(slot.unwrap() > 0);
}

#[cfg(feature = "alchemy")]
#[tokio::test]
async fn test_alchemy_get_token_account_balance() {
    let Some(rpc) = common::alchemy_rpc() else {
        eprintln!("Skipping: ALCHEMY_API_KEY not set");
        return;
    };

    // Use a well-known USDC token account address
    // This is Jupiter's USDC token account (known to have balance)
    let token_account = Pubkey::from_str("4wBqpZM9xaSheVZLYRA9MpKPZmTqKq9m9s7bSXmf52Ws").unwrap();

    let balance = rpc.get_token_account_balance(&token_account).await;
    assert!(
        balance.is_ok(),
        "get_token_account_balance failed: {:?}",
        balance.err()
    );

    let balance = balance.unwrap();
    // Verify the response has the expected fields
    assert!(balance.amount.parse::<u64>().is_ok());
    assert!(balance.decimals > 0);
}

#[cfg(feature = "alchemy")]
#[tokio::test]
async fn test_alchemy_get_signatures_for_address_with_config() {
    let Some(rpc) = common::alchemy_rpc() else {
        eprintln!("Skipping: ALCHEMY_API_KEY not set");
        return;
    };

    // Use a well-known address with transaction history (Jupiter program)
    let address = Pubkey::from_str("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap();

    let config = GetConfirmedSignaturesForAddress2Config {
        before: None,
        until: None,
        limit: Some(5), // Only fetch 5 signatures for testing
        commitment: None,
    };

    let signatures = rpc
        .get_signatures_for_address_with_config(&address, config)
        .await;
    assert!(
        signatures.is_ok(),
        "get_signatures_for_address_with_config failed: {:?}",
        signatures.err()
    );

    let signatures = signatures.unwrap();
    assert!(!signatures.is_empty(), "Expected at least one signature");
    assert!(signatures.len() <= 5, "Expected at most 5 signatures");

    // Verify the response structure
    for sig in signatures {
        assert!(sig.signature.len() > 0, "Signature should not be empty");
        assert!(sig.slot > 0, "Slot should be greater than 0");
    }
}
