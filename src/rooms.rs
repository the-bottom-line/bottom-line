use std::sync::Mutex;

use axum::Json;
use tokio::sync::broadcast;

use crate::{
    game::{GameState, TheBottomLine},
    responses::*,
};

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

    pub fn handle_internal_request(
        &self,
        msg: InternalResponse,
        player_name: &str,
    ) -> Option<Vec<UniqueResponse>> {
        let state = &*self.game.lock().unwrap();
        match msg {
            InternalResponse::GameStarted => {
                let player = state.player_by_name(player_name).unwrap();
                let pickable_characters = state.player_get_selectable_characters(player.id).ok();
                let selecting = state.selecting_characters().unwrap();
                Some(vec![
                    UniqueResponse::StartGame {
                        id: player.id,
                        hand: player.hand.clone(),
                        cash: player.cash,
                        open_characters: selecting.open_characters().to_vec(),
                        player_info: selecting.player_info(player.id),
                    },
                    UniqueResponse::SelectingCharacters {
                        chairman_id: selecting.chairman,
                        pickable_characters,
                        turn_order: selecting.turn_order(),
                    },
                ])
            }
            InternalResponse::SelectedCharacter => {
                let player = state.player_by_name(player_name).unwrap();
                let currently_picking_id = match state {
                    GameState::SelectingCharacters(s) => Some(s.currently_selecting_id()),
                    GameState::Round(_) => None,
                    _ => unreachable!(),
                };

                let pickable_characters = state.player_get_selectable_characters(player.id).ok();

                let selected = UniqueResponse::SelectedCharacter {
                    currently_picking_id,
                    pickable_characters,
                };

                if let GameState::Round(round) = state {
                    // started round
                    Some(vec![
                        selected,
                        UniqueResponse::TurnStarts {
                            player_turn: round.current_player().id,
                            player_turn_cash: 1,
                            player_character: round.current_player().character.unwrap(),
                            draws_n_cards: 3,
                            skipped_characters: vec![],
                        },
                    ])
                } else {
                    Some(vec![selected])
                }
            }
            InternalResponse::DrawnCard {
                player_id,
                card_type,
            } => Some(vec![UniqueResponse::DrewCard {
                player_id,
                card_type,
            }]),
            InternalResponse::PutBackCard {
                player_id,
                card_type,
            } => Some(vec![UniqueResponse::PutBackCard {
                player_id,
                card_type,
            }]),
            InternalResponse::BoughtAsset { player_id, asset } => {
                Some(vec![UniqueResponse::BoughtAsset { player_id, asset }])
            }
            InternalResponse::IssuedLiability {
                player_id,
                liability,
            } => Some(vec![UniqueResponse::IssuedLiability {
                player_id,
                liability,
            }]),
            InternalResponse::TurnEnded { player_id } => {
                match state {
                    GameState::Round(round) => {
                        Some(vec![
                            UniqueResponse::TurnEnded { player_id },
                            UniqueResponse::TurnStarts {
                                player_turn: round.current_player().id,
                                player_turn_cash: 1,
                                player_character: round.current_player().character.unwrap(),
                                draws_n_cards: 3,
                                // TODO: implement concept of skipped characters
                                skipped_characters: vec![],
                            },
                        ])
                    }
                    GameState::SelectingCharacters(selecting) => {
                        let player = state.player_by_name(player_name).unwrap();
                        let pickable_characters =
                            state.player_get_selectable_characters(player.id).ok();
                        Some(vec![UniqueResponse::SelectingCharacters {
                            chairman_id: selecting.chairman,
                            pickable_characters,
                            // player_info: state.player_info(player.id.into()),
                            turn_order: selecting.turn_order(),
                        }])
                    }
                    GameState::Results(_) => todo!(),
                    GameState::Lobby(_) => unreachable!(),
                }
            }
            InternalResponse::PlayerJoined { username }
            | InternalResponse::PlayerLeft { username } => {
                let usernames = state.lobby().unwrap().players().clone();
                Some(vec![UniqueResponse::PlayersInLobby {
                    changed_player: username,
                    usernames,
                }])
            }
        }
    }
}
