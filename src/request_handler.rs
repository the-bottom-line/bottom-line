use std::sync::Arc;

use crate::{
    cards::GameData,
    game::*,
    server::{AppState, Game, RoomState},
};
use either::Either;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum ReceiveJson {
    StartGame,
    DrawCard { card_type: CardType },
    PutBackCard { card_idx: usize },
    BuyAsset { asset_idx: usize },
    IssueLiability { liability_idx: usize },
    GetSelectableCharacters,
    SelectCharacter { character: Character },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SendJson(pub PublicSendJson, pub PrivateSendJson);

impl SendJson {
    pub fn new(public: PublicSendJson, private: PrivateSendJson) -> Self {
        Self(public, private)
    }
}

impl From<PrivateSendJson> for SendJson {
    fn from(private: PrivateSendJson) -> Self {
        Self::new(PublicSendJson::ActionPerformed, private)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum PrivateSendJson {
    ActionNotAllowed,
    GameStartedOk,
    StartGame {
        cash: u8,
        hand: Vec<Either<Asset, Liability>>,
    },
    DrawnCard {
        card: Either<Asset, Liability>,
    },
    PutBackCard {
        remove_idx: Option<usize>,
    },
    BuyAssetOk,
    IssuedLiabilityOk,
    SelectableCharacters {
        characters: Vec<Character>,
    },
    SelectCharacterOk,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum PublicSendJson {
    ActionPerformed, // all-round placeholder
    PlayerJoined {
        username: String,
    },
    PlayerLeft {
        username: String,
    },
    GameStarted,
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
    SelectedCharacter {
        player_id: PlayerId,
    },
}

pub fn handle_public_request(
    msg: PublicSendJson,
    room_state: Arc<RoomState>,
    player_name: &str,
) -> Option<PrivateSendJson> {
    match &*room_state.game.lock().unwrap() {
        Game::GameStarted { state } => {
            let player = state.player_by_name(&player_name).unwrap();
            match msg {
                PublicSendJson::GameStarted => {
                    let hand = player.hand.clone();
                    let cash = player.cash;
                    Some(PrivateSendJson::StartGame { hand, cash })
                }
                _ => None,
            }
        }
        Game::InLobby { user_set: _ } => None,
    }
}

pub fn handle_request(msg: ReceiveJson, room_state: Arc<RoomState>, player_name: &str) -> SendJson {
    //todo parse json request and

    let mut game = room_state.game.lock().unwrap();
    match &mut *game {
        crate::server::Game::GameStarted { state } => {
            let playerid: usize = state.player_by_name(player_name).unwrap().id.into();
            match msg.action {
                ReceiveJson::StartGame => PrivateSendJson::ActionNotAllowed.into(),
                ReceiveJson::DrawCard { card_type } => draw_card(state, card_type, playerid),
                ReceiveJson::PutBackCard { card_idx } => put_back_card(state, card_idx, playerid),
                ReceiveJson::BuyAsset { asset_idx } => play_card(state, asset_idx, playerid),
                ReceiveJson::IssueLiability { liability_idx } => play_card(state, liability_idx, playerid),
                ReceiveJson::GetSelectableCharacters => get_selectable_characters(state, playerid),
                ReceiveJson::SelectCharacter { character } => select_character(state, character, playerid),
            }
        }
        crate::server::Game::InLobby { user_set } => match msg {
            ReceiveJson::StartGame => {
                let names = user_set.iter().cloned().collect::<Vec<_>>();
                let data = GameData::new("assets/cards/boardgame.json").expect("this should exist");
                let state = GameState::new(&names, data);
                *game = Game::GameStarted { state };
                tracing::debug!("{msg:?}");
                SendJson(PublicSendJson::GameStarted, PrivateSendJson::GameStartedOk)
            }
            _ => PrivateSendJson::ActionNotAllowed.into(),
        },
    }
}

fn draw_card(state: &mut GameState, t: CardType, player_idx: usize) -> SendJson {
    if let Some(card) = state.player_draw_card(player_idx, t) {
        return SendJson::new(
            PublicSendJson::DrawnCard {
                player_id: player_idx.into(),
                card_type: t
            },
            PrivateSendJson::DrawnCard {
            card: card.cloned(),
            }
        );
    } else {
        return PrivateSendJson::ActionNotAllowed.into();
    }
}

fn put_back_card(state: &mut GameState, card_idx: usize, player_idx: usize) -> SendJson{
    let t= state.player_give_back_card(player_idx, card_idx);
    if let Some(idx) = t.0 {
        return SendJson::new(
            PublicSendJson::PutBackCard {
                player_id: player_idx.into(),
                card_type: t.1
            },
            PrivateSendJson::PutBackCard {
                remove_idx: Some(idx),
            }
        );
    } else {
        return PrivateSendJson::ActionNotAllowed.into();
    }
}

fn play_card(state: &mut GameState, card_idx: usize, player_idx: usize) -> SendJson{
    if let Some(played_card) = state.player_play_card(player_idx, card_idx) {
        match played_card.used_card {
                    Either::Left(asset) => {
                        return SendJson::new(
            PublicSendJson::BoughtAsset {
                player_id: player_idx.into(), asset: asset } ,
            PrivateSendJson::BuyAssetOk
        );
                    }
                    Either::Right(liability) => {
                        return SendJson::new(
            PublicSendJson::IssuedLiability {
                player_id: player_idx.into(),
                 liability: liability
                },
            PrivateSendJson::IssuedLiabilityOk
        );
                }
            }
        } else {
        return PrivateSendJson::ActionNotAllowed.into();
    }
}

fn select_character(state: &mut GameState, character: Character, player_idx: usize) -> SendJson{
    let cs= state.next_player_select_character(player_idx, character);
    if let Some(c) = cs {
        return SendJson::new(
            PublicSendJson::SelectedCharacter {
                player_id: player_idx.into()
            },
            PrivateSendJson::SelectCharacterOk
        );
    } else {
        return PrivateSendJson::ActionNotAllowed.into();
    }
}

fn get_selectable_characters(state: &mut GameState, player_idx: usize) -> SendJson{
    let cs= state.get_selectable_characters(player_idx);
    if let Some(c) = cs {
        return SendJson::new(
         PublicSendJson::ActionPerformed,
        PrivateSendJson::SelectableCharacters { characters: c }
        );
    } else {
        return PrivateSendJson::ActionNotAllowed.into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt() {
        let action = ReceiveJson::StartGame;

        let action2 = ReceiveJson::DrawCard {
            card_type: CardType::Asset,
        };

        let json = serde_json::to_string(&action).unwrap();
        let json2 = serde_json::to_string(&action2).unwrap();

        println!("json: {json}");
        println!("json2: {json2}");

        let send = PrivateSendJson::PutBackCard { remove_idx: None };

        let sjson = serde_json::to_string(&send).unwrap();

        println!("send json: {sjson}");
    }
}
