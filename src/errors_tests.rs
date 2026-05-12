use super::*;

#[test]
fn test_retryable_errors() {
    let rate_limited = RpcError::RateLimited {
        message: "too many requests".into(),
    };
    assert!(rate_limited.is_retryable());

    let timeout = RpcError::Timeout {
        message: "timed out".into(),
    };
    assert!(timeout.is_retryable());

    let connection = RpcError::ConnectionError {
        message: "connection refused".into(),
    };
    assert!(connection.is_retryable());

    let server = RpcError::TemporaryServerError {
        status: 503,
        message: "service unavailable".into(),
    };
    assert!(server.is_retryable());
}

#[test]
fn test_non_retryable_errors() {
    let simulation = RpcError::SimulationFailed {
        message: "simulation failed".into(),
    };
    assert!(!simulation.is_retryable());

    let invalid_tx = RpcError::InvalidTransaction {
        message: "bad tx".into(),
    };
    assert!(!invalid_tx.is_retryable());

    let unauthorized = RpcError::Unauthorized {
        message: "bad key".into(),
    };
    assert!(!unauthorized.is_retryable());

    let provider = RpcError::ProviderApiError {
        provider: "helius".into(),
        message: "error".into(),
    };
    assert!(!provider.is_retryable());

    let unsupported = RpcError::UnsupportedFeature {
        feature: "das".into(),
    };
    assert!(!unsupported.is_retryable());

    let ws = RpcError::WebSocketError {
        message: "ws error".into(),
    };
    assert!(!ws.is_retryable());

    // Transaction not found should NOT be retryable - it's expected state during polling
    let not_found = RpcError::TransactionNotFound {
        signature: "5vN...xyz".into(),
    };
    assert!(!not_found.is_retryable());
    assert!(!not_found.should_failover()); // Also not failoverable
}

#[test]
fn test_from_http_status() {
    let err = RpcError::from_http_status(429, "rate limited".into());
    assert!(matches!(err, RpcError::RateLimited { .. }));
    assert!(err.is_retryable());

    let err = RpcError::from_http_status(401, "unauthorized".into());
    assert!(matches!(err, RpcError::Unauthorized { .. }));
    assert!(!err.is_retryable());

    let err = RpcError::from_http_status(403, "forbidden".into());
    assert!(matches!(err, RpcError::Unauthorized { .. }));

    let err = RpcError::from_http_status(500, "internal server error".into());
    assert!(matches!(
        err,
        RpcError::TemporaryServerError { status: 500, .. }
    ));
    assert!(err.is_retryable());

    let err = RpcError::from_http_status(502, "bad gateway".into());
    assert!(matches!(
        err,
        RpcError::TemporaryServerError { status: 502, .. }
    ));

    let err = RpcError::from_http_status(400, "bad request".into());
    assert!(matches!(err, RpcError::ProviderApiError { .. }));
    assert!(!err.is_retryable());
}

#[test]
fn test_should_failover() {
    // Retryable errors are also failoverable
    assert!(RpcError::RateLimited { message: "".into() }.should_failover());
    assert!(RpcError::Timeout { message: "".into() }.should_failover());
    assert!(RpcError::ConnectionError { message: "".into() }.should_failover());
    assert!(
        RpcError::TemporaryServerError {
            status: 503,
            message: "".into()
        }
        .should_failover()
    );

    // Unauthorized: not retryable, but IS failoverable (other provider may have valid key)
    assert!(!RpcError::Unauthorized { message: "".into() }.is_retryable());
    assert!(RpcError::Unauthorized { message: "".into() }.should_failover());

    // Request-level errors: NOT failoverable (same request will fail everywhere)
    assert!(!RpcError::SimulationFailed { message: "".into() }.should_failover());
    assert!(!RpcError::InvalidTransaction { message: "".into() }.should_failover());
    assert!(!RpcError::UnsupportedFeature { feature: "".into() }.should_failover());
    assert!(
        !RpcError::TransactionNotFound {
            signature: "".into()
        }
        .should_failover()
    );
}

#[test]
fn test_transaction_not_found_detection() {
    use solana_client::client_error::{ClientError, ClientErrorKind};

    // Test various "not found" error message patterns
    let patterns = vec![
        "Transaction not found",
        "Could not find transaction",
        "transaction signature not found",
        "expected transaction but got null",
    ];

    for pattern in patterns {
        let err = ClientError::new_with_request(
            ClientErrorKind::RpcError(solana_client::rpc_request::RpcError::RpcRequestError(
                pattern.to_string(),
            )),
            solana_client::rpc_request::RpcRequest::GetTransaction,
        );

        // Should be detected as "transaction not found"
        assert!(
            RpcError::is_transaction_not_found_error(&err),
            "Failed to detect pattern: {}",
            pattern
        );

        // Should NOT be retryable
        assert!(
            !RpcError::is_solana_client_error_retryable(&err),
            "Incorrectly marked as retryable: {}",
            pattern
        );

        // Should NOT be failoverable
        assert!(
            !RpcError::is_solana_client_error_failoverable(&err),
            "Incorrectly marked as failoverable: {}",
            pattern
        );
    }

    // Test that other errors are NOT detected as "transaction not found"
    let other_err = ClientError::new_with_request(
        ClientErrorKind::RpcError(solana_client::rpc_request::RpcError::RpcRequestError(
            "Some other error".to_string(),
        )),
        solana_client::rpc_request::RpcRequest::GetBalance,
    );
    assert!(!RpcError::is_transaction_not_found_error(&other_err));
}

#[test]
fn test_error_display() {
    let err = RpcError::RateLimited {
        message: "too fast".into(),
    };
    assert_eq!(err.to_string(), "Rate limited: too fast");

    let err = RpcError::ProviderApiError {
        provider: "Helius".into(),
        message: "something broke".into(),
    };
    assert_eq!(
        err.to_string(),
        "Provider API error: Helius - something broke"
    );

    let err = RpcError::UnsupportedFeature {
        feature: "enhanced_transactions".into(),
    };
    assert_eq!(
        err.to_string(),
        "Feature not supported: enhanced_transactions"
    );

    let err = RpcError::TransactionNotFound {
        signature: "5vNQK...xyz".into(),
    };
    assert_eq!(err.to_string(), "Transaction not found: 5vNQK...xyz");
}
