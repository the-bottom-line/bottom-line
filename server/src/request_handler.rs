use either::Either;
use game::{errors::GameError, game::*, player::*};
use responses::*;

use std::{collections::HashMap, path::PathBuf};

#[derive(Debug)]
pub struct Response(pub InternalResponse, pub DirectResponse);

#[derive(Clone, Debug)]
pub struct InternalResponse(pub HashMap<PlayerId, Vec<UniqueResponse>>);

impl InternalResponse {
    pub fn get_responses(&self, id: PlayerId) -> Option<&[UniqueResponse]> {
        self.0.get(&id).map(AsRef::as_ref)
    }

    pub fn into_inner(self) -> HashMap<PlayerId, Vec<UniqueResponse>> {
        self.0
    }
}

pub fn start_game(state: &mut GameState) -> Result<Response, GameError> {
    let path =
        PathBuf::from(std::env!("CARGO_MANIFEST_DIR")).join("../assets/cards/boardgame.json");
    state.start_game(path)?;

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
                        selectable_characters: selecting
                            .player_get_selectable_characters(p.id)
                            .ok(),
                        open_characters: selecting.open_characters().to_vec(),
                        closed_character: selecting.player_get_closed_character(p.id).ok(),
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

pub fn redeem_liability(
    state: &mut GameState,
    liability_idx: usize,
    player_id: PlayerId,
) -> Result<Response, GameError> {
    let round = state.round_mut()?;

    round.player_redeem_liability(player_id, liability_idx)?;

    let internal = round
        .players()
        .iter()
        .filter(|p| p.id != player_id)
        .map(|p| {
            (
                p.id,
                vec![UniqueResponse::RedeemedLiability {
                    player_id,
                    liability_idx,
                }],
            )
        })
        .collect();

    Ok(Response(
        InternalResponse(internal),
        DirectResponse::YouRedeemedLiability { liability_idx },
    ))
}

fn turn_starts(round: &Round) -> UniqueResponse {
    let current_player = round.current_player();

    UniqueResponse::TurnStarts {
        player_turn: current_player.id,
        player_turn_cash: current_player.turn_cash(round.current_market()),
        player_character: current_player.character,
        draws_n_cards: current_player.draws_n_cards(),
        playable_assets: current_player.playable_assets(),
        playable_liabilities: current_player.playable_liabilities(),
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
                                    selectable_characters: selecting
                                        .player_get_selectable_characters(p.id)
                                        .ok(),
                                    closed_character: selecting
                                        .player_get_closed_character(p.id)
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
                            selectable_characters: selecting
                                .player_get_selectable_characters(p.id)
                                .ok(),
                            open_characters: selecting.open_characters().to_vec(),
                            closed_character: selecting.player_get_closed_character(p.id).ok(),
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
    use super::*;

    #[test]
    fn fmt() {
        let action = FrontendRequest::StartGame;

        let action2 = FrontendRequest::DrawCard {
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
