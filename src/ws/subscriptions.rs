use super::client::WsClient;
use super::types::{SubscriptionHandle, SubscriptionType};
use anyhow::Result;
use solana_sdk::commitment_config::CommitmentConfig;

impl WsClient {
    /// Subscribe to log messages mentioning specified program IDs.
    ///
    /// Returns a `SubscriptionHandle` whose `receiver` yields `WsNotification`
    /// for each matching log entry.
    pub async fn logs_subscribe(
        &self,
        mentions: Vec<String>,
        commitment: Option<CommitmentConfig>,
    ) -> Result<SubscriptionHandle> {
        let filter = serde_json::json!({
            "mentions": mentions,
        });

        let mut params = vec![filter];
        if let Some(c) = commitment {
            params.push(serde_json::json!({
                "commitment": commitment_to_str(c),
            }));
        }

        self.subscribe(
            "logsSubscribe",
            serde_json::Value::Array(params),
            SubscriptionType::Logs,
        )
        .await
    }

    /// Subscribe to changes on a specific account.
    ///
    /// Returns a `SubscriptionHandle` whose `receiver` yields `WsNotification`
    /// for each account update.
    pub async fn account_subscribe(
        &self,
        pubkey: &str,
        commitment: Option<CommitmentConfig>,
    ) -> Result<SubscriptionHandle> {
        let mut params = vec![serde_json::json!(pubkey)];
        if let Some(c) = commitment {
            params.push(serde_json::json!({
                "encoding": "jsonParsed",
                "commitment": commitment_to_str(c),
            }));
        } else {
            params.push(serde_json::json!({
                "encoding": "jsonParsed",
            }));
        }

        self.subscribe(
            "accountSubscribe",
            serde_json::Value::Array(params),
            SubscriptionType::Account,
        )
        .await
    }

    /// Subscribe to signature confirmation status.
    ///
    /// Returns a `SubscriptionHandle` that yields a single `WsNotification`
    /// when the signature reaches the requested commitment level.
    pub async fn signature_subscribe(
        &self,
        signature: &str,
        commitment: Option<CommitmentConfig>,
    ) -> Result<SubscriptionHandle> {
        let mut params = vec![serde_json::json!(signature)];
        if let Some(c) = commitment {
            params.push(serde_json::json!({
                "commitment": commitment_to_str(c),
            }));
        }

        self.subscribe(
            "signatureSubscribe",
            serde_json::Value::Array(params),
            SubscriptionType::Signature,
        )
        .await
    }
}

fn commitment_to_str(c: CommitmentConfig) -> &'static str {
    match c.commitment {
        solana_sdk::commitment_config::CommitmentLevel::Processed => "processed",
        solana_sdk::commitment_config::CommitmentLevel::Confirmed => "confirmed",
        solana_sdk::commitment_config::CommitmentLevel::Finalized => "finalized",
    }
}
