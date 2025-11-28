use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::player::Character;

/// The main error struct of the game logic
#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum GameError {
    #[error(transparent)]
    Lobby(#[from] LobbyError),
    #[error(transparent)]
    SelectingCharacters(#[from] SelectingCharactersError),
    #[error(transparent)]
    PlayCard(#[from] PlayCardError),
    #[error(transparent)]
    RedeemLiability(#[from] RedeemLiabilityError),
    #[error(transparent)]
    GiveBackCard(#[from] GiveBackCardError),
    #[error(transparent)]
    DrawCard(#[from] DrawCardError),
    #[error(transparent)]
    FireCharacter(#[from] FireCharacterError),
    #[error(transparent)]
    Swap(#[from] SwapError),
    #[error(transparent)]
    DivestAsset(#[from] DivestAssetError),
    #[error(transparent)]
    GetDivestAssets(#[from] GetDivestAssetsError),
    #[error("Asset index {0} is invalid")]
    InvalidAssetIndex(u8),
    #[error("Player count should be between 4 and 7, {0} is invalid")]
    InvalidPlayerCount(u8),
    #[error("Player index {0} is invalid")]
    InvalidPlayerIndex(u8),
    #[error("Player name {0} is invalid")]
    InvalidPlayerName(String),
    #[error("Player hasn't selected a character yet")]
    PlayerMissingCharacter,
    #[error("Not player's turn")]
    NotPlayersTurn,
    #[error("Player should still give back at least one card")]
    PlayerShouldGiveBackCard,
    #[error("Action only allowed in Lobby state")]
    NotLobbyState,
    #[error("Action only allowed in Selecting Characters state")]
    NotSelectingCharactersState,
    #[error("Action only allowed in Round state")]
    NotRoundState,
    #[error("Action only allowed in Results state")]
    NotResultsState,
    #[error("Action unavailable in lobby state")]
    NotAvailableInLobbyState,
    #[error("Action unavailable in results state")]
    NotAvailableInResultsState,
}

#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum LobbyError {
    #[error("Username {0} already taken")]
    UsernameAlreadyTaken(String),
    #[error("Username {0} is invalid")]
    InvalidUsername(String),
    #[error("Username {0} is not in lobby")]
    UsernameNotInLobby(String),
}

#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum PlayCardError {
    #[error("Card index {0} is invalid")]
    InvalidCardIndex(u8),
    #[error("Already played the maximum allowed number of assets")]
    ExceedsMaximumAssets,
    #[error("Already played the maximum allowed number of liabilities")]
    ExceedsMaximumLiabilities,
    #[error("{cash} cash is not enough to afford asset worth {cost}")]
    CannotAffordAsset { cash: u8, cost: u8 },
}

#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum RedeemLiabilityError {
    #[error("Character type '{0:?}' cannot redeem liability")]
    NotAllowedToRedeemLiability(Character),
    #[error("Already played the maximum allowed number of liabilities")]
    ExceedsMaximumLiabilities,
    #[error("Invalid liability index {0}")]
    InvalidLiabilityIndex(u8),
    #[error("{cash} gold is not enough to redeem liability with value {cost}")]
    NotEnoughCash { cash: u8, cost: u8 },
}

#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum GiveBackCardError {
    #[error("Card index {0} is invalid")]
    InvalidCardIndex(u8),
    #[error("Player does not have to give back card")]
    Unnecessary,
}

#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum FireCharacterError {
    #[error("Character is not allowed to be fired")]
    InvalidCharacter,
    #[error("Only the shareholder can fire a character")]
    InvalidPlayerCharacter,
    #[error("Player has already fired a character this turn")]
    AlreadyFiredThisTurn,
}

#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum SwapError {
    #[error("Player has already swaped their hand this turn")]
    AlreadySwapedThisTurn,
    #[error("Only the regulator can swap their hand")]
    InvalidPlayerCharacter,
    #[error("invalid card indexes")]
    InvalidCardIdxs,
    #[error("cant swap with this player")]
    InvalidTargetPlayer,
}

#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum DivestAssetError {
    #[error("Can't divest assets from this character")]
    InvalidCharacter,
    #[error("Only the stakeholder can divest assets")]
    InvalidPlayerCharacter,
    #[error("Player has already divested an asset this turn")]
    AlreadyDivestedThisTurn,
    #[error("can't divest red or green assets")]
    CantDivestAssetType,
    #[error("You don't have enough cach to divest this asset")]
    NotEnoughCash,
    #[error("invalid card idex")]
    InvalidCardIdx,
}

#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum GetDivestAssetsError {
    #[error("Only the stakeholder force a player to divest")]
    InvalidPlayerCharacter,
}

#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum DrawCardError {
    #[error("Already drew {0} cards, which is the maximum for this character")]
    MaximumCardsDrawn(u8),
}

#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum SelectingCharactersError {
    #[error("Game is not in a state where characters are being picked")]
    NotPickingCharacters,
    #[error("Already selected character {0:?}")]
    AlreadySelectedCharacter(Character),
    #[error("Character is not availalble to pick")]
    UnavailableCharacter,
    #[error("Player is not chairman")]
    NotChairman,
}

#[derive(Debug, Error)]
pub enum DataParseError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}
