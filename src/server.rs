use crate::{
    game::GameState,
    request_handler::{
        ExternalResponse, InternalResponse, ReceiveData, Response, handle_public_request,
        handle_request,
    },
};

use axum::{
    Json, Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::get,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex}, // std Mutex used only for the username set
};
use tokio::sync::{Mutex as TokioMutex, broadcast}; // async mutex for shared sink

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use serde::{Deserialize, Serialize};

pub enum Game {
    InLobby { user_set: HashSet<String> },
    GameStarted { state: GameState },
}

pub struct AppState {
    /// Keys are the name of the channel
    rooms: Mutex<HashMap<String, Arc<RoomState>>>,
}

pub struct RoomState {
    /// Previously created in main.
    tx: broadcast::Sender<Json<InternalResponse>>,
    pub game: Mutex<Game>,
}

impl RoomState {
    fn new() -> Self {
        Self {
            // Create a new channel for every room
            tx: broadcast::channel(100).0,
            // Track usernames per room rather than globally.
            game: Mutex::new(Game::InLobby {
                user_set: HashSet::new(),
            }),
        }
    }
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

pub async fn setupsocket() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=trace", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app_state = Arc::new(AppState {
        rooms: Mutex::new(HashMap::new()),
    });

    let app = Router::new()
        .route("/websocket", get(websocket_handler))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn websocket(stream: WebSocket, state: Arc<AppState>) {
    // split sink + stream
    let (sender, mut receiver) = stream.split();
    // wrap sink in an async mutex so multiple tasks can send safely
    let sender = Arc::new(TokioMutex::new(sender));

    // receive initial username message
    let mut username = String::new();
    let mut channel = String::new();

    while let Some(Ok(message)) = receiver.next().await {
        #[derive(Deserialize)]
        struct Connect {
            username: String,
            channel: String,
        }

        match message {
            Message::Text(text) => {
                let connect: Connect = match serde_json::from_str(&text) {
                    Ok(connect) => connect,
                    Err(error) => {
                        tracing::error!(%error);
                        let msg =
                            serde_json::to_string(&ExternalResponse::UsernameAlreadyTaken).unwrap();
                        let mut s = sender.lock().await;
                        let _ = s.send(Message::Text(msg.into())).await;
                        break;
                    }
                };

                {
                    // If username that is sent by client is not taken, fill username string.
                    let mut rooms = state.rooms.lock().unwrap();

                    channel = connect.channel.clone();
                    let room = rooms
                        .entry(connect.channel)
                        .or_insert_with(|| Arc::new(RoomState::new()));

                    if let Ok(mut mutex) = room.game.lock() {
                        if let Game::InLobby { user_set } = &mut *mutex {
                            if !user_set.contains(&connect.username) {
                                user_set.insert(connect.username.to_owned());
                                username = connect.username.clone();
                            }
                        }
                    }
                }

                if !username.is_empty() {
                    break;
                } else {
                    // Only send our client that username is taken.
                    let msg = serde_json::to_string(&ExternalResponse::InvalidUsername).unwrap();
                    let mut s = sender.lock().await;
                    let _ = s.send(Message::Text(msg.into())).await;
                    return;
                }
            }
            Message::Close(_) => return,
            _ => continue,
        }
    }

    let room = {
        let rooms = state.rooms.lock().unwrap();
        rooms
            .get(&channel)
            .cloned()
            .expect("The room should exist at this point")
    };

    let tx = room.tx.clone();
    // subscribe to broadcast channel
    let mut rx = tx.subscribe();

    // announce join to everyone
    let msg = InternalResponse::PlayerJoined {
        username: username.clone(),
    };
    tracing::debug!("{msg:?}");
    let _ = tx.send(msg.into());

    // task: forward broadcast messages to this client
    let mut send_task = {
        let name = username.clone();
        let room = room.clone();
        let sender = sender.clone();

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(Json(json)) => {
                        tracing::debug!("public recv: {json:?}");
                        if let Some(private) = handle_public_request(json, room.clone(), &name) {
                            let msg = serde_json::to_string(&private).unwrap();
                            let mut s = sender.lock().await;
                            if s.send(Message::Text(msg.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    // If we lagged behind, just continue
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    // channel closed
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        })
    };

    // task: read client messages, broadcast them, and send a custom reply to the sender only
    let mut recv_task = {
        let tx = tx.clone();
        let sender = sender.clone();
        let name = username.clone();
        let room = room.clone();

        tokio::spawn(async move {
            while let Some(Ok(message)) = receiver.next().await {
                match message {
                    Message::Text(text) => {
                        if let Ok(json) = serde_json::from_str::<ReceiveData>(&text) {
                            tracing::debug!("incoming json: {json:?}");
                            let Response(public, private) =
                                handle_request(json, room.clone(), &name);

                            // // broadcast to everyone (including sender)
                            // let public_ser = serde_json::to_string(&public).unwrap();
                            tracing::debug!("public send: {public:?}");
                            let _ = tx.send(public.into());

                            // send a different message only to the sender
                            let private_ser = serde_json::to_string(&private).unwrap();
                            {
                                let mut s = sender.lock().await;
                                if s.send(private_ser.into()).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Message::Close(_) => break,
                    _ => continue,
                }
            }
        })
    };

    // if either task finishes, abort the other
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    };

    // announce leave
    let msg = InternalResponse::PlayerLeft {
        username: username.clone(),
    };
    tracing::debug!("{msg:?}");
    let _ = tx.send(msg.into());
    // remove username on disconnect
    {
        let rooms = state.rooms.lock().unwrap();
        let room = rooms
            .get(&channel)
            .cloned()
            .expect("The room should exist at this point");
        if let Game::InLobby { user_set } = &mut *room.game.lock().unwrap() {
            user_set.remove(&username);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_tungstenite::connect_async;

    #[tokio::test]
    async fn start_game() {
        let url = "127.0.0.1:3000";

        tokio::task::spawn(async move {
            setupsocket().await;
        });

        let (ws_stream1, _) = connect_async(format!("{}/websocket", url)).await.unwrap();
        let (mut write1, mut read1) = ws_stream1.split();

        let (ws_stream2, _) = connect_async(format!("{}/websocket", url)).await.unwrap();
        let (mut write2, mut read2) = ws_stream2.split();

        let (ws_stream3, _) = connect_async(format!("{}/websocket", url)).await.unwrap();
        let (mut write3, mut read3) = ws_stream3.split();

        let (ws_stream4, _) = connect_async(format!("{}/websocket", url)).await.unwrap();
        let (mut write4, mut read4) = ws_stream4.split();

        write1
            .send(tokio_tungstenite::tungstenite::Message::Text(
                r#"{"channel":"thing","username": "user1"}"#.into(),
            ))
            .await
            .unwrap();
        let msg = read1.next().await.unwrap().unwrap();
        dbg!(msg);
    }
}
