use std::sync::Mutex;

use axum::Json;
use tokio::sync::broadcast;

use crate::{game::GameState, responses::*};

pub struct RoomState {
    /// Previously created in main.
    pub tx: broadcast::Sender<Json<InternalResponse>>,
    pub game: Mutex<GameState>,
}

impl RoomState {
    pub fn new() -> Self {
        Self {
            // Create a new channel for every room
            tx: broadcast::channel(100).0,
            // Track usernames per room rather than globally.
            game: Mutex::new(GameState::new()),
        }
    }
}
