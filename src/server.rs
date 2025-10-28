use crate::{
    game::GameState,
    request_handler::{handle_internal_request, handle_request},
    responses::{DirectResponse, InternalResponse, ReceiveData, Response, ResponseError},
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
use futures_util::{
    sink::SinkExt,
    stream::{SplitSink, StreamExt},
};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex}, // std Mutex used only for the username set
};
use tokio::sync::{Mutex as TokioMutex, broadcast}; // async mutex for shared sink

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[allow(clippy::large_enum_variant)]
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

async fn send_external(
    msg: impl Serialize,
    sender: Arc<TokioMutex<SplitSink<WebSocket, Message>>>,
) -> Result<(), axum::Error> {
    let msg = serde_json::to_string(&msg).unwrap();
    let mut s = sender.lock().await;
    s.send(Message::Text(msg.into())).await
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
        match message {
            Message::Text(text) => {
                let (connect_username, connect_channel) = match serde_json::from_str(&text) {
                    Ok(ReceiveData::Connect { username, channel }) => (username, channel),
                    Err(error) => {
                        tracing::error!(%error);
                        let _ = send_external(
                            DirectResponse::Error(ResponseError::UsernameAlreadyTaken),
                            sender.clone(),
                        )
                        .await;
                        break;
                    }
                    _ => {
                        let _ = send_external(
                            DirectResponse::Error(ResponseError::InvalidData),
                            sender.clone(),
                        )
                        .await;
                        break;
                    }
                };

                {
                    // If username that is sent by client is not taken, fill username string.
                    let mut rooms = state.rooms.lock().unwrap();

                    channel = connect_channel.clone();
                    let room = rooms
                        .entry(connect_channel)
                        .or_insert_with(|| Arc::new(RoomState::new()));

                    if let Ok(mut mutex) = room.game.lock()
                        && let Game::InLobby { user_set } = &mut *mutex
                        && !user_set.contains(&connect_username)
                    {
                        user_set.insert(connect_username.to_owned());
                        username = connect_username.clone();
                    }
                }

                if !connect_username.is_empty() {
                    break;
                } else {
                    // Only send our client that username is taken.
                    let _ = send_external(
                        DirectResponse::Error(ResponseError::InvalidUsername),
                        sender.clone(),
                    )
                    .await;
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
            'outer: loop {
                match rx.recv().await {
                    Ok(Json(json)) => {
                        tracing::debug!("public recv: {json:?}");
                        if let Some(external) = handle_internal_request(json, room.clone(), &name) {
                            for e in external {
                                if send_external(e, sender.clone()).await.is_err() {
                                    break 'outer;
                                }
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
                            let Response(public, external) =
                                handle_request(json, room.clone(), &name);

                            tracing::debug!("public send: {public:?}");
                            if let Some(public) = public {
                                let _ = tx.send(public.into());

                                if send_external(external, sender.clone()).await.is_err() {
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
    use std::time::Duration;

    use crate::{game::PickableCharacters, responses::UniqueResponse};

    use super::*;
    use tokio::time::sleep;
    use tokio_tungstenite::connect_async;

    #[tokio::test]
    async fn start_game() {
        let url = "ws://127.0.0.1:3000/websocket";

        tokio::task::spawn(async move {
            setupsocket().await;
        });

        sleep(Duration::from_millis(250)).await;

        let (ws_stream1, _) = connect_async(url).await.unwrap();
        let (write1, read1) = ws_stream1.split();

        let (ws_stream2, _) = connect_async(url).await.unwrap();
        let (write2, read2) = ws_stream2.split();

        let (ws_stream3, _) = connect_async(url).await.unwrap();
        let (write3, read3) = ws_stream3.split();

        let (ws_stream4, _) = connect_async(url).await.unwrap();
        let (write4, read4) = ws_stream4.split();

        let mut writers = [write1, write2, write3, write4];
        let mut readers = [read1, read2, read3, read4];

        for (i, writer) in writers.iter_mut().enumerate() {
            let msg = serde_json::to_string(&ReceiveData::Connect {
                channel: "thing".to_string(),
                username: format!("user {}", i),
            })
            .unwrap();
            writer.send(msg.into()).await.unwrap();
        }

        sleep(Duration::from_millis(100)).await;

        for (i, reader) in readers.iter_mut().enumerate() {
            for _ in i..4 {
                let msg = reader.next().await.unwrap().unwrap().into_text().unwrap();
                let response = serde_json::from_str::<UniqueResponse>(&msg).unwrap();
                assert!(matches!(response, UniqueResponse::PlayersInLobby { .. }))
            }
        }

        writers[0].send(r#"{"action": "StartGame"}"#.into())
            .await
            .unwrap();

        let msg = readers[0].next().await.unwrap().unwrap().into_text().unwrap();
        let response = serde_json::from_str::<DirectResponse>(&msg).unwrap();
        assert!(matches!(response, DirectResponse::YouStartedGame));

        let mut selectable_character_count = 0;
        let mut player_idx = 0;
        let mut chairman = 0.into();
        let mut pickable = PickableCharacters {
            characters: vec![],
            closed_character: None,
        };

        for (i, reader) in readers.iter_mut().enumerate() {
            let msg = reader.next().await.unwrap().unwrap().into_text().unwrap();
            let response = serde_json::from_str::<UniqueResponse>(&msg).unwrap();
            assert!(matches!(response, UniqueResponse::StartGame { .. }));

            let msg = reader.next().await.unwrap().unwrap().into_text().unwrap();
            let response = serde_json::from_str::<UniqueResponse>(&msg).unwrap();
            assert!(matches!(
                response,
                UniqueResponse::SelectingCharacters { .. }
            ));
            if let UniqueResponse::SelectingCharacters {
                chairman_id,
                pickable_characters: Some(p),
                turn_order,
                ..
            } = response
            {
                assert_eq!(chairman_id, turn_order[0]);
                assert!(p.closed_character.is_some());
                selectable_character_count += 1;
                player_idx = i;
                chairman = chairman_id;
                pickable = p;
            }
        }

        assert_eq!(selectable_character_count, 1);

        let msg = serde_json::to_string(&ReceiveData::SelectCharacter {
            character: pickable.characters[0],
        })
        .unwrap();

        writers[player_idx].send(msg.into()).await.unwrap();

        let msg = readers[player_idx].next().await.unwrap().unwrap().into_text().unwrap();

        let response = serde_json::from_str::<DirectResponse>(&msg).unwrap();
        assert!(matches!(response, DirectResponse::YouSelectedCharacter { .. }));
        
        selectable_character_count = 0;
        
        for (i, reader) in readers.iter_mut().enumerate() {
            let msg = reader.next().await.unwrap().unwrap().into_text().unwrap();
            let response = serde_json::from_str::<UniqueResponse>(&msg).unwrap();
            match response {
                UniqueResponse::SelectedCharacter { player_id, pickable_characters } => {
                    assert_eq!(player_id, chairman);
                    // assert_eq!(character, pickable.characters[0]);
                    if let Some(p) = pickable_characters {
                        assert!(p.closed_character.is_none());
                        selectable_character_count += 1;
                        pickable = p;
                        player_idx = i;
                    }
                }
                _ => panic!("Unexpected response")
            }
        }
        
        assert_eq!(selectable_character_count, 1);
        
        let msg = serde_json::to_string(&ReceiveData::SelectCharacter {
            character: pickable.characters[0],
        })
        .unwrap();

        writers[player_idx].send(msg.into()).await.unwrap();

        let msg = readers[player_idx].next().await.unwrap().unwrap().into_text().unwrap();

        let response = serde_json::from_str::<DirectResponse>(&msg).unwrap();
        assert!(matches!(response, DirectResponse::YouSelectedCharacter { .. }));
        
        selectable_character_count = 0;
        
        for (i, reader) in readers.iter_mut().enumerate() {
            let msg = reader.next().await.unwrap().unwrap().into_text().unwrap();
            let response = serde_json::from_str::<UniqueResponse>(&msg).unwrap();
            match response {
                UniqueResponse::SelectedCharacter { player_id, pickable_characters } => {
                    assert_eq!(player_id, chairman);
                    // assert_eq!(character, pickable.characters[0]);
                    if let Some(p) = pickable_characters {
                        assert!(p.closed_character.is_none());
                        selectable_character_count += 1;
                        pickable = p;
                        player_idx = i;
                    }
                }
                _ => panic!("Unexpected response")
            }
        }
        
        assert_eq!(selectable_character_count, 1);
        
        // let msg = serde_json::to_string(&ReceiveData::SelectCharacter {
        //     character: pickable.characters[0],
        // })
        // .unwrap();

        // writers[player_idx].send(msg.into()).await.unwrap();

        // let msg = readers[player_idx].next().await.unwrap().unwrap().into_text().unwrap();

        // let response = serde_json::from_str::<DirectResponse>(&msg).unwrap();
        // assert!(matches!(response, DirectResponse::SelectedCharacter { .. }));
    }
}
