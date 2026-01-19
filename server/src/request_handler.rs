use either::Either;
use game::{errors::*, game::*, player::*};
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
    let assets_path = std::env::var("ASSETS_DIR")
        .unwrap_or_else(|_| format!("{}/../assets/", env!("CARGO_MANIFEST_DIR")));
    let path = PathBuf::from(assets_path).join("cards/boardgame.json");

    state.start_game(path)?;

    tracing::debug!("Started Game");

    let selecting = state.selecting_characters()?;

    let internal = selecting
        .players()
        .iter()
        .map(|p| {
            (
                p.id(),
                vec![
                    UniqueResponse::StartGame {
                        id: p.id(),
                        hand: p.hand().to_vec(),
                        cash: p.cash(),
                        player_info: selecting.player_info(p.id()),
                        initial_market: selecting.current_market().clone(),
                    },
                    UniqueResponse::SelectingCharacters {
                        chairman_id: selecting.chairman_id(),
                        selectable_characters: selecting
                            .player_get_selectable_characters(p.id())
                            .ok(),
                        open_characters: selecting.open_characters().to_vec(),
                        closed_character: selecting.player_get_closed_character(p.id()).ok(),
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

pub fn use_ability(state: &mut GameState, player_id: PlayerId) -> Result<Response, GameError> {
    let round = state.round_mut()?;
    let player = round.player(player_id)?;
    match player.character() {
        Character::Shareholder if round.current_player().id() == player.id() => Ok(Response(
            InternalResponse(std::collections::HashMap::new()),
            DirectResponse::YouAreFiringSomeone {
                characters: round.player_get_fireble_characters(),
                 character: Character::Shareholder,
                perk: "You can fire a character \n- A fired character skips their turs ".to_string(),
            },
        )),
        Character::Banker if round.current_player().id() == player.id() => Ok(Response(
            InternalResponse(std::collections::HashMap::new()),
            DirectResponse::YouAreTerminatingSomeone {
                characters: round.player_get_fireble_characters(),
                 character: Character::Banker,
                perk: "You can force a player to give you cash based on the amount of different color assets they have +1".to_string(),
            },
        )),
        Character::Regulator if round.current_player().id() == player.id() => Ok(Response(
            InternalResponse(std::collections::HashMap::new()),
            DirectResponse::YouRegulatorOptions {
                options: round.player_get_regulator_swap_players(),
                character: Character::Regulator,
                perk: "You can swap your hand with another player or swap any number of cards with the deck".to_string(),
             }
        )),
        Character::CEO if round.current_player().id() == player.id() => Ok(Response(
            InternalResponse(std::collections::HashMap::new()),
            DirectResponse::YouCharacterAbility {
                character: Character::CEO,
                perk: "- You can buy up to 3 assets \n- Next turn you become chairman".to_string(),
            },
        )),
        Character::CFO if round.current_player().id() == player.id() => Ok(Response(
            InternalResponse(std::collections::HashMap::new()),
            DirectResponse::YouCharacterAbility {
                character: Character::CFO,
                perk: "You can issue or redeem 3 liabilities".to_string(),
            },
        )),
        Character::CSO if round.current_player().id() == player.id() => Ok(Response(
            InternalResponse(std::collections::HashMap::new()),
            DirectResponse::YouCharacterAbility {
                character: Character::CSO,
                perk: "You can buy up to 2 red or green assets".to_string(),
            },
        )),
        Character::HeadRnD if round.current_player().id() == player.id() => Ok(Response(
            InternalResponse(std::collections::HashMap::new()),
            DirectResponse::YouCharacterAbility {
                character: Character::HeadRnD,
                perk: "You can draw six cards and only have to put 2 back".to_string(),
            },
        )),
        Character::Stakeholder if round.current_player().id() == player.id() => Ok(Response(
            InternalResponse(std::collections::HashMap::new()),
            //TODO send other players divest message
            DirectResponse::YouAreDivesting {
                options: round.get_divest_assets(player_id)?,
                character: Character::Stakeholder,
                perk: "you can force a player to divest from an asset by spending the assets market value -1".to_string(),
            },
        )),
        _ => Err(GameError::InvalidPlayerIndex(0)),
    }
}

pub fn get_bonus_cash(state: &mut GameState, player_id: PlayerId) -> Result<Response, GameError> {
    let round = state.round_mut()?;
    let bonus_cash = round.player_get_bonus_cash_character(player_id)?;

    let internal = round
        .players()
        .iter()
        .filter(|p| p.id() != player_id)
        .map(|p| {
            (
                p.id(),
                vec![UniqueResponse::PlayerGotBonusCash {
                    player_id,
                    cash: bonus_cash,
                }],
            )
        })
        .collect();

    Ok(Response(
        InternalResponse(internal),
        DirectResponse::YouBonusCash { cash: bonus_cash },
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
        .filter(|p| p.id() != player_id)
        .map(|p| {
            (
                p.id(),
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
        .filter(|p| p.id() != player_id)
        .map(|p| {
            (
                p.id(),
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
                .filter(|p| p.id() != player_id)
                .map(|p| {
                    (
                        p.id(),
                        vec![UniqueResponse::BoughtAsset {
                            player_id,
                            card_idx,
                            asset: asset.clone(),
                            market_change: played_card.market.clone(),
                        }],
                    )
                })
                .collect();

            Ok(Response(
                InternalResponse(internal),
                DirectResponse::YouBoughtAsset {
                    asset,
                    card_idx,
                    market_change: played_card.market,
                },
            ))
        }
        Either::Right(liability) => {
            let internal = round
                .players()
                .iter()
                .filter(|p| p.id() != player_id)
                .map(|p| {
                    (
                        p.id(),
                        vec![UniqueResponse::IssuedLiability {
                            player_id,
                            card_idx,
                            liability: liability.clone(),
                        }],
                    )
                })
                .collect();

            Ok(Response(
                InternalResponse(internal),
                DirectResponse::YouIssuedLiability {
                    liability,
                    card_idx,
                },
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
        .filter(|p| p.id() != player_id)
        .map(|p| {
            (
                p.id(),
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
        player_turn: current_player.id(),
        player_turn_cash: current_player.turn_cash(),
        player_character: current_player.character(),
        draws_n_cards: current_player.draws_n_cards(),
        gives_back_n_cards: current_player.gives_back_n_cards(),
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
                GameState::BankerTarget(_) => Err(GameError::NotAvailableInBankerTargetState),
                GameState::SelectingCharacters(selecting) => {
                    let internal = selecting
                        .players()
                        .iter()
                        .map(|p| {
                            (
                                p.id(),
                                vec![UniqueResponse::SelectedCharacter {
                                    currently_picking_id: Some(selecting.currently_selecting_id()),
                                    selectable_characters: selecting
                                        .player_get_selectable_characters(p.id())
                                        .ok(),
                                    closed_character: selecting
                                        .player_get_closed_character(p.id())
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
                        .map(|p| (p.id(), vec![turn_starts(round)]))
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

pub fn fire_character(
    state: &mut GameState,
    player_id: PlayerId,
    character: Character,
) -> Result<Response, GameError> {
    let round = state.round_mut()?;

    match round.player_fire_character(player_id, character) {
        Ok(_c) => {
            let internal = round
                .players()
                .iter()
                .filter(|p| p.id() != player_id)
                .map(|p| {
                    (
                        p.id(),
                        vec![UniqueResponse::FiredCharacter {
                            player_id,
                            character,
                        }],
                    )
                })
                .collect();
            Ok(Response(
                InternalResponse(internal),
                DirectResponse::YouFiredCharacter { character },
            ))
        }
        Err(e) => Err(e),
    }
}

pub fn terminate_credit_character(
    state: &mut GameState,
    player_id: PlayerId,
    character: Character,
) -> Result<Response, GameError> {
    let round = state.round_mut()?;

    match round.player_terminate_credit_character(player_id, character) {
        Ok(_c) => {
            let internal = round
                .players()
                .iter()
                .filter(|p| p.id() != player_id)
                .map(|p| {
                    (
                        p.id(),
                        vec![UniqueResponse::TerminatedCreditCharacter {
                            player_id,
                            character,
                        }],
                    )
                })
                .collect();
            Ok(Response(
                InternalResponse(internal),
                DirectResponse::YouTerminateCreditCharacter { character },
            ))
        }
        Err(e) => Err(e),
    }
}

pub fn select_divest_asset(
    state: &mut GameState,
    player_id: PlayerId,
    asset_id: usize,
) -> Result<Response, GameError> {
    let btround = state.bankertarget_mut()?;
    match btround.player_select_divest_asset(player_id, asset_id) {
        Ok(selected) => Ok(create_selected_cards_response(btround, selected, player_id)),
        Err(e) => Err(e),
    }
}

pub fn unselect_divest_asset(
    state: &mut GameState,
    player_id: PlayerId,
    asset_id: usize,
) -> Result<Response, GameError> {
    let btround = state.bankertarget_mut()?;
    match btround.player_unselect_divest_asset(player_id, asset_id) {
        Ok(selected) => Ok(create_selected_cards_response(btround, selected, player_id)),
        Err(e) => Err(e),
    }
}

pub fn select_issue_liability(
    state: &mut GameState,
    player_id: PlayerId,
    liability_id: usize,
) -> Result<Response, GameError> {
    let btround = state.bankertarget_mut()?;
    match btround.player_select_issue_liability(player_id, liability_id) {
        Ok(selected) => Ok(create_selected_cards_response(btround, selected, player_id)),
        Err(e) => Err(e),
    }
}

pub fn unselect_issue_liability(
    state: &mut GameState,
    player_id: PlayerId,
    liability_id: usize,
) -> Result<Response, GameError> {
    let btround = state.bankertarget_mut()?;
    match btround.player_unselect_issue_liability(player_id, liability_id) {
        Ok(selected) => Ok(create_selected_cards_response(btround, selected, player_id)),
        Err(e) => Err(e),
    }
}

fn create_selected_cards_response(
    btround: &mut BankerTargetRound,
    selected: SelectedAssetsAndLiabilities,
    player_id: PlayerId,
) -> Response {
    let internal = btround
        .players()
        .iter()
        .filter(|p| p.id() != player_id)
        .map(|p| {
            (
                p.id(),
                vec![UniqueResponse::SelectedCardsBankerTarget {
                    assets: selected.sold_assets.clone(),
                    liability_count: selected.issued_liabilities.len(),
                }],
            )
        })
        .collect();
    Response(
        InternalResponse(internal),
        DirectResponse::YouSelectCardBankerTarget {
            assets: selected.sold_assets,
            liabilities: selected.issued_liabilities,
        },
    )
}

pub fn pay_banker(
    state: &mut GameState,
    player_id: PlayerId,
    cash: u8,
) -> Result<Response, GameError> {
    let btround = state.bankertarget_mut()?;
    match btround.player_pay_banker(player_id, cash) {
        Ok(pbp) => {
            let internal = btround
                .players()
                .iter()
                .filter(|p| p.id() != player_id)
                .map(|p| {
                    (
                        p.id(),
                        vec![UniqueResponse::PlayerPaidBanker {
                            banker_id: pbp.banker_id,
                            player_id: pbp.target_id,
                            new_banker_cash: pbp.new_banker_cash,
                            new_target_cash: pbp.new_target_cash,
                            paid_amount: pbp.paid_amount,
                            sold_assets: pbp.selected_cards.sold_assets.clone(),
                            issued_liabilities: pbp.selected_cards.issued_liabilities.clone(),
                        }],
                    )
                })
                .collect();
            *state = GameState::Round(btround.into());
            Ok(Response(
                InternalResponse(internal),
                DirectResponse::YouPaidBanker {
                    banker_id: pbp.banker_id,
                    new_banker_cash: pbp.new_banker_cash,
                    your_new_cash: pbp.new_target_cash,
                    paid_amount: pbp.paid_amount,
                    sold_assets: pbp.selected_cards.sold_assets,
                    issued_liabilities: pbp.selected_cards.issued_liabilities,
                },
            ))
        }
        Err(e) => Err(e),
    }
}

pub fn swap_with_deck(
    state: &mut GameState,
    player_id: PlayerId,
    card_idxs: Vec<usize>,
) -> Result<Response, GameError> {
    let round = state.round_mut()?;

    match round.player_swap_with_deck(player_id, card_idxs) {
        Ok(AssetLiabilityCount {
            asset_count,
            liability_count,
        }) => {
            let internal = round
                .players()
                .iter()
                .filter(|p| p.id() != player_id)
                .map(|p| {
                    (
                        p.id(),
                        vec![UniqueResponse::SwappedWithDeck {
                            asset_count,
                            liability_count,
                        }],
                    )
                })
                .collect();
            Ok(Response(
                InternalResponse(internal),
                DirectResponse::YouSwapDeck {
                    cards_to_draw: asset_count + liability_count,
                },
            ))
        }
        Err(e) => Err(e),
    }
}

pub fn swap_with_player(
    state: &mut GameState,
    player_id: PlayerId,
    target_player_id: PlayerId,
) -> Result<Response, GameError> {
    let round = state.round_mut()?;

    let hands = round.player_swap_with_player(player_id, target_player_id)?;

    let internal = round
        .players()
        .iter()
        .filter(|p| ![player_id, target_player_id].contains(&p.id()))
        .map(|p| {
            (
                p.id(),
                vec![UniqueResponse::SwappedWithPlayer {
                    regulator_id: player_id,
                    target_id: target_player_id,
                }],
            )
        })
        .chain(std::iter::once((
            target_player_id,
            vec![UniqueResponse::RegulatorSwappedYourCards {
                new_cards: hands.target_new_hand,
            }],
        )))
        .collect();

    Ok(Response(
        InternalResponse(internal),
        DirectResponse::YouSwapPlayer {
            new_cards: hands.regulator_new_hand,
            target_player_id,
        },
    ))
}

pub fn divest_asset(
    state: &mut GameState,
    stakeholder_id: PlayerId,
    target_id: PlayerId,
    asset_idx: usize,
) -> Result<Response, GameError> {
    let round = state.round_mut()?;

    match round.player_divest_asset(stakeholder_id, target_id, asset_idx) {
        Ok(gold_cost) => {
            let internal = round
                .players()
                .iter()
                .filter(|p| p.id() != stakeholder_id)
                .map(|p| {
                    (
                        p.id(),
                        vec![UniqueResponse::AssetDivested {
                            player_id: stakeholder_id,
                            target_id,
                            asset_idx,
                            paid_gold: gold_cost,
                        }],
                    )
                })
                .collect();
            Ok(Response(
                InternalResponse(internal),
                DirectResponse::YouDivestedAnAsset {
                    target_id,
                    asset_idx,
                    gold_cost,
                },
            ))
        }
        Err(e) => Err(e),
    }
}

pub fn end_turn(state: &mut GameState, player_id: PlayerId) -> Result<Response, GameError> {
    state.end_player_turn(player_id)?;

    match state {
        GameState::Lobby(_) => Err(GameError::NotAvailableInLobbyState),
        GameState::BankerTarget(_) => Err(GameError::NotAvailableInBankerTargetState),
        GameState::SelectingCharacters(selecting) => {
            let internal = selecting
                .players()
                .iter()
                .map(|p| {
                    (
                        p.id(),
                        vec![UniqueResponse::SelectingCharacters {
                            chairman_id: selecting.chairman_id(),
                            selectable_characters: selecting
                                .player_get_selectable_characters(p.id())
                                .ok(),
                            open_characters: selecting.open_characters().to_vec(),
                            closed_character: selecting.player_get_closed_character(p.id()).ok(),
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
            let mut internal: HashMap<PlayerId, Vec<UniqueResponse>> = round
                .players()
                .iter()
                .map(|p| (p.id(), vec![turn_starts(round)]))
                .collect();

            if round.banker_target() == Some(round.current_player().character()) {
                *state = GameState::BankerTarget(round.into());
                for value in internal.values_mut() {
                    value.push(UniqueResponse::PlayerTargetedByBanker {
                        player_turn: state.bankertarget()?.current_player().id(),
                        cash_to_be_paid: state.bankertarget()?.gold_to_be_paid(),
                        is_possible_to_pay_banker: state.bankertarget()?.can_pay_banker(),
                    });
                }
            }

            Ok(Response(
                InternalResponse(internal),
                DirectResponse::YouEndedTurn,
            ))
        }
        GameState::Results(results) => {
            let scores = results.player_scores();

            let internal = results
                .players()
                .iter()
                .map(|p| {
                    (
                        p.id(),
                        vec![UniqueResponse::GameEnded {
                            scores: scores.clone(),
                        }],
                    )
                })
                .collect();

            Ok(Response(
                InternalResponse(internal),
                DirectResponse::YouEndedTurn,
            ))
        }
    }
}

pub fn minus_into_plus(
    state: &mut GameState,
    player_id: PlayerId,
    color: Color,
) -> Result<Response, GameError> {
    let results = state.results_mut()?;

    match results.toggle_minus_into_plus(player_id, color) {
        Ok(new_market) => {
            let player = results.player(player_id)?;
            let new_score = player.score();

            let internal = results
                .players()
                .iter()
                .filter(|p| p.id() != player_id)
                .map(|p| {
                    let new_market = new_market.clone();
                    (
                        p.id(),
                        vec![UniqueResponse::MinusedIntoPlus {
                            player_id,
                            new_market,
                            new_score,
                        }],
                    )
                })
                .collect();

            Ok(Response(
                InternalResponse(internal),
                DirectResponse::YouMinusedIntoPlus {
                    color,
                    new_market,
                    new_score,
                },
            ))
        }
        Err(e) => Err(e),
    }
}

pub fn silver_into_gold(
    state: &mut GameState,
    player_id: PlayerId,
    asset_idx: usize,
) -> Result<Response, GameError> {
    let results = state.results_mut()?;

    match results.toggle_silver_into_gold(player_id, asset_idx) {
        Ok(ToggleSilverIntoGold {
            old_asset_data,
            new_asset_data,
        }) => {
            let player = results.player(player_id)?;
            let new_score = player.score();

            let internal = results
                .players()
                .iter()
                .filter(|p| p.id() != player_id)
                .map(|p| {
                    (
                        p.id(),
                        vec![UniqueResponse::SilveredIntoGold {
                            player_id,
                            old_asset_data,
                            new_asset_data,
                            new_score,
                        }],
                    )
                })
                .collect();

            Ok(Response(
                InternalResponse(internal),
                DirectResponse::YouSilveredIntoGold {
                    old_asset_data,
                    new_asset_data,
                    new_score,
                },
            ))
        }
        Err(e) => Err(e),
    }
}

pub fn change_asset_color(
    state: &mut GameState,
    player_id: PlayerId,
    asset_idx: usize,
    color: Color,
) -> Result<Response, GameError> {
    let results = state.results_mut()?;

    match results.toggle_change_asset_color(player_id, asset_idx, color) {
        Ok(ToggleChangeAssetColor {
            old_asset_data,
            new_asset_data,
        }) => {
            let player = results.player(player_id)?;
            let new_score = player.score();

            let internal = results
                .players()
                .iter()
                .filter(|p| p.id() != player_id)
                .map(|p| {
                    (
                        p.id(),
                        vec![UniqueResponse::ChangedAssetColor {
                            player_id,
                            old_asset_data,
                            new_asset_data,
                            new_score,
                        }],
                    )
                })
                .collect();

            Ok(Response(
                InternalResponse(internal),
                DirectResponse::YouChangedAssetColor {
                    old_asset_data,
                    new_asset_data,
                    new_score,
                },
            ))
        }
        Err(e) => Err(e),
    }
}

pub fn confirm_asset_ability(
    state: &mut GameState,
    player_id: PlayerId,
    asset_idx: usize,
) -> Result<Response, GameError> {
    let results = state.results_mut()?;
    results.confirm_asset_ability(player_id, asset_idx)?;

    let internal = results
        .players()
        .iter()
        .filter(|p| p.id() != player_id)
        .map(|p| {
            (
                p.id(),
                vec![UniqueResponse::ConfirmedAssetAbility {
                    player_id,
                    asset_idx,
                }],
            )
        })
        .collect();

    Ok(Response(
        InternalResponse(internal),
        DirectResponse::YouConfirmedAssetAbility { asset_idx },
    ))
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
