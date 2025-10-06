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
#[serde(rename_all = "snake_case")]
pub enum SendJson {
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
}

pub fn handle_request(msg: ReceiveJson, room_state: Arc<RoomState>, player_name: &str) -> SendJson {
pub fn handle_request(msg: ReceiveJson, room_state: Arc<RoomState>, player_name: &str) -> SendJson {
    //todo parse json request and

    let mut game = room_state.game.lock().unwrap();
    let mut response: SendJson = SendJson::ActionNotAllowed;
    match &mut *game {
        crate::server::Game::GameStarted { state } => {
            let playerid :usize = state.player_by_name(player_name).unwrap().id.into();
            match msg.action {
                ReceiveJsonAction::StartGame => todo!(),
                ReceiveJsonAction::DrawCard { card_type } => {response = draw_card(state, card_type, playerid);},
                ReceiveJsonAction::PutBackCard { card_idx } => todo!(),
                ReceiveJsonAction::BuyAsset { asset_idx } => todo!(),
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

fn draw_card(state: &mut GameState, t: CardType, player_idx :usize) -> SendJson {
    let card: Option<Either<Asset, Liability>> = state.player_draw_card(player_idx, t);
    if (card.){

    }

    return "".into();
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

        let send = SendJson::PutBackCard {
            remove_idx: None
        };

        let sjson = serde_json::to_string(&send).unwrap();

        println!("send json: {sjson}");
    }
}
