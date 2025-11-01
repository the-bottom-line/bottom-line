use std::sync::Arc;

use crate::{game::*, game_errors::GameError, responses::*, server::RoomState};
use either::Either;

pub fn handle_internal_request(
    msg: InternalResponse,
    room_state: Arc<RoomState>,
    player_name: &str,
) -> Option<Vec<UniqueResponse>> {
    let state = &*room_state.game.lock().unwrap();
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
                    player_info: state.player_info(player.id),
                },
                UniqueResponse::SelectingCharacters {
                    chairman_id: selecting.chairman,
                    pickable_characters,
                    turn_order: state.turn_order(),
                },
            ])
        }
        InternalResponse::SelectedCharacter => {
            let player = state.player_by_name(player_name).unwrap();
            let pickable_characters = state.player_get_selectable_characters(player.id).ok();

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
                let player = state.player_by_name(player_name).unwrap();
                let pickable_characters = state.player_get_selectable_characters(player.id).ok();
                Some(vec![UniqueResponse::SelectingCharacters {
                    chairman_id: state.selecting_characters().unwrap().chairman,
                    pickable_characters,
                    // player_info: state.player_info(player.id.into()),
                    turn_order: state.turn_order(),
                }])
            }
        }
        InternalResponse::PlayerJoined { username } | InternalResponse::PlayerLeft { username } => {
            let usernames = state.lobby().unwrap().players().clone();
            Some(vec![UniqueResponse::PlayersInLobby {
                changed_player: username,
                usernames,
            }])
        }
    }
}

pub fn handle_request(msg: ReceiveData, room_state: Arc<RoomState>, player_name: &str) -> Response {
    let state = &mut *room_state.game.lock().unwrap();

    match msg {
        ReceiveData::StartGame => match state {
            GameState::Lobby(_) => match state.start_game("assets/cards/boardgame.json") {
                Ok(_) => {
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
            },
            _ => GameError::NotLobbyState.into(),
        },
        ReceiveData::SelectCharacter { character } => match state {
            GameState::SelectingCharacters(_) => {
                let player_id = state.player_by_name(player_name).unwrap().id;
                select_character(state, character, player_id)
            }
            _ => GameError::NotSelectingCharactersState.into(),
        },
        ReceiveData::DrawCard { card_type } => match state {
            GameState::Round(_) => {
                let player_id = state.player_by_name(player_name).unwrap().id;
                draw_card(state, card_type, player_id)
            }
            _ => GameError::NotRoundState.into(),
        },
        ReceiveData::PutBackCard { card_idx } => match state {
            GameState::Round(_) => {
                let player_id = state.player_by_name(player_name).unwrap().id;
                put_back_card(state, card_idx, player_id)
            }
            _ => GameError::NotRoundState.into(),
        },
        ReceiveData::BuyAsset { card_idx } => match state {
            GameState::Round(_) => {
                let player_id = state.player_by_name(player_name).unwrap().id;
                play_card(state, card_idx, player_id)
            }
            _ => GameError::NotRoundState.into(),
        },
        ReceiveData::IssueLiability { card_idx } => match state {
            GameState::Round(_) => {
                let player_id = state.player_by_name(player_name).unwrap().id;
                play_card(state, card_idx, player_id)
            }
            _ => GameError::NotRoundState.into(),
        },
        ReceiveData::EndTurn => match state {
            GameState::Round(_) => {
                let player_id = state.player_by_name(player_name).unwrap().id;
                end_turn(state, player_id)
            }
            _ => GameError::NotRoundState.into(),
        },
    }
}

fn draw_card(state: &mut GameState, card_type: CardType, player_id: PlayerId) -> Response {
    let card = match state.player_draw_card(player_id, card_type) {
        Ok(card) => card.cloned(),
        Err(e) => return e.into(),
    };

    let player = state.player(player_id).unwrap();

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
    let card_type = match state.player_give_back_card(player_id, card_idx) {
        Ok(card_type) => card_type,
        Err(e) => return e.into(),
    };

    let player = state.player(player_id).unwrap();

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
    match state.player_play_card(player_id, card_idx) {
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
    match state.player_select_character(player_id, character) {
        Ok(_) => Response::new(
            InternalResponse::SelectedCharacter,
            DirectResponse::YouSelectedCharacter { character },
        ),
        Err(e) => e.into(),
    }
}

fn end_turn(state: &mut GameState, player_id: PlayerId) -> Response {
    match state.end_player_turn(player_id) {
        Ok(TurnEnded {
            next_player: Some(player_id),
        }) => Response(
            Some(InternalResponse::TurnEnded { player_id }),
            DirectResponse::YouEndedTurn,
        ),
        Ok(_) => {
            // if next_player is none // TODO: Fix for end of round
            let player_id = state.round().unwrap().chairman;
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
