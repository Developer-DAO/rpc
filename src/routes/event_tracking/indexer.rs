// This module could live as its own microservice
// For now, we can keep it here until proven we need to scale it.

use std::sync::Arc;
use std::time::Duration;
use sqlx::{Pool, Postgres};
use tokio::sync::{mpsc, RwLock};
use tokio::time::sleep;
use tracing::{error, info};
use crate::database::types::RELATIONAL_DATABASE;

#[derive(Debug, Clone)]
pub enum IndexerCommand {
    Start,
    Stop,
    Restart,
    Status,
    /// Run for n iterations then stop
    Iter(u8),
    /// Change tick rate in seconds
    ChangeTick(u16),
    Help,
}

/// Responses from the indexer
#[derive(Debug, Clone)]
pub enum IndexerResponse {
    Started,
    Stopped,
    Restarted,
    AlreadyRunning,
    NotRunning,
    Status(IndexerStatus),
    TickChanged(u16),
    Error(String),
    Help(String),
}

impl IndexerResponse {
    /// Get information about the Indexer and possible commands
    pub fn help() -> String {
        String::from(
            "
                Usage:
                    indexer <command>

                Commands:
                    start        Start the indexer service
                    stop         Stop the indexer service
                    restart      Restart the indexer service
                    status       Get current status of the indexer
                    iter <n>     Run indexer for n iterations (u8)
                    tick <n>     Change the tick rate to n seconds (u16)
                    help         Show this help message
                ")
    }
}

/// Shared state for the indexer
pub struct IndexerState {
    running: RwLock<bool>,
    tick_rate_seconds: RwLock<u16>,
    total_iterations: RwLock<u64>,
    contracts_indexed: RwLock<usize>,
    last_error: RwLock<Option<String>>,
}

impl IndexerState {
    pub fn new(starting_tick_rate:u16) -> Arc<Self> {
        Arc::new(Self {
            running: RwLock::new(false),
            tick_rate_seconds: RwLock::new(starting_tick_rate),
            total_iterations: RwLock::new(0),
            contracts_indexed: RwLock::new(0),
            last_error: RwLock::new(None),
        })
    }

    pub async fn status(&self) -> IndexerStatus {
        IndexerStatus {
            running: *self.running.read().await,
            tick_rate_seconds: *self.tick_rate_seconds.read().await,
            total_iterations: *self.total_iterations.read().await,
            contracts_indexed: *self.contracts_indexed.read().await,
            last_error: self.last_error.read().await.clone(),
        }
    }

    pub async fn set_running(&self, running: bool) {
        *self.running.write().await = running;
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    pub async fn set_tick_rate(&self, seconds: u16) {
        *self.tick_rate_seconds.write().await = seconds;
    }

    pub async fn get_tick_rate(&self) -> u16 {
        *self.tick_rate_seconds.read().await
    }

    pub async fn increment_iterations(&self) {
        *self.total_iterations.write().await += 1;
    }

    pub async fn set_contracts_indexed(&self, count: usize) {
        *self.contracts_indexed.write().await = count;
    }

    pub async fn set_error(&self, error: Option<String>) {
        *self.last_error.write().await = error;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IndexerError {
    #[error("Database not initialized")]
    DatabaseNotInitialized,

    #[error("Unsupported chain: {0}")]
    UnsupportedChain(String),

    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),

    #[error(transparent)]
    RpcError(#[from] alloy::transports::RpcError<alloy::transports::TransportErrorKind>),

    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    #[error(transparent)]
    EnvError(#[from] std::env::VarError),

    #[error(transparent)]
    HexError(#[from] alloy::primitives::hex::FromHexError),
}

/// Handle for controlling the indexer
pub struct IndexerHandle {
    command_tx: mpsc::UnboundedSender<IndexerCommand>,
    response_rx: Arc<RwLock<mpsc::UnboundedReceiver<IndexerResponse>>>,
    state: Arc<IndexerState>,
}

impl IndexerHandle {
    /// Send a command to the indexer
    pub async fn send_command(&self, cmd: IndexerCommand) -> Result<IndexerResponse, String> {
        self.command_tx
            .send(cmd)
            .map_err(|e| format!("Failed to send command: {}", e))?;

        // Wait for response with timeout
        let mut rx = self.response_rx.write().await;
        match tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
            Ok(Some(response)) => Ok(response),
            Ok(None) => Err("Indexer channel closed".to_string()),
            Err(_) => Err("Command timeout".to_string()),
        }
    }

    /// Get current status without sending a command
    pub async fn get_status(&self) -> IndexerStatus {
        self.state.status().await
    }
}



/// Current status of the indexer
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexerStatus {
    pub running: bool,
    pub tick_rate_seconds: u16,
    pub total_iterations: u64,
    pub contracts_indexed: usize,
    pub last_error: Option<String>,
}

/// Spawn the indexer with control channels
pub fn spawn_indexer(initial_tick_seconds: Option<u16>) -> IndexerHandle {
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (response_tx, response_rx) = mpsc::unbounded_channel();
    let state = IndexerState::new(initial_tick_seconds.unwrap_or(15)); // Default to 15 seconds

    let state_clone = state.clone();
    tokio::spawn(async move {
        run_indexer_loop(command_rx, response_tx, state_clone).await;
    });

    IndexerHandle {
        command_tx,
        response_rx: Arc::new(RwLock::new(response_rx)),
        state,
    }
}

/// Main indexer loop with command handling
async fn run_indexer_loop(
    mut command_rx: mpsc::UnboundedReceiver<IndexerCommand>,
    response_tx: mpsc::UnboundedSender<IndexerResponse>,
    state: Arc<IndexerState>,
) {
    info!("Indexer loop started");

    loop {
        // Check for commands (non-blocking)
        while let Ok(cmd) = command_rx.try_recv() {
            let response = handle_command(&cmd, &state).await;
            let _ = response_tx.send(response);
        }

        // Only run if status is "running"
        if state.is_running().await {
            if let Err(e) = indexer_tick(&state).await {
                let error_msg = format!("Indexer tick error: {}", e);
                error!("{}", error_msg);
                state.set_error(Some(error_msg)).await;
            } else {
                state.set_error(None).await;
            }
            state.increment_iterations().await;

            // Sleep for the configured tick rate
            let tick_rate = state.get_tick_rate().await;
            sleep(Duration::from_secs(tick_rate as u64)).await;
        } else {
            // If not running, just sleep briefly to avoid busy-waiting
            sleep(Duration::from_millis(500)).await;
        }
    }
}

/// Handle incoming commands
async fn handle_command(cmd: &IndexerCommand, state: &Arc<IndexerState>) -> IndexerResponse {
    match cmd {
        IndexerCommand::Start => {
            if state.is_running().await {
                IndexerResponse::AlreadyRunning
            } else {
                state.set_running(true).await;
                info!("Indexer started");
                IndexerResponse::Started
            }
        }
        IndexerCommand::Stop => {
            if !state.is_running().await {
                IndexerResponse::NotRunning
            } else {
                state.set_running(false).await;
                info!("Indexer stopped");
                IndexerResponse::Stopped
            }
        }
        IndexerCommand::Restart => {
            state.set_running(false).await;
            sleep(Duration::from_millis(100)).await;
            state.set_running(true).await;
            info!("Indexer restarted");
            IndexerResponse::Restarted
        }
        IndexerCommand::Status => {
            let status = state.status().await;
            IndexerResponse::Status(status)
        }
        IndexerCommand::Iter(n) => {
            // Run for N iterations then stop
            state.set_running(true).await;
            for _ in 0..*n {
                if let Err(e) = indexer_tick(state).await {
                    let error_msg = format!("Iteration error: {}", e);
                    state.set_error(Some(error_msg.clone())).await;
                    return IndexerResponse::Error(error_msg);
                }
                state.increment_iterations().await;
                sleep(Duration::from_secs(state.get_tick_rate().await as u64)).await;
            }
            state.set_running(false).await;
            IndexerResponse::Stopped
        }
        IndexerCommand::ChangeTick(seconds) => {
            state.set_tick_rate(*seconds).await;
            info!("Tick rate changed to {} seconds", seconds);
            IndexerResponse::TickChanged(*seconds)
        }
        IndexerCommand::Help => {
            IndexerResponse::Help(IndexerResponse::help())
        }
    }
}

/// Single iteration of the indexer
async fn indexer_tick(state: &Arc<IndexerState>) -> Result<(), IndexerError> {
    let db = RELATIONAL_DATABASE
        .get()
        .ok_or(IndexerError::DatabaseNotInitialized)?;

    // Get all unique (chain_id, contract_address) combinations
    let contracts = sqlx::query!(
        r#"
        SELECT DISTINCT chain_id, contract_address
        FROM event_subscriptions
        "#
    )
        .fetch_all(db)
        .await?;

    info!("Found {} unique contracts to index", contracts.len());
    state.set_contracts_indexed(contracts.len()).await;

    for contract in contracts {
        if let Err(e) = index_contract(db, &contract.chain_id, &contract.contract_address).await {
            error!(
                "Failed to index contract {} on chain {}: {}",
                contract.contract_address, contract.chain_id, e
            );
        }
    }

    Ok(())
}

async fn index_contract(db: &Pool<Postgres>, chain_id: &str, contract_address:&str ) -> Result<(), IndexerError> {
    todo!()
}
