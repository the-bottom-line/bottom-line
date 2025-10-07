use std::sync::Arc;

use crate::{
    cards::GameData,
    game::*,
    server::{AppState, Game, RoomState},
};
use either::Either;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiveJson {
    action: ReceiveJsonAction,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveJsonAction {
    StartGame,
    DrawCard { card_type: CardType },
    PutBackCard { card_idx: usize },
    BuyAsset { asset_idx: usize },
    IssueLiability { liability_idx: usize },
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
#[serde(rename_all = "snake_case")]
pub enum PrivateSendJson {
    ActionNotAllowed,
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicSendJson {
    ActionPerformed, // all-round placeholder
    GameStarted,
    Msg(String),
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

pub fn handle_request(msg: ReceiveJson, room_state: Arc<RoomState>, player_name: &str) -> SendJson {
    //todo parse json request and

    let mut game = room_state.game.lock().unwrap();
    let mut response = SendJson::from(PrivateSendJson::ActionNotAllowed);
    match &mut *game {
        crate::server::Game::GameStarted { state } => {
            let playerid: usize = state.player_by_name(player_name).unwrap().id.into();
            match msg.action {
                ReceiveJsonAction::StartGame => todo!(),
                ReceiveJsonAction::DrawCard { card_type } => {
                    response = draw_card(state, card_type, playerid);
                },
                ReceiveJsonAction::PutBackCard { card_idx } => {
                    response = put_back_card(state, card_idx, playerid)
                },
                ReceiveJsonAction::BuyAsset { asset_idx } =>  response = buy_asset(state, asset_idx, playerid),
                ReceiveJsonAction::IssueLiability { liability_idx } => todo!(),
                ReceiveJsonAction::SelectCharacter { character } => todo!(),
            }
        }
        crate::server::Game::InLobby { user_set } => match msg.action {
            ReceiveJsonAction::StartGame => {
                let names = user_set.iter().cloned().collect::<Vec<_>>();
                let data = GameData::new("assets/cards/boardgame.json").expect("this should exist");
                let state = GameState::new(&names, data);
                *game = Game::GameStarted { state };
            }
            _ => panic!(),
        },
    }

    return response;
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

fn buy_asset(state: &mut GameState, asset_idx: usize, player_idx: usize) -> SendJson{
    if let Some(played_card) = state.player_play_card(player_idx, asset_idx) {
        return SendJson::new(
            PublicSendJson::BoughtAsset { 
                player_id: player_idx.into(), asset: Some((played_card.card)) } ,
            PrivateSendJson::BuyAssetOk
        );
    } else {
        return PrivateSendJson::ActionNotAllowed.into();
    }
}

fn issue_liability(state: &mut GameState, liability_idx: usize, player_idx: usize) -> SendJson{
    if let Some(played_card) = state.player_play_card(player_idx, liability_idx) {
        return SendJson::new(
            PublicSendJson::BoughtAsset { 
                player_id: player_idx.into(), asset: Some((played_card.card)) },
            PrivateSendJson::IssuedLiabilityOk
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
        let action = ReceiveJson {
            action: ReceiveJsonAction::StartGame, // action: ReceiveJsonAction::DrawCard { card_type: CardType::Asset }
        };

        let action2 = ReceiveJson {
            action: ReceiveJsonAction::DrawCard {
                card_type: CardType::Asset,
            },
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
