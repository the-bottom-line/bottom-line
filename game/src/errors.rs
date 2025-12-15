//! This file contains all errors that are used by the game, leveraging `thiserror`. This includes
//! a general [`GameError`] enum as well as smaller, action-specific
//! errors used throughout the game logic.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::player::{AssetPowerup, Character};

/// The main error enum used by the game logic.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum GameError {
    /// Errors related to the lobby phase
    #[error(transparent)]
    Lobby(#[from] LobbyError),

    /// Errors related to the selecting characters phase
    #[error(transparent)]
    SelectingCharacters(#[from] SelectingCharactersError),

    /// Errors related to the action of playing a card
    #[error(transparent)]
    PlayCard(#[from] PlayCardError),

    /// Errors related to the action of redeeming a liability
    #[error(transparent)]
    RedeemLiability(#[from] RedeemLiabilityError),

    /// Errors related to the action of giving back a card
    #[error(transparent)]
    GiveBackCard(#[from] GiveBackCardError),

    /// Errors related to the action of drawing a card
    #[error(transparent)]
    DrawCard(#[from] DrawCardError),

    /// Errors related to the action of firing a character
    #[error(transparent)]
    FireCharacter(#[from] FireCharacterError),
    
    /// Errors related to the action of terminating a characters credit line
    #[error(transparent)]
    TerminateCreditCharacter(#[from] TerminateCreditCharacterError),

    /// Errors related to the action of swapping cards with another player or the deck
    #[error(transparent)]
    Swap(#[from] SwapError),

    /// Errors related to the action of forcing another player to divest an asset
    #[error(transparent)]
    DivestAsset(#[from] DivestAssetError),

    /// Errors related to the asset abilities
    #[error(transparent)]
    CardAbility(#[from] AssetAbilityError),

    /// Error indicating when a certain index is out of bounds
    #[error("Asset index {0} is invalid")]
    InvalidAssetIndex(u8),

    /// Error indicating when a lobby does not contain between 4 and 7 players
    #[error("Player count should be between 4 and 7, {0} is invalid")]
    InvalidPlayerCount(u8),

    /// Error indicating a certain player index is out of bounds
    #[error("Player index {0} is invalid")]
    InvalidPlayerIndex(u8),

    /// Error indicating that a certain player name is not found among the players
    #[error("Player name {0} is invalid")]
    InvalidPlayerName(String),

    /// Error indicating that a certain player hasn't selected a character yet.
    #[error("Player hasn't selected a character yet")]
    PlayerMissingCharacter,

    /// Error indicating it's not this player's turn
    #[error("Not player's turn")]
    NotPlayersTurn,

    /// Error indicating that this player cannot end their turn because they should still give back
    /// a certain number of cards
    #[error("Player should still give back at least one card")]
    PlayerShouldGiveBackCard,

    /// Error indicating that this action is only allowed in the lobby state
    #[error("Action only allowed in Lobby state")]
    NotLobbyState,

    /// Error indicating that this action is only allowed in the selecting characters state
    #[error("Action only allowed in Selecting Characters state")]
    NotSelectingCharactersState,

    /// Error indicating that this action is only allowed in the round state
    #[error("Action only allowed in Round state")]
    NotRoundState,

    /// Error indicating that this action is only allowed in the results state
    #[error("Action only allowed in Results state")]
    NotResultsState,

    /// Error indicating that this action is not allowed in the lobby state
    #[error("Action unavailable in lobby state")]
    NotAvailableInLobbyState,

    /// Error indicating that this action is not allowed in the banker target state
    #[error("Action unavailable in lobby state")]
    NotAvailableInBankerTargetState,

    /// Error indicating that this action is not allowed in the results state
    #[error("Action unavailable in results state")]
    NotAvailableInResultsState,
}

/// Errors that can happen in the lobby phase.
#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
pub enum LobbyError {
    /// Username already in use.
    #[error("Username {0} already taken")]
    UsernameAlreadyTaken(String),

    /// Username didn't pass validation rules.
    #[error("Username is invalid")]
    InvalidUsername,
}

/// Errors that can happen when someone plays a card.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum PlayCardError {
    /// Provided card index is out of bounds.
    #[error("Card index {0} is invalid")]
    InvalidCardIndex(u8),

    /// Player tried to play more assets than allowed.
    #[error("Already played the maximum allowed number of assets")]
    ExceedsMaximumAssets,

    /// Player tried to play more liabilities than allowed.
    #[error("Already played the maximum allowed number of liabilities")]
    ExceedsMaximumLiabilities,

    /// Player doesn't have enough cash to afford the asset.
    #[error("{cash} cash is not enough to afford asset worth {cost}")]
    CannotAffordAsset {
        /// The amount of cash a player has
        cash: u8,
        /// The cost of the asset
        cost: u8,
    },
}

/// Errors that can happen when redeeming a liability.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum RedeemLiabilityError {
    /// Character type is not allowed to redeem liabilities.
    #[error("Character type '{0:?}' cannot redeem liability")]
    NotAllowedToRedeemLiability(Character),

    /// Already played the maximum allowed number of liabilities.
    #[error("Already played the maximum allowed number of liabilities")]
    ExceedsMaximumLiabilities,

    /// Provided liability index is invalid.
    #[error("Invalid liability index {0}")]
    InvalidLiabilityIndex(u8),

    /// Player doesn't have enough cash to redeem.
    #[error("{cash} gold is not enough to redeem liability with value {cost}")]
    NotEnoughCash {
        /// The amount of cash a player has
        cash: u8,
        /// The cost of the asset
        cost: u8,
    },
}

/// Errors that can happen when a player must give back a card.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum GiveBackCardError {
    /// Provided card index is invalid.
    #[error("Card index {0} is invalid")]
    InvalidCardIndex(u8),

    /// Player does not need to give back any card.
    #[error("Player does not have to give back card")]
    Unnecessary,
}

/// Errors related to firing a character.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum FireCharacterError {
    /// The given character cannot be fired.
    #[error("Character is not allowed to be fired")]
    InvalidCharacter,

    /// Only a particular player may fire the character (role mismatch).
    #[error("Only the shareholder can fire a character")]
    InvalidPlayerCharacter,

    /// Player has already fired a character this turn.
    #[error("Player has already fired a character this turn")]
    AlreadyFiredThisTurn,
}

/// Errors related to terminating a character's credit line.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum TerminateCreditCharacterError {
    /// The given character cannot be fired.
    #[error("Character is not allowed to be fired")]
    InvalidCharacter,

    /// Only a particular player may fire the character (role mismatch).
    #[error("Only the shareholder can fire a character")]
    InvalidPlayerCharacter,

    /// Player has already fired a character this turn.
    #[error("Player has already fired a character this turn")]
    AlreadyFiredThisTurn,
}

/// Errors related to swapping hands/cards.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum SwapError {
    /// Player has already swapped this turn.
    #[error("Player has already swapped their hand this turn")]
    AlreadySwapedThisTurn,

    /// Only the regulator character can swap.
    #[error("Only the regulator can swap their hand")]
    InvalidPlayerCharacter,

    /// Provided card indexes are invalid.
    #[error("invalid card indexes")]
    InvalidCardIdxs,

    /// Can't swap with the provided player target.
    #[error("cant swap with this player")]
    InvalidTargetPlayer,
}

/// Errors related to divesting assets.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum DivestAssetError {
    /// Character type cannot divest assets.
    #[error("Can't divest assets from this character")]
    InvalidCharacter,

    /// Only the stakeholder (role) may divest.
    #[error("Only the stakeholder can divest assets")]
    InvalidPlayerCharacter,

    /// Already divested an asset this turn.
    #[error("Player has already divested an asset this turn")]
    AlreadyDivestedThisTurn,

    /// Cannot divest red or green asset types.
    #[error("can't divest red or green assets")]
    CantDivestAssetType,

    /// Not enough cash to divest this asset (typo preserved in message).
    #[error("You don't have enough cach to divest this asset")]
    NotEnoughCash,

    /// Invalid card index provided (typo preserved in message).
    #[error("invalid card idex")]
    InvalidCardIdx,
}

/// Errors related to drawing cards.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum DrawCardError {
    /// Character has already drawn the maximum allowed for the turn.
    #[error("Already drew {0} cards, which is the maximum for this character")]
    MaximumCardsDrawn(u8),
}

/// Errors that can happen while selecting characters.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum SelectingCharactersError {
    /// The game is not currently in the character-selection state.
    #[error("Game is not in a state where characters are being picked")]
    NotPickingCharacters,

    /// Player already selected the provided character.
    #[error("Already selected character {0:?}")]
    AlreadySelectedCharacter(Character),

    /// Chosen character is unavailable to pick.
    #[error("Character is not availalble to pick")]
    UnavailableCharacter,

    /// Action is restricted to the chairman.
    #[error("Player is not chairman")]
    NotChairman,
}

/// Errors that can happen while performing card abilities
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, PartialEq, Error, Serialize, Deserialize)]
pub enum AssetAbilityError {
    /// Player does not have a card with an ability at that index
    #[error("Player does not have a card with an ability at index {0}")]
    InvalidAbilityIndex(usize),
    /// Player does not have that ability (anymore).
    #[error("This player does not have a card ability '{0:?}")]
    PlayerDoesNotHaveAbility(AssetPowerup),
    /// Player already confirmed choice for this particular asset ability. They cannot change it
    /// anymore.
    #[error("Player already confirmed choice for asset index {0}")]
    AlreadyConfirmedAssetIndex(u8),
}
