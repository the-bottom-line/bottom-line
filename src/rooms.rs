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
                IR::SelectedCharacter => internal::selected_character(selecting, player_name),
                _ => unreachable!(),
            },
            GameState::Round(round) => match msg {
                IR::SelectedCharacter => todo!("probably make it so this doesn't happen"),
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
                IR::TurnEnded { player_id } => todo!(),
                _ => unreachable!(),
            },
            GameState::Results(results) => todo!(),
        }

        // let state = &*self.game.lock().unwrap();
        // match msg {
        //     InternalResponse::GameStarted => {
        //         let player = state.player_by_name(player_name).unwrap();
        //         let pickable_characters = state.player_get_selectable_characters(player.id).ok();
        //         let selecting = state.selecting_characters().unwrap();
        //         Some(vec![
        //             UniqueResponse::StartGame {
        //                 id: player.id,
        //                 hand: player.hand.clone(),
        //                 cash: player.cash,
        //                 open_characters: selecting.open_characters().to_vec(),
        //                 player_info: selecting.player_info(player.id),
        //             },
        //             UniqueResponse::SelectingCharacters {
        //                 chairman_id: selecting.chairman,
        //                 pickable_characters,
        //                 turn_order: selecting.turn_order(),
        //             },
        //         ])
        //     }
        //     InternalResponse::SelectedCharacter => {
        //         let player = state.player_by_name(player_name).unwrap();
        //         let currently_picking_id = match state {
        //             GameState::SelectingCharacters(s) => Some(s.currently_selecting_id()),
        //             GameState::Round(_) => None,
        //             _ => unreachable!(),
        //         };

        //         let pickable_characters = state.player_get_selectable_characters(player.id).ok();

        //         let selected = UniqueResponse::SelectedCharacter {
        //             currently_picking_id,
        //             pickable_characters,
        //         };

        //         if let GameState::Round(round) = state {
        //             // started round
        //             Some(vec![
        //                 selected,
        //                 UniqueResponse::TurnStarts {
        //                     player_turn: round.current_player().id,
        //                     player_turn_cash: 1,
        //                     player_character: round.current_player().character.unwrap(),
        //                     draws_n_cards: 3,
        //                     skipped_characters: vec![],
        //                 },
        //             ])
        //         } else {
        //             Some(vec![selected])
        //         }
        //     }
        //     InternalResponse::DrawnCard {
        //         player_id,
        //         card_type,
        //     } => Some(vec![UniqueResponse::DrewCard {
        //         player_id,
        //         card_type,
        //     }]),
        //     InternalResponse::PutBackCard {
        //         player_id,
        //         card_type,
        //     } => Some(vec![UniqueResponse::PutBackCard {
        //         player_id,
        //         card_type,
        //     }]),
        //     InternalResponse::BoughtAsset { player_id, asset } => {
        //         Some(vec![UniqueResponse::BoughtAsset { player_id, asset }])
        //     }
        //     InternalResponse::IssuedLiability {
        //         player_id,
        //         liability,
        //     } => Some(vec![UniqueResponse::IssuedLiability {
        //         player_id,
        //         liability,
        //     }]),
        //     InternalResponse::TurnEnded { player_id } => {
        //         match state {
        //             GameState::Round(round) => {
        //                 Some(vec![
        //                     UniqueResponse::TurnEnded { player_id },
        //                     UniqueResponse::TurnStarts {
        //                         player_turn: round.current_player().id,
        //                         player_turn_cash: 1,
        //                         player_character: round.current_player().character.unwrap(),
        //                         draws_n_cards: 3,
        //                         // TODO: implement concept of skipped characters
        //                         skipped_characters: vec![],
        //                     },
        //                 ])
        //             }
        //             GameState::SelectingCharacters(selecting) => {
        //                 let player = state.player_by_name(player_name).unwrap();
        //                 let pickable_characters =
        //                     state.player_get_selectable_characters(player.id).ok();
        //                 Some(vec![UniqueResponse::SelectingCharacters {
        //                     chairman_id: selecting.chairman,
        //                     pickable_characters,
        //                     // player_info: state.player_info(player.id.into()),
        //                     turn_order: selecting.turn_order(),
        //                 }])
        //             }
        //             GameState::Results(_) => todo!(),
        //             GameState::Lobby(_) => unreachable!(),
        //         }
        //     }
        //     InternalResponse::PlayerJoined { username }
        //     | InternalResponse::PlayerLeft { username } => {
        //         let usernames = state.lobby().unwrap().players().clone();
        //         Some(vec![UniqueResponse::PlayersInLobby {
        //             changed_player: username,
        //             usernames,
        //         }])
        //     }
        // }
    }
}
