use super::*;
use crate::types::PriorityLevel;
use httpmock::prelude::*;

#[tokio::test]
async fn test_get_priority_fee_estimate_with_levels() {
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
                        "priorityFeeLevels": {
                            "min": 0.0,
                            "low": 100.0,
                            "medium": 10000.0,
                            "high": 50000.0,
                            "veryHigh": 100000.0,
                            "unsafeMax": 500000.0
                        }
                    }
                }"#,
            );
    });

    let provider = AlchemyProvider::new(server.url("/"), reqwest::Client::new());

    let request = PriorityFeeRequest {
        account_keys: vec!["JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string()],
        priority_level: None,
    };

    let result = provider.get_priority_fee_estimate(request).await;
    mock.assert();

    let estimate = result.unwrap();
    // When levels are returned, we use medium as the default estimate
    assert_eq!(estimate.priority_fee, 10000.0);

    let levels = estimate.priority_fee_levels.unwrap();
    assert_eq!(levels.min, 0.0);
    assert_eq!(levels.low, 100.0);
    assert_eq!(levels.medium, 10000.0);
    assert_eq!(levels.high, 50000.0);
    assert_eq!(levels.very_high, 100000.0);
    assert_eq!(levels.unsafe_max, 500000.0);
}

#[tokio::test]
async fn test_get_priority_fee_estimate_with_specific_level() {
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
                        "priorityFee": 50000
                    }
                }"#,
            );
    });

    let provider = AlchemyProvider::new(server.url("/"), reqwest::Client::new());

    let request = PriorityFeeRequest {
        account_keys: vec!["JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string()],
        priority_level: Some(PriorityLevel::High),
    };

    let result = provider.get_priority_fee_estimate(request).await;
    mock.assert();

    let estimate = result.unwrap();
    assert_eq!(estimate.priority_fee, 50000.0);
    // When a specific level is requested, no levels are returned
    assert!(estimate.priority_fee_levels.is_none());
}

#[tokio::test]
async fn test_get_priority_fee_estimate_rpc_error() {
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
                        "code": -32600,
                        "message": "Invalid request"
                    }
                }"#,
            );
    });

    let provider = AlchemyProvider::new(server.url("/"), reqwest::Client::new());

    let request = PriorityFeeRequest {
        account_keys: vec![],
        priority_level: None,
    };

    let result = provider.get_priority_fee_estimate(request).await;
    mock.assert();
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_priority_fee_estimate_http_error() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(POST).path("/");
        then.status(429).body("rate limited");
    });

    let provider = AlchemyProvider::new(server.url("/"), reqwest::Client::new());

    let request = PriorityFeeRequest {
        account_keys: vec!["test".to_string()],
        priority_level: None,
    };

    let result = provider.get_priority_fee_estimate(request).await;
    mock.assert();
    assert!(result.is_err());
}
