use game::{errors::GameError, game::GameState};
use responses::*;
use tokio::sync::broadcast;

use std::sync::Mutex;

use crate::request_handler::*;

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
        msg: FrontendRequest,
        player_name: &str,
    ) -> Result<Response, GameError> {
        let state = &mut *self.game.lock().unwrap();

        match msg {
            FrontendRequest::StartGame => start_game(state),
            FrontendRequest::SelectCharacter { character } => {
                // TODO: do something about this lookup madness
                let player_id = state
                    .selecting_characters()?
                    .player_by_name(player_name)?
                    .id;
                select_character(state, player_id, character)
            }
            FrontendRequest::DrawCard { card_type } => {
                let player_id = state.round()?.player_by_name(player_name)?.id;
                draw_card(state, card_type, player_id)
            }
            FrontendRequest::PutBackCard { card_idx } => {
                let player_id = state.round()?.player_by_name(player_name)?.id;
                put_back_card(state, card_idx, player_id)
            }
            FrontendRequest::BuyAsset { card_idx } => {
                let player_id = state.round()?.player_by_name(player_name)?.id;
                play_card(state, card_idx, player_id)
            }
            FrontendRequest::IssueLiability { card_idx } => {
                let player_id = state.round()?.player_by_name(player_name)?.id;
                play_card(state, card_idx, player_id)
            }
            FrontendRequest::RedeemLiability { liability_idx } => {
                let player_id = state.round()?.player_by_name(player_name)?.id;
                redeem_liability(state, liability_idx, player_id)
            }
            FrontendRequest::FireCharacter { character } => {
                let player_id = state.round()?.player_by_name(player_name)?.id;
                fire_character(state, player_id, character)
            }
            FrontendRequest::EndTurn => {
                let player_id = state.round()?.player_by_name(player_name)?.id;
                end_turn(state, player_id)
            }
        }
    }
}

impl Default for RoomState {
    fn default() -> Self {
        Self::new()
    }
}
