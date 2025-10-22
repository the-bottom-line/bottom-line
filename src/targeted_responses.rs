use crate::{game::*, utility::serde_asset_liability};
use either::Either;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct StartGame {
    cash: u8,
    #[serde(with = "serde_asset_liability::vec")]
    hand: Vec<Either<Asset, Liability>>,
    pickable_characters: Option<PickableCharacters>,
    player_info: Vec<PlayerInfo>,
    turn_order: Vec<PlayerId>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlayersInLobby {
    players: Vec<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct SelectedCharacter;
#[derive(Debug, Clone, Serialize)]
pub struct DrawCard;
#[derive(Debug, Clone, Serialize)]
pub struct BuyAsset;
#[derive(Debug, Clone, Serialize)]
pub struct IssueLiability;
