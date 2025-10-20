use either::Either;
use serde::Serialize;
use crate::{game::*, utility::serde_asset_liability};

#[derive(Debug, Clone, Serialize)]
pub struct StartGame {
    cash: u8,
    #[serde(with = "serde_asset_liability::vec")]
    hand: Vec<Either<Asset, Liability>>,
    pickable_characters: Option<PickableCharacters>,
    player_info: Vec<PlayerInfo>,
    turn_order: Vec<PlayerId>,
}

pub struct SelectedCharacter;
pub struct DrawCard;
pub struct BuyAsset;
pub struct IssueLiability;