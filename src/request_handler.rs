use std::{collections::HashSet, sync::Arc};

use crate::{
    cards::GameData, game::*, game_errors::GameError, server::{Game, RoomState}, utility::serde_asset_liability
};
use either::Either;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum ReceiveData {
    Connect { username: String, channel: String },
    StartGame,
    DrawCard { card_type: CardType },
    PutBackCard { card_idx: usize },
    BuyAsset { asset_idx: usize },
    IssueLiability { liability_idx: usize },
    GetSelectableCharacters,
    SelectCharacter { character: Character },
    EndTurn,
}

#[derive(Debug, Serialize)]
pub struct Response(pub InternalResponse, pub ExternalResponse);

impl Response {
    pub fn new(internal: InternalResponse, external: ExternalResponse) -> Self {
        Self(internal, external)
    }
}

impl From<ExternalResponse> for Response {
    fn from(external: ExternalResponse) -> Self {
        Self::new(InternalResponse::ActionPerformed, external)
    }
}

impl From<GameError> for Response {
    fn from(error: GameError) -> Self {
        Self::new(InternalResponse::ActionPerformed, ExternalResponse::Error(error.into()))
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "action", content = "data")]
pub enum ExternalResponse {
    Error(ResponseError),
    GameStartedOk,
    PlayersInLobby {
        usernames: HashSet<String>,
    },
    StartGame {
        cash: u8,
        #[serde(with = "serde_asset_liability::vec")]
        hand: Vec<Either<Asset, Liability>>,
        pickable_characters: Option<PickableCharacters>,
    },
    DrawnCard {
        #[serde(with = "serde_asset_liability::value")]
        card: Either<Asset, Liability>,
    },
    PutBackCard {
        remove_idx: Option<usize>,
    },
    BuyAssetOk,
    IssuedLiabilityOk,
    SelectableCharacters {
        pickable_characters: PickableCharacters,
    },
    SelectCharacterOk,
    EndedTurnOk,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum InternalResponse {
    ActionPerformed, // all-round placeholder
    PlayerJoined {
        username: String,
    },
    PlayerLeft {
        username: String,
    },
    GameStarted,
    DrawnCard {
        // #[serde(flatten)]
        player_id: PlayerId,
        card_type: CardType,
    },
    PutBackCard {
        // #[serde(flatten)]
        player_id: PlayerId,
        card_type: CardType,
    },
    BoughtAsset {
        // #[serde(flatten)]
        player_id: PlayerId,
        asset: Asset,
    },
    IssuedLiability {
        // #[serde(flatten)]
        player_id: PlayerId,
        liability: Liability,
    },
    SelectedCharacter {
        // #[serde(flatten)]
        player_id: PlayerId,
    },
    EndedTurn {
        player_id: PlayerId,
    },
    NewChairman {
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
}

pub fn handle_public_request(
    msg: InternalResponse,
    room_state: Arc<RoomState>,
    player_name: &str,
) -> Option<ExternalResponse> {
    match &*room_state.game.lock().unwrap() {
        Game::GameStarted { state } => {
            let player = state.player_by_name(&player_name).unwrap();
            match msg {
                InternalResponse::GameStarted => {
                    let hand = player.hand.clone();
                    let cash = player.cash;
                    let pickable_characters = state
                        .player_get_selectable_characters(player.id.into()).ok();
                    Some(ExternalResponse::StartGame {
                        hand,
                        cash,
                        pickable_characters,
                    })
                }
                _ => None,
            }
        }
        Game::InLobby { user_set } => match msg {
            InternalResponse::PlayerJoined { .. } | InternalResponse::PlayerLeft { .. } => {
                Some(ExternalResponse::PlayersInLobby {
                    usernames: user_set.clone(),
                })
            }
            _ => None,
        },
    }
}

pub fn handle_request(msg: ReceiveData, room_state: Arc<RoomState>, player_name: &str) -> Response {
    let mut game = room_state.game.lock().unwrap();
    match &mut *game {
        crate::server::Game::GameStarted { state } => {
            let playerid = state.player_by_name(player_name).unwrap().id.into();
            match msg {
                ReceiveData::Connect { .. } => ExternalResponse::Error(ResponseError::GameAlreadyStarted).into(),
                ReceiveData::StartGame => ExternalResponse::Error(ResponseError::GameAlreadyStarted).into(),
                ReceiveData::DrawCard { card_type } => draw_card(state, card_type, playerid),
                ReceiveData::PutBackCard { card_idx } => put_back_card(state, card_idx, playerid),
                ReceiveData::BuyAsset { asset_idx } => play_card(state, asset_idx, playerid),
                ReceiveData::IssueLiability { liability_idx } => {
                    play_card(state, liability_idx, playerid)
                }
                ReceiveData::GetSelectableCharacters => get_selectable_characters(state, playerid),
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
                    InternalResponse::GameStarted,
                    ExternalResponse::GameStartedOk,
                )
            }
            _ => ExternalResponse::Error(ResponseError::GameNotYetStarted).into(),
        },
    }
}

fn draw_card(state: &mut GameState, t: CardType, player_idx: usize) -> Response {
    match state.player_draw_card(player_idx, t) {
        Ok(card) => Response::new(
            InternalResponse::DrawnCard {
                player_id: player_idx.into(),
                card_type: t,
            },
            ExternalResponse::DrawnCard {
                card: card.cloned(),
            },
        ),
        Err(e) => e.into()
    }
}

fn put_back_card(state: &mut GameState, card_idx: usize, player_idx: usize) -> Response {
    match state.player_give_back_card(player_idx, card_idx) {
        Ok(card_type) => Response::new(
            InternalResponse::PutBackCard {
                player_id: player_idx.into(),
                card_type,
            },
            ExternalResponse::PutBackCard {
                remove_idx: Some(card_idx),
            },
        ),
        Err(e) => e.into()
    }
}

fn play_card(state: &mut GameState, card_idx: usize, player_idx: usize) -> Response {
    match state.player_play_card(player_idx, card_idx) {
        Ok(played_card) => match played_card.used_card {
            Either::Left(asset) => {
                return Response::new(
                    InternalResponse::BoughtAsset {
                        player_id: player_idx.into(),
                        asset: asset,
                    },
                    ExternalResponse::BuyAssetOk,
                );
            }
            Either::Right(liability) => {
                return Response::new(
                    InternalResponse::IssuedLiability {
                        player_id: player_idx.into(),
                        liability: liability,
                    },
                    ExternalResponse::IssuedLiabilityOk,
                );
            }
        },
        Err(e) => e.into()
    }
}

fn select_character(state: &mut GameState, character: Character, player_idx: usize) -> Response {
    match state.player_select_character(player_idx, character) {
        Ok(_) => Response::new(
            InternalResponse::SelectedCharacter {
                player_id: player_idx.into(),
            },
            ExternalResponse::SelectCharacterOk,
        ),
        Err(e) => e.into(),
    }
}

fn get_selectable_characters(state: &mut GameState, player_idx: usize) -> Response {
    match state.player_get_selectable_characters(player_idx)
    {
        Ok(pickable_characters) => Response::new(
            InternalResponse::ActionPerformed,
            ExternalResponse::SelectableCharacters {
                pickable_characters,
            },
        ),
        Err(e) => e.into()
    }
}

fn end_turn(state: &mut GameState, player_idx: usize) -> Response {
    match state.end_player_turn(player_idx) {
        Ok(TurnEnded {
            next_player: Some(player_id),
        }) => Response(
            InternalResponse::EndedTurn { player_id },
            ExternalResponse::EndedTurnOk,
        ),
        Ok(_) => {
            let player_id = state.chairman().id;
            Response(
                InternalResponse::NewChairman { player_id },
                ExternalResponse::EndedTurnOk,
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

        let send = ExternalResponse::PutBackCard { remove_idx: None };

        let sjson = serde_json::to_string(&send).unwrap();

        println!("send json: {sjson}");
    }
}
