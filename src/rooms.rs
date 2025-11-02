use std::sync::Mutex;

use axum::Json;
use tokio::sync::broadcast;

use crate::{
    errors::GameError,
    game::{GameState, TheBottomLine},
    request_handler::{external, internal},
    responses::*,
};

/// All-encompassing state each room has access to
pub struct RoomState {
    /// Internal broadcast that can be received by any connected thread
    pub tx: broadcast::Sender<Json<InternalResponse>>,
    /// Per-room gamestate
    pub game: Mutex<GameState>,
}

impl RoomState {
    pub fn new() -> Self {
        Self {
            tx: broadcast::channel(100).0,
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
            ReceiveData::StartGame => match state {
                GameState::Lobby(_) => {
                    state.start_game("assets/cards/boardgame.json")?;
                    tracing::debug!("{msg:?}");
                    Ok(Response(
                        InternalResponse::GameStarted,
                        DirectResponse::YouStartedGame,
                    ))
                }
                _ => Err(GameError::NotLobbyState),
            },
            ReceiveData::SelectCharacter { character } => {
                // TODO: do something about this lookup madness
                let player_id = state.player_by_name(player_name)?.id;
                external::select_character(state, player_id, character)
            }
            ReceiveData::DrawCard { card_type } => {
                let player_id = state.player_by_name(player_name)?.id;
                external::draw_card(state, card_type, player_id)
            }
            ReceiveData::PutBackCard { card_idx } => {
                let player_id = state.player_by_name(player_name)?.id;
                external::put_back_card(state, card_idx, player_id)
            }
            ReceiveData::BuyAsset { card_idx } => {
                let player_id = state.player_by_name(player_name)?.id;
                external::play_card(state, card_idx, player_id)
            }
            ReceiveData::IssueLiability { card_idx } => {
                let player_id = state.player_by_name(player_name)?.id;
                external::play_card(state, card_idx, player_id)
            }
            ReceiveData::EndTurn => {
                let player_id = state.player_by_name(player_name)?.id;
                external::end_turn(state, player_id)
            }
        }
    }

    pub fn handle_internal_request(
        &self,
        msg: InternalResponse,
        player_name: &str,
    ) -> Vec<UniqueResponse> {
        use InternalResponse as IR;

        match &*self.game.lock().unwrap() {
            GameState::Lobby(lobby) => match msg {
                IR::PlayerJoined { username } => internal::players_in_lobby(lobby, username),
                IR::PlayerLeft { username } => internal::players_in_lobby(lobby, username),
                _ => unreachable!(),
            },
            GameState::SelectingCharacters(selecting) => match msg {
                IR::GameStarted => internal::game_started(selecting, player_name),
                IR::SelectedCharacter => {
                    internal::selected_character_selecting(selecting, player_name)
                }
                IR::TurnEnded { player_id } => {
                    internal::turn_ended_selecting(selecting, player_id, player_name)
                }
                _ => unreachable!(),
            },
            GameState::Round(round) => match msg {
                IR::SelectedCharacter => internal::selected_character_round(round),
                IR::DrawnCard {
                    player_id,
                    card_type,
                } => internal::drawn_card(player_id, card_type),
                IR::PutBackCard {
                    player_id,
                    card_type,
                } => internal::put_back_card(player_id, card_type),
                IR::BoughtAsset { player_id, asset } => internal::bought_asset(player_id, asset),
                IR::IssuedLiability {
                    player_id,
                    liability,
                } => internal::issued_liability(player_id, liability),
                IR::TurnEnded { player_id } => internal::turn_ended_round(round, player_id),
                _ => unreachable!(),
            },
            GameState::Results(results) => todo!(),
        }
    }
}
