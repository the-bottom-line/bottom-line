use game::{errors::GameError, game::GameState};
use responses::*;

use crate::{request_handler::Response, rooms::RoomState};

use axum::{
    Router,
    extract::{
        State,
        ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade},
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
    time::Duration,
};
use tokio::sync::{Mutex as TokioMutex, broadcast}; // async mutex for shared sink

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub struct AppState {
    /// Keys are the name of the channel
    rooms: Arc<Mutex<HashMap<String, Arc<RoomState>>>>,
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
        rooms: Arc::new(Mutex::new(HashMap::new())),
    });

    let app = Router::new()
        .route("/websocket", get(websocket_handler))
        .with_state(app_state);

    // PANIC: this crashes if the port is not available. Since we control the server, we know it is
    // available and so this is safe to unwrap.
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    // PANIC: since we know the listener to have a valid address, this cannot crash.
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    // PANIC: Although this returns a result type, as specified by the axum documentation this will
    // never actually complete or return an error
    axum::serve(listener, app).await.unwrap();
}

async fn send_external(
    msg: impl Serialize,
    sender: Arc<TokioMutex<SplitSink<WebSocket, Message>>>,
) -> Result<(), axum::Error> {
    // PANIC: the documentation of `serde_json::to_string` specifies that it can return an error if
    // the implementation of `Serialize` fails for the given type, or if the type contains a map
    // with non-string keys. Since neither of those things are true, this as safe to unwrap.
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
                            DirectResponse::from(ResponseError::InvalidData),
                            sender.clone(),
                        )
                        .await;
                        continue;
                    }
                };

                let error_response = {
                    // PANIC: a mutex can only poison if any other thread that has access to it
                    // crashes. Since this cannot happen, unwrapping is safe.
                    let mut rooms = state.rooms.lock().unwrap();
                    channel = connect_channel.clone();
                    let room = rooms
                        .entry(connect_channel)
                        .or_insert_with(|| create_room(channel.clone(), state.rooms.clone()));

                    // PANIC: a mutex can only poison if any other thread that has access to it
                    // crashes. Since this cannot happen, unwrapping is safe.
                    match &mut *room.game.lock().unwrap() {
                        GameState::Lobby(lobby) => match lobby.join(connect_username.clone()) {
                            Ok(player) => {
                                debug_assert_eq!(player.name(), connect_username);
                                username = player.name().to_owned();
                                channel_idx = player.id().into();
                                break;
                            }
                            Err(e) => DirectResponse::from(GameError::from(e)),
                        },
                        // If the game is already running check and see if the player that is trying to connect had previously
                        // disconnected, if they are allow them to rejoin, and notify the other players that someone
                        // rejoined.
                        GameState::Round(round) => match round.player_by_name(&connect_username) {
                            Ok(player) => {
                                debug_assert_eq!(player.name(), connect_username);
                                match round.rejoin(player.id()) {
                                    Ok(p) => {
                                        username = p.name().to_owned();
                                        channel_idx = p.id().into();
                                        tracing::debug!("Player rejoined: {:?}", p.id());
                                        break;
                                    }
                                    Err(e) => DirectResponse::from(e),
                                }
                            }
                            Err(e) => DirectResponse::from(e),
                        },
                        GameState::SelectingCharacters(round) => {
                            match round.player_by_name(&connect_username) {
                                Ok(player) => {
                                    debug_assert_eq!(player.name(), connect_username);
                                    match round.rejoin(player.id()) {
                                        Ok(p) => {
                                            username = p.name().to_owned();
                                            channel_idx = p.id().into();
                                            tracing::debug!("Player rejoined: {:?}", p.id());
                                            break;
                                        }
                                        Err(e) => DirectResponse::from(e),
                                    }
                                }
                                Err(e) => DirectResponse::from(e),
                            }
                        }
                        _ => DirectResponse::from(ResponseError::GameAlreadyStarted),
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
        // PANIC: a mutex can only poison if any other thread that has access to it crashes. Since
        // this cannot happen, unwrapping is safe.
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

    let confirm = DirectResponse::YouJoinedGame {
        username: username.clone(),
        channel: channel.clone(),
    };
    tracing::debug!("Targeted Response: {:?}", confirm);
    let _ = send_external(confirm, sender.clone()).await;
    let mut rejoin_message: Option<DirectResponse> = None;
    // announce join to everyone
    // PANIC: a mutex can only poison if any other thread that has access to it crashes. Since this
    // cannot happen, unwrapping is safe.
    match &*room.game.lock().unwrap() {
        GameState::Lobby(lobby) => {
            let internal = UniqueResponse::PlayersInLobby {
                changed_player: username.clone(),
                usernames: lobby.usernames().iter().map(ToString::to_string).collect(),
            };
            tracing::debug!("Global Response: {:?}", internal);
            let _ = room.tx.send(internal);
        }
        GameState::Round(_) | GameState::SelectingCharacters(_) => {
            rejoin_message = Some(DirectResponse::YouRejoined)
        }
        // TODO: handle joins after game starts
        _ => return,
    }

    if let Some(message) = &rejoin_message {
        tracing::debug!("Sending rejoin message: {:?}", message);
        let _ = send_external(message, sender.clone()).await;
    }

    // task: forward broadcast messages to this client
    let mut send_task = {
        let sender = sender.clone();

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(UniqueResponse::RoomClosed { reason, .. }) => {
                        let frame = CloseFrame {
                            code: reason as u16,
                            reason: format!("{reason:?}").into(),
                        };

                        let mut s = sender.lock().await;
                        if s.send(Message::Close(Some(frame))).await.is_err() {
                            break;
                        }
                    }
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
                        if let Ok(json) = serde_json::from_str::<FrontendRequest>(&text) {
                            tracing::debug!("incoming request: {json:?}");

                            let direct = match room.handle_request(json, &name) {
                                Ok(Response(internal, direct)) => {
                                    for (id, responses) in internal.into_inner() {
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
        _ = &mut send_task => {
            recv_task.abort();
            player_send_task.abort();
        },
        _ = &mut recv_task => {
            send_task.abort();
            player_send_task.abort();
        }
        _ = &mut player_send_task => {
            recv_task.abort();
            send_task.abort();
        },
    };

    // announce leave
    // PANIC: a mutex can only poison if any other thread that has access to it crashes. Since this
    // cannot happen, unwrapping is safe.
    match &mut *room.game.lock().unwrap() {
        GameState::Lobby(lobby) => {
            // remove username on disconnect
            lobby.leave(&username);

            // send updated list to everyone
            for i in 0..lobby.len() {
                let _ = room.player_tx[i].send(UniqueResponse::PlayersInLobby {
                    changed_player: username.clone(),
                    usernames: lobby.usernames().iter().map(ToString::to_string).collect(),
                });
            }
        }
        // If we are outside of the lobby state then the game will already have started
        // We need to modify the player object and let the other players in that room know
        // that the player has disconnected. This also marks them as available for reconnecting.
        GameState::Round(game) => {
            let p = game.player_by_name(&username);
            match p {
                Ok(player) => {
                    let id = player.id();
                    let _ = game.leave(id); // This can fail but we just continue silently if it does
                    tracing::debug!("Player left: {:?}", id);
                }
                Err(_) => {
                    tracing::debug!(
                        "A disconnect happened but no connected player could be found."
                    );
                }
            }
        }
        GameState::SelectingCharacters(game) => {
            let p = game.player_by_name(&username);
            match p {
                Ok(player) => {
                    let id = player.id();
                    let _ = game.leave(id);
                    tracing::debug!("Player left: {:?}", id);
                }
                Err(_) => {
                    tracing::debug!(
                        "A disconnect happened but no connected player could be found."
                    );
                }
            }
        }
        _ => (),
    }
    // if let Ok(lobby) = room.game.lock().unwrap().lobby_mut() {
    //     // remove username on disconnect
    //     lobby.leave(&username);

    //     // send updated list to everyone
    //     for i in 0..lobby.len() {
    //         let _ = room.player_tx[i].send(UniqueResponse::PlayersInLobby {
    //             changed_player: username.clone(),
    //             usernames: lobby.usernames().iter().map(ToString::to_string).collect(),
    //         });
    //     }
    // }
}

pub fn create_room(
    channel: String,
    rooms: Arc<Mutex<HashMap<String, Arc<RoomState>>>>,
) -> Arc<RoomState> {
    let room = Arc::new(RoomState::new());

    let cleanup_handle = spawn_cleanup_task(channel.clone(), room.clone(), rooms.clone());

    *room.cleanup_handle.lock().unwrap() = Some(cleanup_handle);

    tracing::debug!("Created room with channel '{channel}'");

    room
}

fn spawn_cleanup_task(
    channel: String,
    room: Arc<RoomState>,
    rooms: Arc<Mutex<HashMap<String, Arc<RoomState>>>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        const DEFAULT_INACTIVITY_TIMEOUT: Duration = Duration::from_secs(300); // 5 min
        const DEFAULT_CLEANUP_INTERVAL: Duration = Duration::from_secs(30);

        let inactivity_timeout = if let Ok(timeout) = std::env::var("INACTIVITY_TIMEOUT") {
            Duration::from_secs(
                timeout
                    .parse()
                    .expect("ENV INACTIVITY_TIMEOUT should be a positive integer"),
            )
        } else {
            DEFAULT_INACTIVITY_TIMEOUT
        };
        let cleanup_interval = if let Ok(interval) = std::env::var("CLEANUP_INTERVAL") {
            Duration::from_secs(
                interval
                    .parse()
                    .expect("ENV CLEANUP_INTERVAL should be a positive integer"),
            )
        } else {
            DEFAULT_CLEANUP_INTERVAL
        };

        loop {
            tokio::time::sleep(cleanup_interval).await;

            let elapsed = room.last_activity.lock().unwrap().elapsed();

            if elapsed > inactivity_timeout {
                tracing::info!(
                    "Room with channel name '{}' inactive for {:?}, closing",
                    channel,
                    elapsed
                );

                let msg = UniqueResponse::RoomClosed {
                    channel: channel.clone(),
                    reason: RoomCloseReason::Inactive,
                };

                if let Err(e) = room.tx.send(msg) {
                    tracing::error!(%e);
                }

                // Give the messages a little bit of time to be sent out and received
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;

                // if let Err(e) = room.tx.send(UniqueResponse::ShutdownConnection) {
                //     tracing::error!(%e);
                // }

                // tokio::time::sleep(std::time::Duration::from_millis(250)).await;

                // Remove from HashMap to drop RoomState and close connected channels which cleans
                // up both the room as well as its connected user threads.
                rooms.lock().unwrap().remove(&channel);

                break;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use claim::*;
    use futures_util::stream::SplitStream;
    use serde::Deserialize;
    use tokio::sync::OnceCell;
    use tokio_tungstenite::{WebSocketStream, connect_async};
    use tungstenite::{Message, protocol::CloseFrame};

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

    static SERVER: OnceCell<()> = OnceCell::const_new();

    // #[fixture]
    async fn server_url() -> &'static str {
        SERVER
            .get_or_init(|| async {
                tokio::spawn(async {
                    setupsocket().await;
                });
            })
            .await;

        sleep(250).await;

        "ws://127.0.0.1:3000/websocket"
    }

    // #[rstest]
    #[tokio::test]
    async fn start_game() {
        // I don't understand why this is needed, but if I don't do this, somehow both tests
        // interfere. Hacky way to make sure neither test fucks with the other.
        sleep(2000).await;

        let url = server_url().await;

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
                    channel: "server-test".to_string(),
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

        send(&mut writers[0], FrontendRequest::StartGame).await;

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
                selectable_characters: Some(characters),
                closed_character,
                ..
            } = response
            {
                assert_some!(closed_character);

                let character = characters[0];
                send(writer, FrontendRequest::SelectCharacter { character }).await;

                let response = receive(reader).await;
                assert_matches!(
                    response,
                    DirectResponse::YouSelectedCharacter { character }
                        if character == characters[0]
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
                    selectable_characters: Some(characters),
                    closed_character,
                    ..
                } = response
                {
                    assert_none!(closed_character);

                    let character = characters[0];
                    send(writer, FrontendRequest::SelectCharacter { character }).await;

                    let response = receive(reader).await;
                    assert_matches!(
                        response,
                        DirectResponse::YouSelectedCharacter { character }
                            if character == characters[0]
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

    #[tokio::test]
    async fn room_timeout() {
        // Safe in single-threaded programs. No other threads are spun up by this point, so
        // this should be fine.
        unsafe {
            std::env::set_var("INACTIVITY_TIMEOUT", "5");
            std::env::set_var("CLEANUP_INTERVAL", "1");
        };

        let url = server_url().await;

        let (ws_stream1, _) = connect_async(url).await.unwrap();
        let (mut write1, mut read1) = ws_stream1.split();

        // room is created. Now it should take 5 seconds to be shut down for inactivity.
        send(
            &mut write1,
            Connect::Connect {
                channel: "timeout-test".to_owned(),
                username: "user 1".to_owned(),
            },
        )
        .await;

        let response = receive(&mut read1).await;
        assert!(matches!(response, UniqueResponse::PlayersInLobby { .. }));

        sleep(6000).await;

        let msg = read1
            .next()
            .await
            .expect("Stream ended")
            .expect("Failed to read message");

        assert!(matches!(msg, Message::Close(Some(CloseFrame { .. }))));
    }
}
