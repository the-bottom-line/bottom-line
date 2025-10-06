use std::sync::Arc;

use crate::server::{AppState, RoomState};
use axum::extract::ws::Utf8Bytes;
use serde::{Deserialize, Serialize};

pub fn handle_request(msg: Utf8Bytes, room_state: Arc<RoomState>) -> Utf8Bytes {
    //todo parse json request and

    return msg;
    return "".into();
}
