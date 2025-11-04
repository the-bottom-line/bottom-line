use std::sync::Mutex;

use tokio::sync::broadcast;

use crate::{errors::GameError, game::GameState, request_handler::*, responses::*};

/// All-encompassing state each room has access to
pub struct RoomState {
    /// Internal broadcast that can be received by any connected thread
    pub tx: broadcast::Sender<UniqueResponse>,
    /// Internal broadcast channels to send responses specific to each player
    pub player_tx: [broadcast::Sender<UniqueResponse>; 7],
    /// Per-room gamestate
    pub game: Mutex<GameState>,
}

impl RoomState {
    pub fn new() -> Self {
        Self {
            tx: broadcast::channel(64).0,
            player_tx: [
                broadcast::channel(64).0,
                broadcast::channel(64).0,
                broadcast::channel(64).0,
                broadcast::channel(64).0,
                broadcast::channel(64).0,
                broadcast::channel(64).0,
                broadcast::channel(64).0,
            ],
            game: Mutex::new(GameState::new()),
        }
    }

    pub fn handle_request(
        &self,
        msg: ReceiveData,
        player_name: &str,
    ) -> Result<Response, GameError> {
        let state = &mut *self.game.lock().unwrap();

        match msg {
            ReceiveData::StartGame => start_game(state),
            ReceiveData::SelectCharacter { character } => {
                // TODO: do something about this lookup madness
                let player_id = state
                    .selecting_characters()?
                    .player_by_name(player_name)?
                    .id;
                select_character(state, player_id, character)
            }
            ReceiveData::DrawCard { card_type } => {
                let player_id = state.round()?.player_by_name(player_name)?.id;
                draw_card(state, card_type, player_id)
            }
            ReceiveData::PutBackCard { card_idx } => {
                let player_id = state.round()?.player_by_name(player_name)?.id;
                put_back_card(state, card_idx, player_id)
            }
            ReceiveData::BuyAsset { card_idx } => {
                let player_id = state.round()?.player_by_name(player_name)?.id;
                play_card(state, card_idx, player_id)
            }
            ReceiveData::IssueLiability { card_idx } => {
                let player_id = state.round()?.player_by_name(player_name)?.id;
                play_card(state, card_idx, player_id)
            }
            ReceiveData::EndTurn => {
                let player_id = state.round()?.player_by_name(player_name)?.id;
                end_turn(state, player_id)
            }
        }
    }
}
