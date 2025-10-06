use axum::{
    extract::{
        ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex}, // std Mutex used only for the username set
};
use tokio::sync::{broadcast, Mutex as TokioMutex}; // async mutex for shared sink

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use serde::{Deserialize, Serialize};

struct AppState {
    /// Keys are the name of the channel
    rooms: Mutex<HashMap<String, RoomState>>,
}

struct RoomState {
    /// Previously stored in AppState
    user_set: HashSet<String>,
    /// Previously created in main.
    tx: broadcast::Sender<String>,
}

impl RoomState {
    fn new() -> Self {
        Self {
            // Track usernames per room rather than globally.
            user_set: HashSet::new(),
            // Create a new channel for every room
            tx: broadcast::channel(100).0,
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

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
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
    let mut tx = None::<broadcast::Sender<String>>;

    while let Some(Ok(message)) = receiver.next().await {
        if let Message::Text(name) = message {            
            #[derive(Deserialize)]
            struct Connect {
                username: String,
                channel: String,
            }

            let connect: Connect = match serde_json::from_str(&name) {
                Ok(connect) => connect,
                Err(error) => {
                    tracing::error!(%error);
                    let mut s = sender.lock().await;
                let _ = s
                    .send(Message::Text(Utf8Bytes::from_static(
                        "Failed to parse connect message",
                    )))
                    .await;
                    break;
                }
            };
            {
                // If username that is sent by client is not taken, fill username string.
                let mut rooms = state.rooms.lock().unwrap();

                channel = connect.channel.clone();
                let room = rooms.entry(connect.channel).or_insert_with(RoomState::new);

                tx = Some(room.tx.clone());

                if !room.user_set.contains(&connect.username) {
                    room.user_set.insert(connect.username.to_owned());
                    username = connect.username.clone();
                }
            }
            if tx.is_some() && !username.is_empty() {
                break;
            } else {
                // Only send our client that username is taken.
                let mut s = sender.lock().await;
                let _ = s
                    .send(Message::Text(Utf8Bytes::from_static(
                        "Username already taken.",
                    )))
                    .await;
                return;
            }


        }
    }

    let tx = tx.unwrap();
    // subscribe to broadcast channel
    let mut rx = tx.subscribe();

    // announce join to everyone
    let msg = format!("{username} joined.");
    tracing::debug!("{msg}");
    let _ = tx.send(msg);

    // task: forward broadcast messages to this client
    let mut send_task = {
        let sender = sender.clone();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        let mut s = sender.lock().await;
                        if s.send(Message::Text(format!("{msg}").into())).await.is_err() {
                            break;
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

        tokio::spawn(async move {
            while let Some(Ok(Message::Text(text))) = receiver.next().await {
                // broadcast to everyone (including sender)
                let _ = tx.send(format!("{name}: {text}"));

                // send a different message only to the sender
                let mut s = sender.lock().await;
                if s
                    .send(Message::Text(format!("(private) You said: {text}").into()))
                    .await
                    .is_err()
                {
                    break;
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
    let msg = format!("{username} left.");
    tracing::debug!("{msg}");
    let _ = tx.send(msg);
    let mut rooms = state.rooms.lock().unwrap();
    // free username
     rooms.get_mut(&channel).unwrap().user_set.remove(&username);

}

