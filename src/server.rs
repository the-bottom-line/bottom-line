use crate::{errors::GameError, game::GameState, responses::*, rooms::RoomState};

use axum::{
    Router,
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
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::{Mutex as TokioMutex, broadcast}; // async mutex for shared sink

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub struct AppState {
    /// Keys are the name of the channel
    rooms: Mutex<HashMap<String, Arc<RoomState>>>,
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

    let mut channel_idx = 8; // invalid id to start
    let mut username = String::new();
    let mut channel = String::new();

    // receive initial username message
    while let Some(Ok(message)) = receiver.next().await {
        match message {
            Message::Text(text) => {
                let (connect_username, connect_channel) = match serde_json::from_str(&text) {
                    Ok(Connect::Connect { username, channel }) => (username, channel),
                    Err(error) => {
                        tracing::error!(%error);
                        let _ = send_external(
                            DirectResponse::Error(ResponseError::InvalidData),
                            sender.clone(),
                        )
                        .await;
                        continue;
                    }
                };

                let error_response = {
                    let mut rooms = state.rooms.lock().unwrap();
                    channel = connect_channel.clone();
                    let room = rooms
                        .entry(connect_channel)
                        .or_insert_with(|| Arc::new(RoomState::new()));

                    match &mut *room.game.lock().unwrap() {
                        GameState::Lobby(lobby) => match lobby.join(connect_username.clone()) {
                            Ok(player_id) => {
                                username = connect_username.clone();
                                channel_idx = player_id.into();
                                break;
                            }
                            Err(e) => DirectResponse::from(GameError::from(e)),
                        },
                        _ => DirectResponse::Error(ResponseError::GameAlreadyStarted),
                    }
                };

                let _ = send_external(error_response, sender.clone()).await;
                return;
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

    let mut player_rx = room.player_tx[channel_idx].subscribe();

    // announce join to everyone
    match &*room.game.lock().unwrap() {
        GameState::Lobby(lobby) => {
            let internal = UniqueResponse::PlayersInLobby {
                changed_player: username.clone(),
                usernames: lobby.usernames(),
            };

            tracing::debug!("Global Response: {:?}", internal);
            let _ = room.tx.send(internal);
        }
        // TODO: handle joins after game starts
        _ => return,
    }

    // task: forward broadcast messages to this client
    let mut send_task = {
        let sender = sender.clone();

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        tracing::debug!("unique send: {msg:?}");
                        if send_external(msg, sender.clone()).await.is_err() {
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

    // task: forward player messages to this client
    let mut player_send_task = {
        let sender = sender.clone();

        tokio::spawn(async move {
            loop {
                match player_rx.recv().await {
                    Ok(msg) => {
                        tracing::debug!("unique send: {msg:?}");
                        if send_external(msg, sender.clone()).await.is_err() {
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
        let sender = sender.clone();
        let name = username.clone();
        let room = room.clone();

        tokio::spawn(async move {
            while let Some(Ok(message)) = receiver.next().await {
                match message {
                    Message::Text(text) => {
                        if let Ok(json) = serde_json::from_str::<ReceiveData>(&text) {
                            tracing::debug!("incoming json: {json:?}");

                            let direct = match room.handle_request(json, &name) {
                                Ok(Response(internal, direct)) => {
                                    for (id, responses) in internal.into_inner() {
                                        tracing::debug!("internal send: {responses:?}");
                                        let idx = usize::from(id);
                                        for r in responses {
                                            let _ = room.player_tx[idx].send(r.clone());
                                        }
                                    }

                                    direct
                                }
                                Err(e) => e.into(),
                            };
                            tracing::debug!("direct response: {direct:?}");

                            if send_external(direct, sender.clone()).await.is_err() {
                                break;
                            }
                        }
                    }
                    Message::Close(_) => break,
                    _ => continue,
                }
            }
        })
    };

    // if any task finishes, abort the others
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
        _ = &mut player_send_task => player_send_task.abort(),
    };

    // announce leave
    match room.game.lock().unwrap().lobby_mut().ok() {
        Some(lobby) => {
            // remove username on disconnect
            lobby.leave(&username);

            // send updated list to everyone
            for i in 0..lobby.len() {
                let _ = room.player_tx[i].send(UniqueResponse::PlayersInLobby {
                    changed_player: username.clone(),
                    usernames: lobby.usernames(),
                });
            }
        }
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::responses::UniqueResponse;

    use super::*;
    use claim::*;
    use futures_util::stream::SplitStream;
    use serde::Deserialize;
    use tokio_tungstenite::{WebSocketStream, connect_async};

    pub async fn receive<T, S>(reader: &mut SplitStream<WebSocketStream<S>>) -> T
    where
        for<'a> T: Deserialize<'a>,
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        let msg = reader
            .next()
            .await
            .expect("Stream ended")
            .expect("Failed to read message")
            .into_text()
            .expect("Message was not text");

        serde_json::from_str::<T>(&msg).unwrap()
    }

    pub async fn send<S>(writer: &mut S, response: impl Serialize)
    where
        S: futures_util::Sink<tokio_tungstenite::tungstenite::Message> + Unpin,
        S::Error: std::fmt::Debug,
    {
        let msg = serde_json::to_string(&response).expect("Serialization failed");

        writer
            .send(msg.into())
            .await
            .expect("Sending message failed");
    }

    async fn sleep(milliseconds: u64) {
        tokio::time::sleep(Duration::from_millis(milliseconds)).await
    }

    #[tokio::test]
    async fn start_game() {
        let url = "ws://127.0.0.1:3000/websocket";

        tokio::task::spawn(async move {
            setupsocket().await;
        });

        sleep(250).await;

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
            send(
                writer,
                Connect::Connect {
                    channel: "thing".to_string(),
                    username: format!("user {}", i),
                },
            )
            .await;
        }

        sleep(100).await;

        for (i, reader) in readers.iter_mut().enumerate() {
            // The first player gets 4 lists (one with one, one with two players and so on), the
            // second player gets one with two and so on
            for _ in i..4 {
                let response = receive(reader).await;
                assert!(matches!(response, UniqueResponse::PlayersInLobby { .. }))
            }
        }

        send(&mut writers[0], ReceiveData::StartGame).await;

        let response = receive(&mut readers[0]).await;
        assert!(matches!(response, DirectResponse::YouStartedGame));

        for reader in readers.iter_mut() {
            let response = receive(reader).await;
            assert_matches!(response, UniqueResponse::StartGame { .. });
        }

        let mut selected = None::<usize>;

        for (i, (reader, writer)) in readers.iter_mut().zip(&mut writers).enumerate() {
            let response = receive(reader).await;
            assert_matches!(response, UniqueResponse::SelectingCharacters { .. });

            if let UniqueResponse::SelectingCharacters {
                pickable_characters: Some(p),
                ..
            } = response
            {
                assert_some!(p.closed_character);

                let character = p.characters[0];
                send(writer, ReceiveData::SelectCharacter { character }).await;

                let response = receive(reader).await;
                assert_matches!(
                    response,
                    DirectResponse::YouSelectedCharacter { character }
                        if character == p.characters[0]
                );

                selected = Some(i);
            }
        }

        for _ in 1..readers.len() {
            let chosen = selected.unwrap();

            // Since this isn't done in the main loop
            let response = receive(&mut readers[chosen]).await;
            assert_matches!(response, UniqueResponse::SelectedCharacter { .. });

            for (i, (reader, writer)) in readers
                .iter_mut()
                .zip(&mut writers)
                .enumerate()
                .filter(|(i, _)| *i != chosen)
            {
                let response = receive(reader).await;
                assert_matches!(response, UniqueResponse::SelectedCharacter { .. });

                if let UniqueResponse::SelectedCharacter {
                    pickable_characters: Some(p),
                    ..
                } = response
                {
                    assert_none!(p.closed_character);

                    let character = p.characters[0];
                    send(writer, ReceiveData::SelectCharacter { character }).await;

                    let response = receive(reader).await;
                    assert_matches!(
                        response,
                        DirectResponse::YouSelectedCharacter { character }
                            if character == p.characters[0]
                    );

                    selected = Some(i);
                }
            }
        }

        for reader in readers.iter_mut() {
            let response = receive(reader).await;
            assert_matches!(response, UniqueResponse::TurnStarts { .. });
        }
    }
}
