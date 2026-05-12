use crate::config::RpcConfig;

// WsClient tests require a real WebSocket server, so we only test
// that the connect fails gracefully with bad URLs.

#[tokio::test]
async fn test_ws_connect_invalid_url_fails() {
    let config = RpcConfig::custom("https://invalid", Some("wss://invalid-ws-url:99999"));
    let result = super::WsClient::connect(&config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_ws_connect_no_ws_url_fails() {
    let config = RpcConfig::custom("https://invalid", None);
    let result = super::WsClient::connect(&config).await;
    assert!(result.is_err());
    assert!(
        result
            .err()
            .unwrap()
            .to_string()
            .contains("No WebSocket URL configured")
    );
}
