use crate::routes::relayer::types::PoktChains;
use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket};
use axum::extract::{Path, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use http::StatusCode;
use thiserror::Error;
use tokio::{select, sync::mpsc};
use tokio_tungstenite::tungstenite::ClientRequestBuilder;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{
    connect_async_tls_with_config, tungstenite::Message as TungsteniteMessage,
};
use tracing::{debug, info, warn};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    path: Path<[String; 2]>,
) -> Result<axum::response::Response, WsError> {
    let path = path
        .first()
        .ok_or(WsError::MissingRoute)?
        .parse::<PoktChains>()
        .map_err(|_| WsError::InvalidRoute)?;

    let ws = ws.max_message_size(1024 * 1024);
    let res = ws.on_upgrade(async move |user_socket| {
        let (user_tx, mut user_rv): (SplitSink<WebSocket, Message>, SplitStream<WebSocket>) =
            user_socket.split();
        // CONSIDERATION: timeout await call
        let subscription = user_rv.next().await;
        let Some(Ok(Message::Text(sub_info))) = subscription else {
            warn!("Unexpected message received.");
            return;
        };
        let (shutdown_tx, shutdown_rx): (
            mpsc::UnboundedSender<Command>,
            mpsc::UnboundedReceiver<Command>,
        ) = mpsc::unbounded_channel();
        let (cleanup_tx, cleanup_rx): (
            mpsc::UnboundedSender<Command>,
            mpsc::UnboundedReceiver<Command>,
        ) = mpsc::unbounded_channel();
        handle_user_msgs(cleanup_rx, user_rv, shutdown_tx).await;
        handle_ws_conn(path, shutdown_rx, cleanup_tx, sub_info, user_tx).await;
    });
    Ok(res)
}

pub async fn handle_user_msgs(
    mut cleanup_rx: mpsc::UnboundedReceiver<Command>,
    mut user_rv: SplitStream<WebSocket>,
    shutdown_tx: mpsc::UnboundedSender<Command>,
) {
    tokio::spawn(async move {
        // max wait time?
        loop {
            select! {
                Some(Command::Kill) = cleanup_rx.recv() => {
                    warn!("Exiting user msg handler");
                    return
                },
                Some(Ok(msg)) = user_rv.next() => {
                    match msg {
                        Message::Text(_utf8_bytes) => {},
                        Message::Binary(_bytes) => {},
                        Message::Ping(_bytes) => shutdown_tx.send(Command::Pong).unwrap(),
                        Message::Pong(_bytes) => shutdown_tx.send(Command::Ping).unwrap(),
                        Message::Close(_close_frame) => {
                            shutdown_tx.send(Command::Kill).unwrap();
                            return;
                        },
                    }
                }
                // prevents a panic
                else => {
                    debug!("User channel and ws channel dropped");
                    return;
                }
            }
        }
    });
}

#[tracing::instrument]
pub async fn handle_ws_conn(
    path: PoktChains,
    mut shutdown_rx: mpsc::UnboundedReceiver<Command>,
    cleanup_tx: mpsc::UnboundedSender<Command>,
    sub_info: Utf8Bytes,
    mut user_tx: SplitSink<WebSocket, Message>,
) {
    tokio::spawn(async move {
        'node_reconnect: loop {
            let request = if cfg!(feature = "dev") {
                let url = dotenvy::var("SEPOLIA_WS").unwrap().parse().unwrap();
                ClientRequestBuilder::new(url)
            } else {
                ClientRequestBuilder::new("ws://localhost:3069/v1".parse().unwrap())
                    .with_header("Target-Service-Id", String::from(path.id()))
            };

            let config = WebSocketConfig::default().max_message_size(Some(16 * 1024 * 1024));
            let (node_socket, _res) =
                match connect_async_tls_with_config(request, Some(config), false, None).await {
                    Ok((node_socket, _res)) => (node_socket, _res),
                    Err(e) => {
                        tracing::error!("Failed to connect to websocket: {e}");
                        if let Err(e) = cleanup_tx.send(Command::Kill) {
                            debug!("Failed to clean up user channel: {e}");
                        }
                        break 'node_reconnect;
                    }
                };

            let (mut node_tx, mut node_rv) = node_socket.split();

            node_tx
                .send(TungsteniteMessage::Text(
                    tokio_tungstenite::tungstenite::Utf8Bytes::from(sub_info.as_str()),
                ))
                .await
                .unwrap();

            loop {
                select! {
                        Some(cmd) = shutdown_rx.recv() => {
                            match cmd {
                                // graceful closure
                                Command::Kill => {
                                    let closure = axum::extract::ws::Message::Close(
                                    Some(
                                        axum::extract::ws::CloseFrame {
                                            code: 1000,
                                            reason: axum::extract::ws::Utf8Bytes::from_static("Graceful shutdown (user sent close frame)")
                                        }
                                    ));
                                    user_tx.send(closure).await.unwrap();
                                    info!("Graceful shutdown");
                                },
                                Command::Pong => user_tx.send(axum::extract::ws::Message::Ping(axum::body::Bytes::new())).await.unwrap(),
                                Command::Ping => user_tx.send(axum::extract::ws::Message::Pong(axum::body::Bytes::new())).await.unwrap(),
                            }
                        }
                        Some(Ok(msg)) = node_rv.next() => {
                            if let Some(m) = convert(msg) {
                                match m {
                                    Message::Text(_) => {
                                        if let Err(e) = user_tx.send(m).await {
                                            node_tx.send(TungsteniteMessage::Close(None)).await.unwrap();
                                            cleanup_tx.send(Command::Kill).unwrap();
                                            warn!("Failed to relay msg to user from node: {e}");
                                            return;
                                        }
                                    }
                                    Message::Close(_close_frame) => {
                                        // log node info and close frame leading to ws closure
                                        // handle close of Node's WS connection
                                        warn!("Lost connection to node. Reconnecting ...");
                                        continue 'node_reconnect;
                                    }
                                    _ => {}
                            }
                        }
                    }
                }
            }
        }
        //        warn!("Exiting connection bridge");
    });
}

fn convert(msg: TungsteniteMessage) -> Option<Message> {
    match msg {
        TungsteniteMessage::Text(utf8_bytes) => Some(Message::Text({
            axum::extract::ws::Utf8Bytes::from(utf8_bytes.as_str())
        })),
        TungsteniteMessage::Binary(bytes) => Some(Message::Binary(bytes)),
        TungsteniteMessage::Ping(bytes) => Some(Message::Ping(bytes)),
        TungsteniteMessage::Pong(bytes) => Some(Message::Pong(bytes)),
        TungsteniteMessage::Close(close_frame) => Some(Message::Close({
            match close_frame {
                Some(reason) => Some({
                    info!("WS Connection Closed: {}", reason);
                    CloseFrame {
                        code: reason.code.into(),
                        reason: axum::extract::ws::Utf8Bytes::from(reason.reason.as_str()),
                    }
                }),
                None => None,
            }
        })),
        TungsteniteMessage::Frame(_) => None,
    }
}

#[derive(Debug, Error)]
pub enum WsError {
    #[error("Route not present")]
    MissingRoute,
    #[error("Route unsupported or malformed")]
    InvalidRoute,
}

impl IntoResponse for WsError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::BAD_REQUEST, self.to_string()).into_response()
    }
}

pub enum Command {
    Kill,
    Pong,
    Ping,
}
