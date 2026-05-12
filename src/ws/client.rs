use crate::config::RpcConfig;
use crate::errors::RpcError;
use crate::ws::types::{
    SubscriptionHandle, SubscriptionType, WsJsonRpcNotification, WsNotification,
    WsSubscribeResponse,
};
use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::{Notify, RwLock, mpsc};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

/// Default ping interval for keepalive.
const PING_INTERVAL: Duration = Duration::from_secs(30);

/// Maximum reconnect delay.
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(30);

/// Base reconnect delay.
const BASE_RECONNECT_DELAY: Duration = Duration::from_millis(500);

/// WebSocket client with auto-reconnect and subscription management.
pub struct WsClient {
    ws_url: String,
    subscriptions: Arc<RwLock<HashMap<u64, mpsc::UnboundedSender<WsNotification>>>>,
    pending_subscribes:
        Arc<RwLock<HashMap<u64, tokio::sync::oneshot::Sender<Result<u64, String>>>>>,
    write_tx: Arc<RwLock<Option<mpsc::UnboundedSender<Message>>>>,
    next_id: Arc<AtomicU64>,
    connected: Arc<AtomicBool>,
    subscription_params: Arc<RwLock<Vec<(String, serde_json::Value)>>>,
    reconnect_notify: Arc<Notify>,
}

impl WsClient {
    /// Connect to the WebSocket server and start the read/write/ping loops.
    pub async fn connect(config: &RpcConfig) -> Result<Self> {
        let ws_url = config
            .ws_url
            .as_ref()
            .context("No WebSocket URL configured")?
            .clone();

        let client = Self {
            ws_url,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            pending_subscribes: Arc::new(RwLock::new(HashMap::new())),
            write_tx: Arc::new(RwLock::new(None)),
            next_id: Arc::new(AtomicU64::new(1)),
            connected: Arc::new(AtomicBool::new(false)),
            subscription_params: Arc::new(RwLock::new(Vec::new())),
            reconnect_notify: Arc::new(Notify::new()),
        };

        client.establish_connection().await?;
        client.spawn_reconnect_watcher();
        Ok(client)
    }

    /// Establish a WebSocket connection and spawn read/write/ping loops.
    async fn establish_connection(&self) -> Result<()> {
        let (ws_stream, _) = connect_async(&self.ws_url)
            .await
            .context("Failed to connect to WebSocket")?;

        info!(url = %self.ws_url, "WebSocket connected");
        self.connected.store(true, Ordering::SeqCst);

        let (write, read) = ws_stream.split();

        let (write_tx, write_rx) = mpsc::unbounded_channel::<Message>();
        {
            let mut tx = self.write_tx.write().await;
            *tx = Some(write_tx);
        }

        // Spawn write loop
        let connected = self.connected.clone();
        tokio::spawn(async move {
            let mut write = write;
            let mut write_rx = write_rx;
            while let Some(msg) = write_rx.recv().await {
                if write.send(msg).await.is_err() {
                    connected.store(false, Ordering::SeqCst);
                    break;
                }
            }
        });

        // Spawn read loop
        let subscriptions = self.subscriptions.clone();
        let pending = self.pending_subscribes.clone();
        let connected = self.connected.clone();
        let write_tx_arc = self.write_tx.clone();
        let reconnect_notify = self.reconnect_notify.clone();

        tokio::spawn(async move {
            let mut read = read;
            loop {
                match read.next().await {
                    Some(Ok(Message::Text(text))) => {
                        Self::handle_message(&text, &subscriptions, &pending).await;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let tx = write_tx_arc.read().await;
                        if let Some(tx) = tx.as_ref() {
                            let _ = tx.send(Message::Pong(data));
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        warn!("WebSocket disconnected");
                        connected.store(false, Ordering::SeqCst);
                        reconnect_notify.notify_one();
                        break;
                    }
                    Some(Err(e)) => {
                        error!(error = %e, "WebSocket read error");
                        connected.store(false, Ordering::SeqCst);
                        reconnect_notify.notify_one();
                        break;
                    }
                    _ => {}
                }
            }
        });

        // Spawn ping loop
        let write_tx = self.write_tx.clone();
        let connected = self.connected.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(PING_INTERVAL).await;
                if !connected.load(Ordering::SeqCst) {
                    break;
                }
                let tx = write_tx.read().await;
                if let Some(tx) = tx.as_ref() {
                    if tx.send(Message::Ping(vec![].into())).is_err() {
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Spawn a background task that watches for reconnect notifications
    /// and re-establishes the connection with exponential backoff.
    fn spawn_reconnect_watcher(&self) {
        let ws_url = self.ws_url.clone();
        let subscriptions = self.subscriptions.clone();
        let pending_subscribes = self.pending_subscribes.clone();
        let write_tx = self.write_tx.clone();
        let next_id = self.next_id.clone();
        let connected = self.connected.clone();
        let subscription_params = self.subscription_params.clone();
        let reconnect_notify = self.reconnect_notify.clone();

        tokio::spawn(async move {
            loop {
                // Wait for a reconnect signal
                reconnect_notify.notified().await;

                if connected.load(Ordering::SeqCst) {
                    continue;
                }

                let mut delay = BASE_RECONNECT_DELAY;
                loop {
                    tokio::time::sleep(delay).await;
                    info!(url = %ws_url, "Attempting WebSocket reconnect");

                    match connect_async(&ws_url).await {
                        Ok((ws_stream, _)) => {
                            info!(url = %ws_url, "WebSocket reconnected");
                            connected.store(true, Ordering::SeqCst);

                            let (write, read) = ws_stream.split();
                            let (new_write_tx, write_rx) = mpsc::unbounded_channel::<Message>();

                            {
                                let mut tx = write_tx.write().await;
                                *tx = Some(new_write_tx.clone());
                            }

                            // Spawn write loop
                            let conn = connected.clone();
                            tokio::spawn(async move {
                                let mut write = write;
                                let mut write_rx = write_rx;
                                while let Some(msg) = write_rx.recv().await {
                                    if write.send(msg).await.is_err() {
                                        conn.store(false, Ordering::SeqCst);
                                        break;
                                    }
                                }
                            });

                            // Resubscribe
                            let params = subscription_params.read().await;
                            for (method, param) in params.iter() {
                                let id = next_id.fetch_add(1, Ordering::SeqCst);
                                let request = serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "method": method,
                                    "params": param,
                                });
                                if let Ok(text) = serde_json::to_string(&request) {
                                    let _ = new_write_tx.send(Message::Text(text.into()));
                                }
                            }

                            // Spawn read loop
                            let subs = subscriptions.clone();
                            let pend = pending_subscribes.clone();
                            let conn = connected.clone();
                            let wt = write_tx.clone();
                            let rn = reconnect_notify.clone();

                            tokio::spawn(async move {
                                let mut read = read;
                                loop {
                                    match read.next().await {
                                        Some(Ok(Message::Text(text))) => {
                                            WsClient::handle_message(&text, &subs, &pend).await;
                                        }
                                        Some(Ok(Message::Ping(data))) => {
                                            let tx = wt.read().await;
                                            if let Some(tx) = tx.as_ref() {
                                                let _ = tx.send(Message::Pong(data));
                                            }
                                        }
                                        Some(Ok(Message::Close(_))) | None | Some(Err(_)) => {
                                            conn.store(false, Ordering::SeqCst);
                                            rn.notify_one();
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                            });

                            // Spawn ping loop
                            let conn = connected.clone();
                            let wt = write_tx.clone();
                            tokio::spawn(async move {
                                loop {
                                    tokio::time::sleep(PING_INTERVAL).await;
                                    if !conn.load(Ordering::SeqCst) {
                                        break;
                                    }
                                    let tx = wt.read().await;
                                    if let Some(tx) = tx.as_ref() {
                                        if tx.send(Message::Ping(vec![].into())).is_err() {
                                            break;
                                        }
                                    }
                                }
                            });

                            break; // Successfully reconnected, go back to waiting
                        }
                        Err(e) => {
                            warn!(
                                error = %e,
                                delay_ms = delay.as_millis() as u64,
                                "Reconnect failed, retrying"
                            );
                            delay = (delay * 2).min(MAX_RECONNECT_DELAY);
                        }
                    }
                }
            }
        });
    }

    async fn handle_message(
        text: &str,
        subscriptions: &Arc<RwLock<HashMap<u64, mpsc::UnboundedSender<WsNotification>>>>,
        pending: &Arc<RwLock<HashMap<u64, tokio::sync::oneshot::Sender<Result<u64, String>>>>>,
    ) {
        // Try to parse as a subscribe response first
        if let Ok(sub_response) = serde_json::from_str::<WsSubscribeResponse>(text) {
            let mut pending = pending.write().await;
            if let Some(sender) = pending.remove(&sub_response.id) {
                if let Some(err) = sub_response.error {
                    let _ = sender.send(Err(err.message));
                } else if let Some(sub_id) = sub_response.result {
                    let _ = sender.send(Ok(sub_id));
                }
            }
            return;
        }

        // Try to parse as a notification
        if let Ok(notification) = serde_json::from_str::<WsJsonRpcNotification>(text) {
            let subs = subscriptions.read().await;
            if let Some(sender) = subs.get(&notification.params.subscription) {
                let _ = sender.send(WsNotification {
                    subscription: notification.params.subscription,
                    result: notification.params.result,
                });
            } else {
                debug!(
                    subscription_id = notification.params.subscription,
                    "Received notification for unknown subscription"
                );
            }
            return;
        }

        debug!(message = text, "Unhandled WebSocket message");
    }

    /// Send a subscribe request and return a handle for receiving notifications.
    pub(crate) async fn subscribe(
        &self,
        method: &str,
        params: serde_json::Value,
        subscription_type: SubscriptionType,
    ) -> Result<SubscriptionHandle> {
        if !self.connected.load(Ordering::SeqCst) {
            bail!(RpcError::WebSocketError {
                message: "WebSocket not connected".to_string(),
            });
        }

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        // Set up oneshot for subscribe response
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        {
            let mut pending = self.pending_subscribes.write().await;
            pending.insert(id, resp_tx);
        }

        // Store params for resubscription on reconnect
        {
            let mut sub_params = self.subscription_params.write().await;
            sub_params.push((method.to_string(), params));
        }

        // Send the subscribe request
        let msg =
            serde_json::to_string(&request).context("Failed to serialize subscribe request")?;
        {
            let tx = self.write_tx.read().await;
            let tx = tx
                .as_ref()
                .context("WebSocket write channel not available")?;
            tx.send(Message::Text(msg.into()))
                .map_err(|e| anyhow::anyhow!("Failed to send subscribe request: {e}"))?;
        }

        // Wait for the subscribe response
        let subscription_id = tokio::time::timeout(Duration::from_secs(10), resp_rx)
            .await
            .context("Subscribe request timed out")?
            .context("Subscribe response channel closed")?
            .map_err(|e| anyhow::anyhow!("Subscribe error: {e}"))?;

        // Create notification channel
        let (notif_tx, notif_rx) = mpsc::unbounded_channel();
        {
            let mut subs = self.subscriptions.write().await;
            subs.insert(subscription_id, notif_tx);
        }

        Ok(SubscriptionHandle {
            receiver: notif_rx,
            subscription_id,
            subscription_type,
        })
    }

    /// Check if the client is currently connected.
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;
