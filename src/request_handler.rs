use crate::{errors::GameError, game::*, player::*, responses::*, rooms::RoomState};
use either::Either;

pub fn handle_request(msg: ReceiveData, room_state: &RoomState, player_name: &str) -> Response {
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
