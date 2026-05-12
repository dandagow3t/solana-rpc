use super::*;
use httpmock::prelude::*;

#[tokio::test]
async fn test_get_enhanced_transactions_success() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(POST).path("/v0/transactions");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"[
                    {
                        "description": "Test transferred 1 SOL to Recipient",
                        "type": "TRANSFER",
                        "source": "SYSTEM_PROGRAM",
                        "fee": 5000,
                        "feePayer": "5ZiE3vAkrdXBgyFL7KqG3RoEGBws4CjRcXVbABDLZTgx",
                        "signature": "5VERv8NMhHSbEpzfR1ULhHB1xzF3e4sMnTQbUyFVzSHxA1N2oNGBfkB3c6qW3AsMUwP",
                        "slot": 171942732,
                        "timestamp": 1694000000,
                        "nativeTransfers": [
                            {
                                "fromUserAccount": "5ZiE3vAkrdXBgyFL7KqG3RoEGBws4CjRcXVbABDLZTgx",
                                "toUserAccount": "7YttLkHDoNj9wyDur5pM1ejNaAvT9X4eSTYAg54g9ePG",
                                "amount": 1000000000
                            }
                        ],
                        "tokenTransfers": [],
                        "accountData": [
                            {
                                "account": "5ZiE3vAkrdXBgyFL7KqG3RoEGBws4CjRcXVbABDLZTgx",
                                "nativeBalanceChange": -1000005000
                            }
                        ]
                    }
                ]"#,
            );
    });

    let provider = HeliusProvider::new(server.url(""), reqwest::Client::new());

    let signatures =
        vec!["5VERv8NMhHSbEpzfR1ULhHB1xzF3e4sMnTQbUyFVzSHxA1N2oNGBfkB3c6qW3AsMUwP".to_string()];
    let result = provider.get_enhanced_transactions(&signatures).await;
    mock.assert();

    let txs = result.unwrap();
    assert_eq!(txs.len(), 1);

    let tx = &txs[0];
    assert_eq!(tx.transaction_type.as_deref(), Some("TRANSFER"));
    assert_eq!(tx.source.as_deref(), Some("SYSTEM_PROGRAM"));
    assert_eq!(tx.fee, Some(5000));
    assert_eq!(
        tx.fee_payer.as_deref(),
        Some("5ZiE3vAkrdXBgyFL7KqG3RoEGBws4CjRcXVbABDLZTgx")
    );
    assert_eq!(tx.slot, Some(171942732));
    assert_eq!(tx.timestamp, Some(1694000000));

    let native_transfers = tx.native_transfers.as_ref().unwrap();
    assert_eq!(native_transfers.len(), 1);
    assert_eq!(native_transfers[0].amount, 1000000000);
}

#[tokio::test]
async fn test_get_enhanced_transaction_single() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(POST).path("/v0/transactions");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"[
                    {
                        "signature": "testsig123",
                        "type": "SWAP",
                        "source": "JUPITER"
                    }
                ]"#,
            );
    });

    let provider = HeliusProvider::new(server.url(""), reqwest::Client::new());

    let result = provider.get_enhanced_transaction("testsig123").await;
    mock.assert();

    let tx = result.unwrap();
    assert_eq!(tx.signature, "testsig123");
    assert_eq!(tx.transaction_type.as_deref(), Some("SWAP"));
}

#[tokio::test]
async fn test_get_enhanced_transactions_http_error() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(POST).path("/v0/transactions");
        then.status(401).body("unauthorized");
    });

    let provider = HeliusProvider::new(server.url(""), reqwest::Client::new());

    let result = provider
        .get_enhanced_transactions(&["sig".to_string()])
        .await;
    mock.assert();
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_enhanced_transaction_empty_response() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(POST).path("/v0/transactions");
        then.status(200)
            .header("content-type", "application/json")
            .body("[]");
    });

    let provider = HeliusProvider::new(server.url(""), reqwest::Client::new());

    let result = provider.get_enhanced_transaction("nosig").await;
    mock.assert();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No enhanced transaction returned")
    );
}
