#![cfg(feature = "websocket")]

mod common;

use aignt_solana_rpc::config::RpcConfig;
use aignt_solana_rpc::ws::WsClient;

#[tokio::test]
async fn test_ws_connect_and_subscribe_logs() {
    let Some(api_key) = common::helius_api_key() else {
        eprintln!("Skipping: HELIUS_API_KEY not set");
        return;
    };

    let config = RpcConfig::helius(&api_key);
    let ws = WsClient::connect(&config).await;
    assert!(ws.is_ok(), "WsClient connect failed: {:?}", ws.err());

    let ws = ws.unwrap();
    assert!(ws.is_connected());

    // Subscribe to Jupiter logs
    let handle = ws
        .logs_subscribe(
            vec!["JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string()],
            None,
        )
        .await;

    assert!(handle.is_ok(), "logs_subscribe failed: {:?}", handle.err());
    let handle = handle.unwrap();
    assert!(handle.subscription_id > 0);
}

#[tokio::test]
async fn test_ws_connect_and_subscribe_account() {
    let Some(api_key) = common::helius_api_key() else {
        eprintln!("Skipping: HELIUS_API_KEY not set");
        return;
    };

    let config = RpcConfig::helius(&api_key);
    let ws = WsClient::connect(&config).await.unwrap();

    let handle = ws
        .account_subscribe("So11111111111111111111111111111111111111112", None)
        .await;

    assert!(
        handle.is_ok(),
        "account_subscribe failed: {:?}",
        handle.err()
    );
}
