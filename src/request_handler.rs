use std::sync::Arc;

use crate::{game::TheBottomLine, server::{AppState, RoomState}};
use axum::extract::ws::Utf8Bytes;
use serde::{Deserialize, Serialize};

pub fn handle_request(msg: Utf8Bytes, room_state: Arc<RoomState>) -> Utf8Bytes {
    //todo parse json request and

    let mut game = room_state.game.lock().unwrap();
    
    match &mut *game {
        crate::server::Game::GameStarted { state } => {
            state.player_draw_card(0, crate::game::CardType::Asset);
        },
        _ => {}
    }

    return msg;
    return "".into();
}
