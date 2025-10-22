use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{
    cards::GameData,
    game::*,
    game_errors::{GameError, SelectableCharactersError},
    server::{AppState, Game, RoomState},
    targeted_responses::*,
    utility::serde_asset_liability::{self, vec},
};
use either::Either;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum ReceiveData {
    Connect { username: String, channel: String },
    StartGame,
    SelectCharacter { character: Character },
    DrawCard { card_type: CardType },
    PutBackCard { card_idx: usize },
    BuyAsset { card_idx: usize },
    IssueLiability { card_idx: usize },
    // GetSelectableCharacters,
    EndTurn,
}

#[derive(Debug)]
pub struct Response(pub Option<InternalResponse>, pub PersonalResponse);

impl Response {
    pub fn new(internal: Option<InternalResponse>, external: PersonalResponse) -> Self {
        Self(internal, external)
    }
}

impl From<PersonalResponse> for Response {
    fn from(external: PersonalResponse) -> Self {
        Self::new(None, external)
    }
}

impl From<GameError> for Response {
    fn from(error: GameError) -> Self {
        Self::new(None, PersonalResponse::Error(error.into()))
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "action", content = "data")]
pub enum PersonalResponse {
    Error(ResponseError),
    GameStarted,
    SelectedCharacter {
        character: Character,
    },
    DrawnCard {
        #[serde(with = "serde_asset_liability::value")]
        card: Either<Asset, Liability>,
    },
    PutBackCard {
        card_idx: usize,
    },
    BoughtAsset {
        asset: Asset,
    },
    IssuedLiability {
        liability: Liability,
    },
    EndedTurn,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "action", content = "data")]
pub enum TargetedResponse {
    PlayersInLobby {
        changed_player: String,
        usernames: HashSet<String>,
    },
    StartGame {
        cash: u8,
        #[serde(with = "serde_asset_liability::vec")]
        hand: Vec<Either<Asset, Liability>>,
    },
    SelectingCharacters {
        chairman_id: PlayerId,
        pickable_characters: Option<PickableCharacters>,
        player_info: Vec<PlayerInfo>,
        turn_order: Vec<PlayerId>,
    },
    SelectedCharacter {
        player_id: PlayerId,
        character: Character,
        pickable_characters: Option<PickableCharacters>,
    },
    TurnStarts {
        /// Id of the player whose turn it is
        player_turn: PlayerId,
        /// Extra cash received by the player whose turn it is
        player_turn_cash: u8,
        player_character: Character,
        draws_n_cards: u8,
        skipped_characters: Vec<Character>,
    },
    DrawnCard {
        player_id: PlayerId,
        card_type: CardType,
    },
    PutBackCard {
        player_id: PlayerId,
        card_type: CardType,
    },
    BoughtAsset {
        player_id: PlayerId,
        asset: Asset,
    },
    IssuedLiability {
        player_id: PlayerId,
        liability: Liability,
    },
    TurnEnded {
        player_id: PlayerId,
    },
    /// Always sent with SelectingCharacters?
    RoundEnded,
    GameEnded,
}

#[derive(Clone, Debug)]
pub enum InternalResponse {
    PlayerJoined {
        username: String,
    },
    PlayerLeft {
        username: String,
    },
    GameStarted,
    SelectedCharacter {
        player_id: PlayerId,
        character: Character,
    },
    DrawnCard {
        player_id: PlayerId,
        card_type: CardType,
    },
    PutBackCard {
        player_id: PlayerId,
        card_type: CardType,
    },
    BoughtAsset {
        player_id: PlayerId,
        asset: Asset,
    },
    IssuedLiability {
        player_id: PlayerId,
        liability: Liability,
    },
    EndedTurn {
        player_id: PlayerId,
    },
}

#[derive(Debug, Error, Serialize)]
pub enum ResponseError {
    #[error(transparent)]
    Game(#[from] GameError),
    #[error("Game has not yet started")]
    GameNotYetStarted,
    #[error("Game has already started")]
    GameAlreadyStarted,
    #[error("Username already taken")]
    UsernameAlreadyTaken,
    #[error("Username is invalid")]
    InvalidUsername,
    #[error("Data is not valid for this state")]
    InvalidData,
}

impl<I: Into<ResponseError>> From<I> for PersonalResponse {
    fn from(error: I) -> Self {
        PersonalResponse::Error(error.into())
    }
}

pub fn handle_public_request(
    msg: InternalResponse,
    room_state: Arc<RoomState>,
    player_name: &str,
) -> Option<Vec<TargetedResponse>> {
    match &*room_state.game.lock().unwrap() {
        Game::GameStarted { state } => {
            let player = state.player_by_name(&player_name).unwrap();
            match msg {
                InternalResponse::GameStarted => {
                    let hand = player.hand.clone();
                    let cash = player.cash;
                    let pickable_characters = state
                        .player_get_selectable_characters(player.id.into())
                        .ok();
                    Some(vec![
                        TargetedResponse::StartGame { hand, cash },
                        TargetedResponse::SelectingCharacters {
                            chairman_id: state.chairman().id,
                            pickable_characters,
                            player_info: state.player_info(player.id.into()),
                            turn_order: state.turn_order(),
                        },
                    ])
                }
                InternalResponse::SelectedCharacter {
                    player_id,
                    character,
                } => {
                    let pickable_characters = state
                        .player_get_selectable_characters(player.id.into())
                        .ok();
                    let selected = TargetedResponse::SelectedCharacter {
                        player_id,
                        character,
                        pickable_characters,
                    };

                    if let Some(player) = state.current_player() {
                        // starting round
                        Some(vec![
                            selected,
                            TargetedResponse::TurnStarts {
                                player_turn: player.id,
                                player_turn_cash: 1,
                                player_character: player.character.unwrap(),
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
                } => Some(vec![TargetedResponse::DrawnCard {
                    player_id,
                    card_type,
                }]),
                InternalResponse::PutBackCard {
                    player_id,
                    card_type,
                } => Some(vec![TargetedResponse::PutBackCard {
                    player_id,
                    card_type,
                }]),
                InternalResponse::BoughtAsset { player_id, asset } => {
                    Some(vec![TargetedResponse::BoughtAsset { player_id, asset }])
                }
                InternalResponse::IssuedLiability {
                    player_id,
                    liability,
                } => Some(vec![TargetedResponse::IssuedLiability {
                    player_id,
                    liability,
                }]),
                InternalResponse::EndedTurn { player_id } => {
                    if let Some(player) = state.current_player() {
                        Some(vec![
                            TargetedResponse::TurnEnded { player_id },
                            TargetedResponse::TurnStarts {
                                player_turn: player.id,
                                player_turn_cash: 1,
                                player_character: player.character.unwrap(),
                                draws_n_cards: 3,
                                skipped_characters: vec![],
                            },
                        ])
                    } else {
                        let pickable_characters = state
                            .player_get_selectable_characters(player.id.into())
                            .ok();
                        Some(vec![TargetedResponse::SelectingCharacters {
                            chairman_id: state.chairman().id,
                            pickable_characters,
                            player_info: state.player_info(player.id.into()),
                            turn_order: state.turn_order(),
                        }])
                    }
                }
                InternalResponse::PlayerJoined { .. } => None,
                InternalResponse::PlayerLeft { .. } => None,
            }
        }
        Game::InLobby { user_set } => match msg {
            InternalResponse::PlayerJoined { username }
            | InternalResponse::PlayerLeft { username } => {
                Some(vec![TargetedResponse::PlayersInLobby {
                    changed_player: username,
                    usernames: user_set.clone(),
                }])
            }
            _ => None,
        },
    }
}

pub fn handle_request(msg: ReceiveData, room_state: Arc<RoomState>, player_name: &str) -> Response {
    let mut game = room_state.game.lock().unwrap();
    match &mut *game {
        crate::server::Game::GameStarted { state } => {
            let playerid = state.player_by_name(player_name).unwrap().id;
            match msg {
                ReceiveData::Connect { .. } => {
                    PersonalResponse::Error(ResponseError::GameAlreadyStarted).into()
                }
                ReceiveData::StartGame => {
                    PersonalResponse::Error(ResponseError::GameAlreadyStarted).into()
                }
                ReceiveData::DrawCard { card_type } => draw_card(state, card_type, playerid),
                ReceiveData::PutBackCard { card_idx } => put_back_card(state, card_idx, playerid),
                ReceiveData::BuyAsset {
                    card_idx: asset_idx,
                } => play_card(state, asset_idx, playerid),
                ReceiveData::IssueLiability {
                    card_idx: liability_idx,
                } => play_card(state, liability_idx, playerid),
                ReceiveData::SelectCharacter { character } => {
                    select_character(state, character, playerid)
                }
                ReceiveData::EndTurn => end_turn(state, playerid),
            }
        }
        crate::server::Game::InLobby { user_set } => match msg {
            ReceiveData::StartGame => {
                let names = user_set.iter().cloned().collect::<Vec<_>>();
                let data = GameData::new("assets/cards/boardgame.json").expect("this should exist");
                let state = GameState::new(&names, data);
                *game = Game::GameStarted { state };
                tracing::debug!("{msg:?}");
                Response(
                    Some(InternalResponse::GameStarted),
                    PersonalResponse::GameStarted,
                )
            }
            _ => PersonalResponse::Error(ResponseError::GameNotYetStarted).into(),
        },
    }
}

fn draw_card(state: &mut GameState, card_type: CardType, player_id: PlayerId) -> Response {
    match state.player_draw_card(player_id.into(), card_type) {
        Ok(card) => Response::new(
            Some(InternalResponse::DrawnCard {
                player_id,
                card_type,
            }),
            PersonalResponse::DrawnCard {
                card: card.cloned(),
            },
        ),
        Err(e) => e.into(),
    }
}

fn put_back_card(state: &mut GameState, card_idx: usize, player_id: PlayerId) -> Response {
    match state.player_give_back_card(player_id.into(), card_idx) {
        Ok(card_type) => Response::new(
            Some(InternalResponse::PutBackCard {
                player_id,
                card_type,
            }),
            PersonalResponse::PutBackCard { card_idx },
        ),
        Err(e) => e.into(),
    }
}

fn play_card(state: &mut GameState, card_idx: usize, player_id: PlayerId) -> Response {
    match state.player_play_card(player_id.into(), card_idx) {
        Ok(played_card) => match played_card.used_card {
            Either::Left(asset) => {
                return Response::new(
                    Some(InternalResponse::BoughtAsset {
                        player_id,
                        asset: asset.clone(),
                    }),
                    PersonalResponse::BoughtAsset { asset },
                );
            }
            Either::Right(liability) => {
                return Response::new(
                    Some(InternalResponse::IssuedLiability {
                        player_id,
                        liability: liability.clone(),
                    }),
                    PersonalResponse::IssuedLiability { liability },
                );
            }
        },
        Err(e) => e.into(),
    }
}

fn select_character(state: &mut GameState, character: Character, player_id: PlayerId) -> Response {
    match state.player_select_character(player_id.into(), character) {
        Ok(_) => Response::new(
            Some(InternalResponse::SelectedCharacter {
                player_id,
                character,
            }),
            PersonalResponse::SelectedCharacter { character },
        ),
        Err(e) => e.into(),
    }
}



fn end_turn(state: &mut GameState, player_id: PlayerId) -> Response {
    match state.end_player_turn(player_id.into()) {
        Ok(TurnEnded {
            next_player: Some(player_id),
        }) => Response(
            Some(InternalResponse::EndedTurn { player_id }),
            PersonalResponse::EndedTurn,
        ),
        Ok(_) => {
            // if next_player is none // TODO: Fix for end of round
            let player_id = state.chairman().id;
            Response(
                Some(InternalResponse::EndedTurn { player_id }),
                PersonalResponse::EndedTurn,
            )
        }
        Err(e) => e.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt() {
        let action = ReceiveData::StartGame;

        let action2 = ReceiveData::DrawCard {
            card_type: CardType::Asset,
        };

        let json = serde_json::to_string(&action).unwrap();
        let json2 = serde_json::to_string(&action2).unwrap();

        println!("json: {json}");
        println!("json2: {json2}");

        let send = PersonalResponse::PutBackCard { card_idx: 123 };

        let sjson = serde_json::to_string(&send).unwrap();

        println!("send json: {sjson}");
    }
}
