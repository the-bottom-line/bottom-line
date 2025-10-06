use std::sync::Arc;

use crate::{
    cards::GameData,
    game::*,
    server::{AppState, Game, RoomState},
};
use axum::extract::ws::Utf8Bytes;
use either::Either;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiveJson {
    user_id: usize,
    action: ReceiveJsonAction,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReceiveJsonAction {
    StartGame,
    DrawCard { card_type: CardType },
    PutBackCard { card: Either<Asset, Liability> },
    BuyAsset { asset: Asset },
    IssueLiability { liability: Liability },
    SelectCharacter { character: Character },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SendJson {}

pub struct SendJsonAction {}

pub fn handle_request(msg: ReceiveJson, room_state: Arc<RoomState>) -> Utf8Bytes {
    //todo parse json request and

    let mut game = room_state.game.lock().unwrap();

    match &mut *game {
        crate::server::Game::GameStarted { state } => {
            match msg.action {
                ReceiveJsonAction::StartGame => todo!(),
                ReceiveJsonAction::DrawCard { card_type } => todo!(),
                ReceiveJsonAction::PutBackCard { card } => todo!(),
                ReceiveJsonAction::BuyAsset { asset } => todo!(),
                ReceiveJsonAction::IssueLiability { liability } => todo!(),
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

    return "".into();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt() {
        let action = ReceiveJson {
            user_id: 10,
            action: ReceiveJsonAction::StartGame, // action: ReceiveJsonAction::DrawCard { card_type: CardType::Asset }
        };

        let action2 = ReceiveJson {
            user_id: 20,
            action: ReceiveJsonAction::DrawCard {
                card_type: CardType::Asset,
            },
        };

        let json = serde_json::to_string(&action).unwrap();
        let json2 = serde_json::to_string(&action2).unwrap();

        println!("json: {json}");
        println!("json2: {json2}");
    }
}
