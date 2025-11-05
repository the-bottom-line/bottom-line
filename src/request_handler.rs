use either::Either;

use crate::{errors::GameError, game::*, player::*, responses::*};

pub fn start_game(state: &mut GameState) -> Result<Response, GameError> {
    state.start_game("assets/cards/boardgame.json")?;

    tracing::debug!("Started Game");

    let selecting = state.selecting_characters()?;

    let internal = selecting
        .players()
        .iter()
        .map(|p| {
            (
                p.id,
                vec![
                    UniqueResponse::StartGame {
                        id: p.id,
                        hand: p.hand.clone(),
                        cash: p.cash,
                        player_info: selecting.player_info(p.id),
                    },
                    UniqueResponse::SelectingCharacters {
                        chairman_id: selecting.chairman,
                        pickable_characters: selecting.player_get_selectable_characters(p.id).ok(),
                        open_characters: selecting.open_characters().to_vec(),
                        turn_order: selecting.turn_order(),
                    },
                ],
            )
        })
        .collect();

    Ok(Response(
        InternalResponse(internal),
        DirectResponse::YouStartedGame,
    ))
}

pub fn draw_card(
    state: &mut GameState,
    card_type: CardType,
    player_id: PlayerId,
) -> Result<Response, GameError> {
    let round = state.round_mut()?;
    let card = round.player_draw_card(player_id, card_type)?.cloned();
    let player = round.player(player_id)?;

    let internal = round
        .players()
        .iter()
        .filter(|p| p.id != player_id)
        .map(|p| {
            (
                p.id,
                vec![UniqueResponse::DrewCard {
                    player_id,
                    card_type,
                }],
            )
        })
        .collect();

    Ok(Response(
        InternalResponse(internal),
        DirectResponse::YouDrewCard {
            card,
            can_draw_cards: player.can_draw_cards(),
            can_give_back_cards: player.should_give_back_cards(),
        },
    ))
}

pub fn put_back_card(
    state: &mut GameState,
    card_idx: usize,
    player_id: PlayerId,
) -> Result<Response, GameError> {
    let round = state.round_mut()?;
    let card_type = round.player_give_back_card(player_id, card_idx)?;
    let player = round.player(player_id)?;

    let internal = round
        .players()
        .iter()
        .filter(|p| p.id != player_id)
        .map(|p| {
            (
                p.id,
                vec![UniqueResponse::PutBackCard {
                    player_id,
                    card_type,
                }],
            )
        })
        .collect();

    Ok(Response(
        InternalResponse(internal),
        DirectResponse::YouPutBackCard {
            card_idx,
            can_draw_cards: player.can_draw_cards(),
            can_give_back_cards: player.should_give_back_cards(),
        },
    ))
}

pub fn play_card(
    state: &mut GameState,
    card_idx: usize,
    player_id: PlayerId,
) -> Result<Response, GameError> {
    let round = state.round_mut()?;
    let played_card = round.player_play_card(player_id, card_idx)?;

    match played_card.used_card {
        Either::Left(asset) => {
            let internal = round
                .players()
                .iter()
                .filter(|p| p.id != player_id)
                .map(|p| {
                    (
                        p.id,
                        vec![UniqueResponse::BoughtAsset {
                            player_id,
                            asset: asset.clone(),
                        }],
                    )
                })
                .collect();

            Ok(Response(
                InternalResponse(internal),
                DirectResponse::YouBoughtAsset { asset },
            ))
        }
        Either::Right(liability) => {
            let internal = round
                .players()
                .iter()
                .filter(|p| p.id != player_id)
                .map(|p| {
                    (
                        p.id,
                        vec![UniqueResponse::IssuedLiability {
                            player_id,
                            liability: liability.clone(),
                        }],
                    )
                })
                .collect();

            Ok(Response(
                InternalResponse(internal),
                DirectResponse::YouIssuedLiability { liability },
            ))
        }
    }
}

fn turn_starts(round: &Round) -> UniqueResponse {
    let current_player = round.current_player();

    UniqueResponse::TurnStarts {
        player_turn: current_player.id,
        player_turn_cash: current_player.turn_cash(),
        player_character: current_player.character,
        draws_n_cards: current_player.draws_n_cards(),
        skipped_characters: round.skipped_characters(),
    }
}

pub fn select_character(
    state: &mut GameState,
    player_id: PlayerId,
    character: Character,
) -> Result<Response, GameError> {
    match state.player_select_character(player_id, character) {
        Ok(_) => {
            match state {
                GameState::Lobby(_) => Err(GameError::NotAvailableInLobbyState),
                GameState::SelectingCharacters(selecting) => {
                    let internal = selecting
                        .players()
                        .iter()
                        .map(|p| {
                            (
                                p.id,
                                vec![UniqueResponse::SelectedCharacter {
                                    currently_picking_id: Some(selecting.currently_selecting_id()),
                                    pickable_characters: selecting
                                        .player_get_selectable_characters(p.id)
                                        .ok(),
                                }],
                            )
                        })
                        .collect();

                    Ok(Response(
                        InternalResponse(internal),
                        DirectResponse::YouSelectedCharacter { character },
                    ))
                }
                GameState::Round(round) => {
                    // TODO: turn is the same for everyone. Simplify maybe
                    let internal = round
                        .players()
                        .iter()
                        .map(|p| (p.id, vec![turn_starts(round)]))
                        .collect();

                    Ok(Response(
                        InternalResponse(internal),
                        DirectResponse::YouSelectedCharacter { character },
                    ))
                }
                GameState::Results(_) => Err(GameError::NotAvailableInResultsState),
            }
        }
        Err(e) => Err(e),
    }
}

pub fn end_turn(state: &mut GameState, player_id: PlayerId) -> Result<Response, GameError> {
    state.end_player_turn(player_id)?;

    match state {
        GameState::Lobby(_) => Err(GameError::NotAvailableInLobbyState),
        GameState::SelectingCharacters(selecting) => {
            let internal = selecting
                .players()
                .iter()
                .map(|p| {
                    (
                        p.id,
                        vec![UniqueResponse::SelectingCharacters {
                            chairman_id: selecting.chairman,
                            pickable_characters: selecting
                                .player_get_selectable_characters(p.id)
                                .ok(),
                            open_characters: selecting.open_characters().to_vec(),
                            turn_order: selecting.turn_order(),
                        }],
                    )
                })
                .collect();

            Ok(Response(
                InternalResponse(internal),
                DirectResponse::YouEndedTurn,
            ))
        }
        GameState::Round(round) => {
            let internal = round
                .players()
                .iter()
                .map(|p| (p.id, vec![turn_starts(round)]))
                .collect();

            Ok(Response(
                InternalResponse(internal),
                DirectResponse::YouEndedTurn,
            ))
        }
        GameState::Results(_) => Err(GameError::NotAvailableInResultsState),
    }
}

#[cfg(test)]
mod tests {
    use crate::{player::*, responses::*};

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
