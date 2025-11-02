use crate::{errors::GameError, game::*, player::*, responses::*, rooms::RoomState};
use either::Either;

pub mod internal {
    use crate::{game::*, player::*, responses::*};

    pub fn players_in_lobby(lobby: &Lobby, changed_player: String) -> Vec<UniqueResponse> {
        let usernames = lobby.players().clone();
        vec![UniqueResponse::PlayersInLobby {
            changed_player,
            usernames,
        }]
    }

    pub fn game_started(selecting: &SelectingCharacters, username: &str) -> Vec<UniqueResponse> {
        let player = selecting.player_by_name(username).unwrap();

        vec![
            UniqueResponse::StartGame {
                id: player.id,
                hand: player.hand.clone(),
                cash: player.cash,
                open_characters: selecting.open_characters().to_vec(),
                player_info: selecting.player_info(player.id),
            },
            UniqueResponse::SelectingCharacters {
                chairman_id: selecting.chairman,
                pickable_characters: selecting.player_get_selectable_characters(player.id).ok(),
                turn_order: selecting.turn_order(),
            },
        ]
    }

    pub fn selected_character(
        selecting: &SelectingCharacters,
        username: &str,
    ) -> Vec<UniqueResponse> {
        let player = selecting.player_by_name(username).unwrap();
        let currently_picking_id = selecting.currently_selecting_id();

        todo!()

        //     let pickable_characters = state.player_get_selectable_characters(player.id).ok();

        //     let selected = UniqueResponse::SelectedCharacter {
        //         currently_picking_id,
        //         pickable_characters,
        //     };

        //     if let GameState::Round(round) = state {
        //         // started round
        //         Some(vec![
        //             selected,
        //             UniqueResponse::TurnStarts {
        //                 player_turn: round.current_player().id,
        //                 player_turn_cash: 1,
        //                 player_character: round.current_player().character.unwrap(),
        //                 draws_n_cards: 3,
        //                 skipped_characters: vec![],
        //             },
        //         ])
        //     } else {
        //         Some(vec![selected])
        //     }
    }

    pub fn drawn_card(player_id: PlayerId, card_type: CardType) -> Vec<UniqueResponse> {
        vec![UniqueResponse::DrewCard {
            player_id,
            card_type,
        }]
    }

    pub fn put_back_card(player_id: PlayerId, card_type: CardType) -> Vec<UniqueResponse> {
        vec![UniqueResponse::PutBackCard {
            player_id,
            card_type,
        }]
    }

    pub fn bought_asset(player_id: PlayerId, asset: Asset) -> Vec<UniqueResponse> {
        vec![UniqueResponse::BoughtAsset { player_id, asset }]
    }

    pub fn issued_liability(player_id: PlayerId, liability: Liability) -> Vec<UniqueResponse> {
        vec![UniqueResponse::IssuedLiability {
            player_id,
            liability,
        }]
    }

    pub fn turn_ended(player_id: PlayerId) -> Vec<UniqueResponse> {
        todo!()

        // match state {
        //     GameState::Round(round) => {
        //         Some(vec![
        //             UniqueResponse::TurnEnded { player_id },
        //             UniqueResponse::TurnStarts {
        //                 player_turn: round.current_player().id,
        //                 player_turn_cash: 1,
        //                 player_character: round.current_player().character.unwrap(),
        //                 draws_n_cards: 3,
        //                 // TODO: implement concept of skipped characters
        //                 skipped_characters: vec![],
        //             },
        //         ])
        //     }
        //     GameState::SelectingCharacters(selecting) => {
        //         let player = state.player_by_name(player_name).unwrap();
        //         let pickable_characters =
        //             state.player_get_selectable_characters(player.id).ok();
        //         Some(vec![UniqueResponse::SelectingCharacters {
        //             chairman_id: selecting.chairman,
        //             pickable_characters,
        //             // player_info: state.player_info(player.id.into()),
        //             turn_order: selecting.turn_order(),
        //         }])
        //     }
        //     GameState::Results(_) => todo!(),
        //     GameState::Lobby(_) => unreachable!(),
        // }
    }
}

pub mod external {
    use either::Either;

    use crate::{errors::GameError, game::*, player::*, responses::*};

    pub fn draw_card(
        state: &mut GameState,
        card_type: CardType,
        player_id: PlayerId,
    ) -> Result<Response, GameError> {
        let card = state.player_draw_card(player_id, card_type)?.cloned();
        let player = state.player(player_id)?;

        Ok(Response(
            InternalResponse::DrawnCard {
                player_id,
                card_type,
            },
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
        let card_type = state.player_give_back_card(player_id, card_idx)?;
        let player = state.player(player_id)?;

        Ok(Response(
            InternalResponse::PutBackCard {
                player_id,
                card_type,
            },
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
        let played_card = state.player_play_card(player_id, card_idx)?;

        match played_card.used_card {
            Either::Left(asset) => Ok(Response(
                InternalResponse::BoughtAsset {
                    player_id,
                    asset: asset.clone(),
                },
                DirectResponse::YouBoughtAsset { asset },
            )),
            Either::Right(liability) => Ok(Response(
                InternalResponse::IssuedLiability {
                    player_id,
                    liability: liability.clone(),
                },
                DirectResponse::YouIssuedLiability { liability },
            )),
        }
    }

    pub fn select_character(
        state: &mut GameState,
        player_id: PlayerId,
        character: Character,
    ) -> Result<Response, GameError> {
        match state.player_select_character(player_id, character) {
            Ok(_) => Ok(Response(
                InternalResponse::SelectedCharacter,
                DirectResponse::YouSelectedCharacter { character },
            )),
            Err(e) => Err(e),
        }
    }

    pub fn end_turn(state: &mut GameState, player_id: PlayerId) -> Result<Response, GameError> {
        match state.end_player_turn(player_id)? {
            TurnEnded {
                next_player: Some(player_id),
            } => Ok(Response(
                InternalResponse::TurnEnded { player_id },
                DirectResponse::YouEndedTurn,
            )),
            _ => {
                // if next_player is none // TODO: Fix for end of round
                let player_id = state.selecting_characters().unwrap().chairman;
                Ok(Response(
                    InternalResponse::TurnEnded { player_id },
                    DirectResponse::YouEndedTurn,
                ))
            }
        }
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
