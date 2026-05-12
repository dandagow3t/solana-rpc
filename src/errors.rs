use thiserror::Error;

#[derive(Error, Debug)]
pub enum RpcError {
    // Retryable (triggers failover)
    #[error("Rate limited: {message}")]
    RateLimited { message: String },

    #[error("Timeout: {message}")]
    Timeout { message: String },

    #[error("Connection error: {message}")]
    ConnectionError { message: String },

    #[error("Server error ({status}): {message}")]
    TemporaryServerError { status: u16, message: String },

    // Non-retryable, but failoverable (another provider may have a valid key)
    #[error("Unauthorized: {message}")]
    Unauthorized { message: String },

    // Non-retryable, non-failoverable (request itself is invalid)
    #[error("Simulation failed: {message}")]
    SimulationFailed { message: String },

    #[error("Invalid transaction: {message}")]
    InvalidTransaction { message: String },

    #[error("Transaction not found: {signature}")]
    TransactionNotFound { signature: String },

    // Provider-specific
    #[error("Provider API error: {provider} - {message}")]
    ProviderApiError { provider: String, message: String },

    #[error("Feature not supported: {feature}")]
    UnsupportedFeature { feature: String },

    // WebSocket
    #[error("WebSocket error: {message}")]
    WebSocketError { message: String },

    // Passthrough
    #[error("Solana client error: {0}")]
    SolanaClient(#[from] solana_client::client_error::ClientError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

impl RpcError {
    /// Returns true if this error is transient and the request should be retried
    /// on the **same** provider.
    pub fn is_retryable(&self) -> bool {
        match self {
            RpcError::RateLimited { .. }
            | RpcError::Timeout { .. }
            | RpcError::ConnectionError { .. }
            | RpcError::TemporaryServerError { .. } => true,
            RpcError::SolanaClient(err) => Self::is_solana_client_error_retryable(err),
            _ => false,
        }
    }

    /// Returns true if a **different** provider might succeed where this one failed.
    ///
    /// This is a superset of `is_retryable()`: everything retryable is also
    /// failoverable, plus errors like 401/403 (bad key on provider A doesn't
    /// mean provider B is bad).
    pub fn should_failover(&self) -> bool {
        match self {
            // Definitely not failoverable -- the request itself is invalid or transaction doesn't exist
            RpcError::SimulationFailed { .. }
            | RpcError::InvalidTransaction { .. }
            | RpcError::TransactionNotFound { .. }
            | RpcError::UnsupportedFeature { .. }
            | RpcError::Serialization(_) => false,
            // SolanaClient errors: inspect the inner error
            RpcError::SolanaClient(err) => Self::is_solana_client_error_failoverable(err),
            // Everything else (rate limits, timeouts, connection errors, 5xx,
            // unauthorized, provider errors, websocket, http) -- try the other provider
            _ => true,
        }
    }

    /// Inspect a `solana_client::ClientError` for retryable HTTP conditions.
    fn is_solana_client_error_retryable(err: &solana_client::client_error::ClientError) -> bool {
        use solana_client::client_error::ClientErrorKind;

        // Check if this is a "transaction not found" error - NOT retryable
        if Self::is_transaction_not_found_error(err) {
            return false;
        }

        match err.kind() {
            ClientErrorKind::Reqwest(reqwest_err) => {
                if reqwest_err.is_timeout() || reqwest_err.is_connect() {
                    return true;
                }
                matches!(
                    reqwest_err.status().map(|s| s.as_u16()),
                    Some(429 | 500..=599)
                )
            }
            ClientErrorKind::Io(_) => true,
            _ => false,
        }
    }

    /// Inspect a `solana_client::ClientError` for failover-worthy conditions.
    fn is_solana_client_error_failoverable(err: &solana_client::client_error::ClientError) -> bool {
        use solana_client::client_error::ClientErrorKind;

        // Check if this is a "transaction not found" error - NOT failoverable
        // (other providers won't have it either if it doesn't exist)
        if Self::is_transaction_not_found_error(err) {
            return false;
        }

        match err.kind() {
            // Transaction/signing errors are request-level issues, not provider issues
            ClientErrorKind::TransactionError(_) | ClientErrorKind::SigningError(_) => false,
            // Reqwest errors: timeout, connect, HTTP status -- all worth trying another provider
            ClientErrorKind::Reqwest(_) | ClientErrorKind::Io(_) => true,
            // RPC-level errors (e.g. method not found, invalid params): could differ by provider
            ClientErrorKind::RpcError(_) => true,
            _ => true,
        }
    }

    /// Check if a solana_client error indicates a "transaction not found" scenario.
    /// This happens when querying a transaction that doesn't exist or hasn't landed yet.
    fn is_transaction_not_found_error(err: &solana_client::client_error::ClientError) -> bool {
        let err_msg = err.to_string().to_lowercase();

        // Common patterns in Solana RPC "not found" errors
        err_msg.contains("transaction not found")
            || err_msg.contains("could not find transaction")
            || err_msg.contains("transaction signature not found")
            // RPC returns null for non-existent transactions, which gets parsed as an error
            || err_msg.contains("expected transaction")
    }

    /// Classify a reqwest error into an RpcError variant.
    pub fn from_reqwest(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            RpcError::Timeout {
                message: err.to_string(),
            }
        } else if err.is_connect() {
            RpcError::ConnectionError {
                message: err.to_string(),
            }
        } else if let Some(status) = err.status() {
            match status.as_u16() {
                429 => RpcError::RateLimited {
                    message: err.to_string(),
                },
                401 | 403 => RpcError::Unauthorized {
                    message: err.to_string(),
                },
                500..=599 => RpcError::TemporaryServerError {
                    status: status.as_u16(),
                    message: err.to_string(),
                },
                _ => RpcError::Http(err),
            }
        } else {
            RpcError::Http(err)
        }
    }

    /// Classify an HTTP status code + body into an RpcError variant.
    pub fn from_http_status(status: u16, body: String) -> Self {
        match status {
            429 => RpcError::RateLimited { message: body },
            401 | 403 => RpcError::Unauthorized { message: body },
            500..=599 => RpcError::TemporaryServerError {
                status,
                message: body,
            },
            _ => RpcError::ProviderApiError {
                provider: "unknown".to_string(),
                message: format!("HTTP {status}: {body}"),
            },
        }
    }
}

#[cfg(test)]
#[path = "errors_tests.rs"]
mod errors_tests;
