use std::sync::Arc;

use crate::{
    cards::GameData,
    game::*,
    responses::*,
    server::{Game, RoomState},
};
use either::Either;

pub fn handle_internal_request(
    msg: InternalResponse,
    room_state: Arc<RoomState>,
    player_name: &str,
) -> Option<Vec<UniqueResponse>> {
    match &*room_state.game.lock().unwrap() {
        Game::GameStarted { state } => {
            let player = state.player_by_name(player_name).unwrap();
            match msg {
                InternalResponse::GameStarted => {
                    let pickable_characters = state
                        .player_get_selectable_characters(player.id.into())
                        .ok();
                    Some(vec![
                        UniqueResponse::StartGame {
                            id: player.id,
                            hand: player.hand.clone(),
                            cash: player.cash,
                            open_characters: state.open_characters().to_vec(),
                            player_info: state.player_info(player.id.into()),
                        },
                        UniqueResponse::SelectingCharacters {
                            chairman_id: state.chairman,
                            pickable_characters,
                            turn_order: state.turn_order(),
                        },
                    ])
                }
                InternalResponse::SelectedCharacter => {
                    let pickable_characters = state
                        .player_get_selectable_characters(player.id.into())
                        .ok();

                    let currently_picking_id = state.currently_selecting_id();

                    let selected = UniqueResponse::SelectedCharacter {
                        currently_picking_id,
                        pickable_characters,
                    };

                    if let Some(player) = state.current_player() {
                        // starting round
                        Some(vec![
                            selected,
                            UniqueResponse::TurnStarts {
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
                    if let Some(player) = state.current_player() {
                        Some(vec![
                            UniqueResponse::TurnEnded { player_id },
                            UniqueResponse::TurnStarts {
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
                        Some(vec![UniqueResponse::SelectingCharacters {
                            chairman_id: state.chairman,
                            pickable_characters,
                            // player_info: state.player_info(player.id.into()),
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
                Some(vec![UniqueResponse::PlayersInLobby {
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
                    DirectResponse::Error(ResponseError::GameAlreadyStarted).into()
                }
                ReceiveData::StartGame => {
                    DirectResponse::Error(ResponseError::GameAlreadyStarted).into()
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
                match GameState::new(&names, data) {
                    Ok(state) => {
                        *game = Game::GameStarted { state };
                        tracing::debug!("{msg:?}");
                        Response(
                            Some(InternalResponse::GameStarted),
                            DirectResponse::YouStartedGame,
                        )
                    }
                    Err(e) => {
                        tracing::error!("Failed to start game: {}", e);
                        e.into()
                    }
                }
            }
            _ => DirectResponse::Error(ResponseError::GameNotYetStarted).into(),
        },
    }
}

fn draw_card(state: &mut GameState, card_type: CardType, player_id: PlayerId) -> Response {
    let card = match state.player_draw_card(player_id.into(), card_type) {
        Ok(card) => card.cloned(),
        Err(e) => return e.into(),
    };

    let player = state.player(player_id.into()).unwrap();

    Response::new(
        InternalResponse::DrawnCard {
            player_id,
            card_type,
        },
        DirectResponse::YouDrewCard {
            card,
            can_draw_cards: player.can_draw_cards(),
            can_give_back_cards: player.should_give_back_cards(),
        },
    )
}

fn put_back_card(state: &mut GameState, card_idx: usize, player_id: PlayerId) -> Response {
    let card_type = match state.player_give_back_card(player_id.into(), card_idx) {
        Ok(card_type) => card_type,
        Err(e) => return e.into(),
    };

    let player = state.player(player_id.into()).unwrap();

    Response::new(
        InternalResponse::PutBackCard {
            player_id,
            card_type,
        },
        DirectResponse::YouPutBackCard {
            card_idx,
            can_draw_cards: player.can_draw_cards(),
            can_give_back_cards: player.should_give_back_cards(),
        },
    )
}

fn play_card(state: &mut GameState, card_idx: usize, player_id: PlayerId) -> Response {
    match state.player_play_card(player_id.into(), card_idx) {
        Ok(played_card) => match played_card.used_card {
            Either::Left(asset) => Response::new(
                InternalResponse::BoughtAsset {
                    player_id,
                    asset: asset.clone(),
                },
                DirectResponse::YouBoughtAsset { asset },
            ),
            Either::Right(liability) => Response::new(
                InternalResponse::IssuedLiability {
                    player_id,
                    liability: liability.clone(),
                },
                DirectResponse::YouIssuedLiability { liability },
            ),
        },
        Err(e) => e.into(),
    }
}

fn select_character(state: &mut GameState, character: Character, player_id: PlayerId) -> Response {
    match state.player_select_character(player_id.into(), character) {
        Ok(_) => Response::new(
            InternalResponse::SelectedCharacter,
            DirectResponse::YouSelectedCharacter { character },
        ),
        Err(e) => e.into(),
    }
}

fn end_turn(state: &mut GameState, player_id: PlayerId) -> Response {
    match state.end_player_turn(player_id.into()) {
        Ok(TurnEnded {
            next_player: Some(player_id),
        }) => Response(
            Some(InternalResponse::TurnEnded { player_id }),
            DirectResponse::YouEndedTurn,
        ),
        Ok(_) => {
            // if next_player is none // TODO: Fix for end of round
            let player_id = state.chairman;
            Response(
                Some(InternalResponse::TurnEnded { player_id }),
                DirectResponse::YouEndedTurn,
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

        let send = DirectResponse::YouPutBackCard {
            card_idx: 123,
            can_draw_cards: true,
            can_give_back_cards: true,
        };

        let sjson = serde_json::to_string(&send).unwrap();

        println!("send json: {sjson}");
    }
}
