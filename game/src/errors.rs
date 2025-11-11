use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::player::Character;

/// The main error struct of the game logic
#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum GameError {
    #[error(transparent)]
    LobbyError(#[from] LobbyError),
    #[error(transparent)]
    PlayCard(#[from] PlayCardError),
    #[error(transparent)]
    GiveBackCard(#[from] GiveBackCardError),
    #[error(transparent)]
    DrawCard(#[from] DrawCardError),
    #[error(transparent)]
    SelectableCharacters(#[from] SelectableCharactersError),
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
    #[error("Username is invalid")]
    InvalidUsername,
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
pub enum DrawCardError {
    #[error("Already drew {0} cards, which is the maximum for this character")]
    MaximumCardsDrawn(u8),
}

#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum SelectableCharactersError {
    #[error("Game is not in a state where characters are being picked")]
    NotPickingCharacters,
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
