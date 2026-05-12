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
                        "priorityFeeEstimate": 1000.0,
                        "priorityFeeLevels": {
                            "min": 0.0,
                            "low": 100.0,
                            "medium": 1000.0,
                            "high": 5000.0,
                            "veryHigh": 10000.0,
                            "unsafeMax": 50000.0
                        }
                    }
                }"#,
            );
    });

    let provider = HeliusProvider::new(server.url("/"), reqwest::Client::new());

    let request = PriorityFeeRequest {
        account_keys: vec!["JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string()],
        priority_level: None,
    };

    let result = provider.get_priority_fee_estimate(request).await;
    mock.assert();

    let estimate = result.unwrap();
    assert_eq!(estimate.priority_fee, 1000.0);

    let levels = estimate.priority_fee_levels.unwrap();
    assert_eq!(levels.min, 0.0);
    assert_eq!(levels.low, 100.0);
    assert_eq!(levels.medium, 1000.0);
    assert_eq!(levels.high, 5000.0);
    assert_eq!(levels.very_high, 10000.0);
    assert_eq!(levels.unsafe_max, 50000.0);
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
                        "priorityFeeEstimate": 5000.0
                    }
                }"#,
            );
    });

    let provider = HeliusProvider::new(server.url("/"), reqwest::Client::new());

    let request = PriorityFeeRequest {
        account_keys: vec!["JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string()],
        priority_level: Some(PriorityLevel::High),
    };

    let result = provider.get_priority_fee_estimate(request).await;
    mock.assert();

    let estimate = result.unwrap();
    assert_eq!(estimate.priority_fee, 5000.0);
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
                        "code": -32602,
                        "message": "Invalid params: no account keys provided"
                    }
                }"#,
            );
    });

    let provider = HeliusProvider::new(server.url("/"), reqwest::Client::new());

    let request = PriorityFeeRequest {
        account_keys: vec![],
        priority_level: None,
    };

    let result = provider.get_priority_fee_estimate(request).await;
    mock.assert();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid params"));
}

#[tokio::test]
async fn test_get_priority_fee_estimate_http_error() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(POST).path("/");
        then.status(429).body("rate limited");
    });

    let provider = HeliusProvider::new(server.url("/"), reqwest::Client::new());

    let request = PriorityFeeRequest {
        account_keys: vec!["test".to_string()],
        priority_level: None,
    };

    let result = provider.get_priority_fee_estimate(request).await;
    mock.assert();
    assert!(result.is_err());
}
