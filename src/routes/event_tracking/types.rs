use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use sqlx::types::{JsonValue};
use uuid::Uuid;

/// Represents a user's subscription to track events from a smart contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSubscription {
    pub id: Uuid,
    pub user_email: String,
    pub tracked_event_id: Uuid,
    pub created_at: OffsetDateTime,
}

/// Represents an actual event that was captured from the blockchain
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TrackedEvent {
    pub id: Uuid,
    pub chain_id: String,
    pub contract_address: String,
    pub block_number: i64,
    pub tx_hash: String,
    pub log_index: i32,
    pub event_signature: String,
    pub event_data: JsonValue, // JSONB from Postgres
    pub block_timestamp: OffsetDateTime,
    pub confirmed: bool,
    pub created_at: OffsetDateTime,
}

/// Tracks the last synced block for each contract we're monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSyncState {
    pub chain_id: String,
    pub contract_address: String,
    pub last_synced_block: i64,
    pub last_synced_at: OffsetDateTime,
}

/// Request payload for subscribing to events
#[derive(Debug, Deserialize)]
pub struct SubscribeRequest {
    pub customer_email: String,
    pub chain_id: String,
    pub contract_address: String,
    /// List of event signatures to track. If empty/None, track all events.
    pub event_signatures: Option<Vec<String>>,
}

/// Request payload for unsubscribing from events
#[derive(Debug, Deserialize)]
pub struct UnsubscribeRequest {
    pub customer_email: String,
    pub chain_id: Option<String>,
    pub contract_address: Option<String>,
    pub event_signature: Option<Vec<String>>,
}

/// Query parameters for fetching events
#[derive(Debug, Deserialize)]
pub struct GetEventsQuery {
    pub customer_email: String,
    pub chain_id: Option<String>,
    pub contract_address: Option<String>,
    pub event_signature: Option<String>,
    pub confirmed_only: Option<bool>,
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

/// Response for event queries
#[derive(Debug, Serialize)]
pub struct EventsResponse {
    pub events: Vec<TrackedEvent>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

