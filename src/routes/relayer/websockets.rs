use crate::routes::relayer::types::PoktChains;
use axum::body::Bytes;
use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket};
use axum::extract::{Path, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use http::StatusCode;
use thiserror::Error;
use tokio::{select, sync::mpsc};
use tokio_tungstenite::tungstenite::ClientRequestBuilder;
use tokio_tungstenite::{
    connect_async_tls_with_config, tungstenite::Message as TungsteniteMessage,
};
use tracing::{info, warn};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    path: Path<[String; 2]>,
) -> Result<axum::response::Response, WsError> {
    let path = path
        .first()
        .ok_or(WsError::MissingRoute)?
        .parse::<PoktChains>()
        .map_err(|_| WsError::InvalidRoute)?;

    let res = ws.on_upgrade(async move |user_socket| {
        let (user_tx, mut user_rv) = user_socket.split();
        // CONSIDERATION: timeout await call
        let subscription = user_rv.next().await;
        let Some(Ok(Message::Text(sub_info))) = subscription else {
            // handle error case
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
        select! {
            Some(Command::Kill) = cleanup_rx.recv() => {
            },
            Some(Ok(Message::Close(_close_frame))) = user_rv.next() => {
                shutdown_tx.send(Command::Kill).unwrap();
            }
        }

        warn!("Exiting user msg handler");
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
        let mut reconnect_count: u8 = 0;
        'reconnect: loop {
            if reconnect_count >= 3 {
                cleanup_tx.send(Command::Kill).unwrap();
                break 'reconnect;
            }
            let request = ClientRequestBuilder::new("ws://localhost:3070/v1".parse().unwrap())
                .with_header("Target-Service-Id", String::from(path.id()));

            let Ok((node_socket, _res)) =
                connect_async_tls_with_config(request, None, false, None).await
            else {
                tracing::error!("Failed to connect to websocket");
                break 'reconnect;
            };

            let (mut node_tx, mut node_rv) = node_socket.split();

            node_tx
                .send(TungsteniteMessage::Text(
                    tokio_tungstenite::tungstenite::Utf8Bytes::from(sub_info.as_str()),
                ))
                .await
                .unwrap();

            'ws_bridge: loop {
                select! {
                    Some(Command::Kill) = shutdown_rx.recv() => {
                        // send close frame to user ws
                        user_tx.send(Message::Close(None)).await.unwrap();
                    }
                    Some(Ok(msg)) = node_rv.next() => {
                        if let Some(m) = convert(msg) {
                            match m {
                                Message::Text(_) => {
                                    if let Err(_e) = user_tx.send(m).await {
                                        // User WS is disconnected
                                        node_tx.send(TungsteniteMessage::Close(None)).await.unwrap();
                                        cleanup_tx.send(Command::Kill).unwrap();
                                        warn!("Failed to relay msg to user from node");
                                        break 'reconnect;
                                    }
                                }
                                Message::Close(_close_frame) => {
                                    // log node info and close frame leading to ws closure
                                    // handle close of Node's WS connection
                                    let ping = axum::extract::ws::Message::Ping(Bytes::new());
                                    if let Err(_e) = user_tx.send(ping).await {
                                        // do not reestablish connection and kill loop
                                        // log user info and conditions leading to ws closure
                                        cleanup_tx.send(Command::Kill).unwrap();
                                        warn!("Closing user ws connection...");
                                        break 'reconnect;
                                    }
                                    reconnect_count += 1;
                                    warn!("Reconnecting ...");
                                    break 'ws_bridge;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        warn!("Exiting connection bridge");
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
}
