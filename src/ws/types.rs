use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Types of subscriptions available via WebSocket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionType {
    /// Subscribe to logs mentioning specific program IDs.
    Logs,
    /// Subscribe to account changes.
    Account,
    /// Subscribe to signature confirmations.
    Signature,
}

impl std::fmt::Display for SubscriptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscriptionType::Logs => write!(f, "logs"),
            SubscriptionType::Account => write!(f, "account"),
            SubscriptionType::Signature => write!(f, "signature"),
        }
    }
}

/// A notification received from a WebSocket subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsNotification {
    /// The subscription ID this notification belongs to.
    pub subscription: u64,
    /// The raw JSON value of the notification result.
    pub result: serde_json::Value,
}

/// Handle to an active subscription. Drop to unsubscribe.
pub struct SubscriptionHandle {
    /// Receiver for notifications from this subscription.
    pub receiver: mpsc::UnboundedReceiver<WsNotification>,
    /// The subscription ID assigned by the server.
    pub subscription_id: u64,
    /// Type of subscription.
    pub subscription_type: SubscriptionType,
}

/// JSON-RPC notification message from the WebSocket server.
#[derive(Debug, Deserialize)]
pub struct WsJsonRpcNotification {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub method: String,
    pub params: WsJsonRpcNotificationParams,
}

#[derive(Debug, Deserialize)]
pub struct WsJsonRpcNotificationParams {
    pub subscription: u64,
    pub result: serde_json::Value,
}

/// JSON-RPC subscribe response.
#[derive(Debug, Deserialize)]
pub struct WsSubscribeResponse {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<u64>,
    pub error: Option<WsRpcError>,
}

#[derive(Debug, Deserialize)]
pub struct WsRpcError {
    pub code: i64,
    pub message: String,
}
