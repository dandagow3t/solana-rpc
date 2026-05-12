use super::*;
use httpmock::prelude::*;

#[tokio::test]
async fn test_get_enhanced_transaction_success() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(POST).path("/");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                    "jsonrpc": "2.0",
                    "id": "1",
                    "result": {
                        "slot": 250000000,
                        "blockTime": 1700000000,
                        "meta": {
                            "fee": 5000,
                            "err": null
                        },
                        "transaction": {
                            "signatures": ["testsig"]
                        }
                    }
                }"#,
            );
    });

    let provider = AlchemyProvider::new(server.url("/"), reqwest::Client::new());
    let result = provider.get_enhanced_transaction("testsig").await;
    mock.assert();

    let tx = result.unwrap();
    assert_eq!(tx.signature, "testsig");
    assert_eq!(tx.fee, Some(5000));
    assert_eq!(tx.slot, Some(250000000));
    assert_eq!(tx.timestamp, Some(1700000000));
    assert_eq!(tx.source.as_deref(), Some("Alchemy"));
}

#[tokio::test]
async fn test_get_enhanced_transaction_rpc_error() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(POST).path("/");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                    "jsonrpc": "2.0",
                    "id": "1",
                    "error": {
                        "code": -32602,
                        "message": "Transaction not found"
                    }
                }"#,
            );
    });

    let provider = AlchemyProvider::new(server.url("/"), reqwest::Client::new());
    let result = provider.get_enhanced_transaction("invalid").await;
    mock.assert();
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_enhanced_transactions_multiple() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(POST).path("/");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{
                    "jsonrpc": "2.0",
                    "id": "1",
                    "result": {
                        "slot": 100,
                        "blockTime": 1700000000,
                        "meta": {
                            "fee": 5000
                        },
                        "transaction": {
                            "signatures": ["sig"]
                        }
                    }
                }"#,
            );
    });

    let provider = AlchemyProvider::new(server.url("/"), reqwest::Client::new());
    let sigs = vec!["sig1".to_string(), "sig2".to_string()];
    let result = provider.get_enhanced_transactions(&sigs).await;
    mock.assert_calls(2);

    let txs = result.unwrap();
    assert_eq!(txs.len(), 2);
}

#[tokio::test]
async fn test_get_enhanced_transaction_http_error() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(POST).path("/");
        then.status(500).body("internal error");
    });

    let provider = AlchemyProvider::new(server.url("/"), reqwest::Client::new());
    let result = provider.get_enhanced_transaction("sig").await;
    mock.assert();
    assert!(result.is_err());
}
