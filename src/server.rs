use crate::{game::GameState, responses::*, rooms::RoomState};

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
    collections::HashMap,
    sync::{Arc, Mutex}, // std Mutex used only for the username set
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

    // receive initial username message
    let mut username = String::new();
    let mut channel = String::new();

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

                if !connect_username.is_empty() {
                    // If username that is sent by client is not taken, fill username string.
                    let mut rooms = state.rooms.lock().unwrap();

                    channel = connect_channel.clone();
                    let room = rooms
                        .entry(connect_channel)
                        .or_insert_with(|| Arc::new(RoomState::new()));

                    if let Ok(mut mutex) = room.game.lock()
                        && let GameState::Lobby(lobby) = &mut *mutex
                        && lobby.join(connect_username.to_owned())
                    {
                        username = connect_username.clone();
                    } else {
                        // TODO: Idk if this sends because I don't .await but also I get an error
                        // because it stops being sync? idk wtf is going on here
                        let _ = send_external(
                            DirectResponse::Error(ResponseError::GameAlreadyStarted),
                            sender.clone(),
                        );
                        return;
                    }
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
                        for e in room.handle_internal_request(json, &name) {
                            // tracing::debug!("unique send: {e:?}");
                            if send_external(e, sender.clone()).await.is_err() {
                                break 'outer;
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

                            let direct = match room.handle_request(json, &name) {
                                Ok(Response(internal, direct)) => {
                                    tracing::debug!("internal send: {internal:?}");

                                    let _ = tx.send(Json(internal));

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
        if let GameState::Lobby(lobby) = &mut *room.game.lock().unwrap() {
            lobby.leave(&username);
        }
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

        for (reader, writer) in readers.iter_mut().zip(&mut writers) {
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
            }
        }

        for _ in 1..readers.len() {
            for (reader, writer) in readers.iter_mut().zip(&mut writers) {
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
                }
            }
        }

        for reader in readers.iter_mut() {
            let response = receive(reader).await;
            assert_matches!(response, UniqueResponse::SelectedCharacter { .. });

            let response = receive(reader).await;
            assert_matches!(response, UniqueResponse::TurnStarts { .. });
        }
    }
}
