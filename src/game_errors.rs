use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The main error struct of the game logic
#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum GameError {
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
    #[error("Not player's turn")]
    NotPlayersTurn,
    #[error("Player should still give back at least one card")]
    PlayerShouldGiveBackCard,
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
}
