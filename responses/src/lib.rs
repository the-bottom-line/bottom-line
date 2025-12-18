//! This crate contains all requests and responses used by the frontend of _The Bottom Line_.

#![warn(missing_docs)]

use either::Either;
use game::{errors::GameError, game::*, player::*, utility::serde_asset_liability};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "ts")]
use ts_rs::TS;

/// The connect response. The very first thing a client should send is this request.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = game::SHARED_TS_DIR))]
#[cfg_attr(feature = "ts", ts(export))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum Connect {
    /// The connect action
    Connect {
        /// The username of the player who wants to connect.
        username: String,
        /// The channel code of the player who wants to connect.
        channel: String,
    },
}

/// Requests that are sent from the frontend, to be handled by the backend.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = game::SHARED_TS_DIR))]
#[cfg_attr(feature = "ts", ts(export))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum FrontendRequest {
    /// Tries to start the game.
    StartGame,
    /// Tries to select a character for this player.
    SelectCharacter {
        /// The character the player wants to select.
        character: Character,
    },
    /// Tries to draw a card for this player.
    DrawCard {
        /// The [`CardType`] the player wants to draw.
        card_type: CardType,
    },
    /// Tries to put back a card for this player.
    PutBackCard {
        /// The index of the card this player is trying to put back.
        card_idx: usize,
    },
    /// Tries to buy an asset for this player.
    BuyAsset {
        /// The index of the card the player wants to buy.
        card_idx: usize,
    },
    /// Tries to issue a liability for this player.
    IssueLiability {
        /// The index of the card the player wants to issue.
        card_idx: usize,
    },
    /// Tries to redeem a liability for this player.
    RedeemLiability {
        /// The index of the issued liability the player wanst to redeem.
        liability_idx: usize,
    },
    /// Tries to use the ability for this player.
    UseAbility,
    /// Tries to fire a particular character by this player.
    FireCharacter {
        /// The character that is to be fired.
        character: Character,
    },
    /// Tries toterminate credit from particular character by this player.
    TerminateCreditCharacter {
        /// The character who's credit line will be terminated.
        character: Character,
    },
    SelectAssetToDivest {
        asset_id: usize,
    },
    UnselectAssetToDivest {
        asset_id: usize,
    },
    SelectLiabilityToIssue {
        liability_id: usize,
    },
    UnselectLiabilityToIssue {
        liability_id: usize,
    },
    /// Tries to send cash to the banker when player is targeted
    PayBanker {
        /// The amount of cash to pay
        cash: u8,
    },
    /// Tries to swap a list of card indices with the deck for this player.
    SwapWithDeck {
        /// The list of card indices to be swapped with the deck.
        card_idxs: Vec<usize>,
    },
    /// Tries to swap hands of this player with another player.
    SwapWithPlayer {
        /// The id of the player which is to be swapped with.
        target_player_id: PlayerId,
    },
    /// Tries to force another player to divest one of their assets at market value minus one, which
    /// is to be paid by this player.
    DivestAsset {
        /// The id of the player which would be forced to divest their asset.
        target_player_id: PlayerId,
        /// The index of the asset that is to be divested.
        card_idx: usize,
    },
    /// Tries to end the turn of this player.
    EndTurn,
    /// Tries to turn minus into zero or zero into plus for the player's market at the end of the
    /// game. Related to [`AssetPowerup::MinusIntoPlus`](game::player::AssetPowerup::MinusIntoPlus).
    MinusIntoPlus {
        /// The color to change the minus from.
        color: Color,
    },
    /// Tries to turn the silver of a particular asset into gold. Related to
    /// [`AssetPowerup::SilverIntoGold`](game::player::AssetPowerup::MinusIntoPlus).
    SilverIntoGold {
        /// The index of the asset to change silver into gold from.
        asset_idx: usize,
    },
    /// Tries to change the color of any bought asset into another color. Related to
    /// [`AssetPowerup::ChangeAssetColor`](game::player::AssetPowerup::ChangeAssetColor).
    ChangeAssetColor {
        /// The index of the asset to change color from.
        asset_idx: usize,
        /// The new color of the asset.
        color: Color,
    },
    /// Tries to confirm the usage of a asset ability.
    ConfirmAssetAbility {
        /// The index of the asset which ability was used.
        asset_idx: usize,
    },
}

/// A response type that a player receives after performing an action. Can either be an error or
/// a confirmation that the action was succesful, including the data needed to update the UI
/// accordingly.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = game::SHARED_TS_DIR))]
#[cfg_attr(feature = "ts", ts(export))]
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum DirectResponse {
    /// An error returned when the action was not succesful.
    Error {
        /// The error message.
        message: String,
        /// The error type.
        source: ResponseError,
    },
    /// Confirmation that this player started the game.
    YouStartedGame,
    /// Confirmation that this player selected a character.
    YouSelectedCharacter {
        /// The character this player selected.
        character: Character,
    },
    /// Confirmation that this player fired a character.
    YouFiredCharacter {
        /// The character that was fired.
        character: Character,
    },
    /// Confirmation that this character's credit line is terminated.
    YouTerminateCreditCharacter {
        /// The character who's credit line was terminated.
        character: Character,
    },
    /// Confirmation that you paid some gold to the banker.
    YouPaidBanker {
        /// The amount of gold paid
        banker_id: PlayerId,
        new_banker_cash: u8,
        your_new_cash: u8,
        paid_amount: u8,
        sold_assets: Vec<SoldAssetToPayBanker>,
        issued_liabilities: Vec<IssuedLiabilityToPayBanker>,
    },
    YouSelectCardBankerTarget {
        assets: Vec<SoldAssetToPayBanker>,
        liabilities: Vec<IssuedLiabilityToPayBanker>,
    },
    /// Confirmation that this player was succesful in getting regulator options
    YouRegulatorOptions {
        /// The options this player has to swap with other players.
        options: Vec<RegulatorSwapPlayer>,
        /// Always [`Character::Regulator`]
        character: Character,
        /// A string containing information about what this character is allowed to do.
        perk: String,
    },
    /// Confirmation that this player can proceed swapping with the deck.
    YouSwapDeck {
        /// The amount of cards this player may draw from the deck.
        cards_to_draw: usize,
    },
    /// Confirmation that this player was succesful in swapping with a player.
    YouSwapPlayer {
        /// The new hand of the player.
        #[cfg_attr(
            feature = "ts",
            ts(as = "Vec<serde_asset_liability::EitherAssetLiability>")
        )]
        #[serde(with = "serde_asset_liability::vec")]
        new_cards: Vec<Either<Asset, Liability>>,
    },
    /// Confirmation that this player is now forcing another player to divest.
    YouAreDivesting {
        /// A list of cards for each player, which can either be or not be divested.
        options: Vec<DivestPlayer>,
        /// Always [`Character::Stakeholder`]
        character: Character,
        /// A string containing information about what this character is allowed to do.
        perk: String,
    },
    /// Confirmation that this player drew a card.
    YouDrewCard {
        /// The card that was drawn
        #[cfg_attr(feature = "ts", ts(as = "serde_asset_liability::EitherAssetLiability"))]
        #[serde(with = "serde_asset_liability::value")]
        card: Either<Asset, Liability>,
        /// Whether this player can draw another card.
        can_draw_cards: bool,
        /// Whether this player should still give back any cards.
        can_give_back_cards: bool,
    },
    /// Confirmation that this player put back a card.
    YouPutBackCard {
        /// The index of the card this player put back.
        card_idx: usize,
        /// Whether this player can draw another card.
        can_draw_cards: bool,
        /// Whether this player should still give back any cards.
        can_give_back_cards: bool,
    },
    /// Confirmation that this player is trying to use their character ability.
    YouCharacterAbility {
        /// The character of the player.
        character: Character,
        /// A string containing information about what this player is allowed to do.
        perk: String,
    },
    /// Confirmation that this player bought an asset.
    YouBoughtAsset {
        /// The asset this player bought.
        asset: Asset,
        /// If the market changed, a list of events and a new market is returned.
        market_change: Option<MarketChange>,
    },
    /// Confirmation that this player issued a liability.
    YouIssuedLiability {
        /// The liability the player issued.
        liability: Liability,
    },
    /// Confirmation that this player
    YouAreFiringSomeone {
        /// The list of available characters to fire.
        characters: Vec<Character>,
        /// Always [`Character::Shareholder`]
        character: Character,
        /// A string containing information on what this character is allowed to do.
        perk: String,
    },
    /// Confirmation that this player divested an asset of another player.
    YouDivestedAnAsset {
        /// The id of the player who is forced to divest one of their assets.
        target_id: PlayerId,
        /// The index of the asset they are forced to divest.
        asset_idx: usize,
        /// The amount of gold it cost to divest this asset.
        gold_cost: u8,
    },
    /// Confirmation that this player is terminating the credit of another player.
    YouAreTerminatingSomeone {
        /// A list of characters whose credit can be terminated.
        characters: Vec<Character>,
        /// Always [`Character::Banker`]
        character: Character,
        /// A string containing information on what this character is allowed to do.
        perk: String,
    },
    /// Confirmation that this player redeemed a liability.
    YouRedeemedLiability {
        /// The index of the liability that was redeemed.
        liability_idx: usize,
    },
    /// Confirmation that this player ended their turn.
    YouEndedTurn,
    /// Confirms that this player changed one of their market colors.
    YouMinusedIntoPlus {
        /// The market color that was changed,
        color: Color,
        /// The new market for this player.
        new_market: Market,
        /// The updated player score.
        new_score: f64,
    },
    /// Confirms that this player changed the silver of one of their cards into gold.
    YouSilveredIntoGold {
        /// The data of the old asset that should be updated. When no card was selected before this
        /// action, this value will be `None`.
        old_asset_data: Option<SilverIntoGoldData>,
        /// The data of the new asset that should be updated. When a card was deselected, this value
        /// will be `None`.
        new_asset_data: Option<SilverIntoGoldData>,
        /// The updated player score.
        new_score: f64,
    },
    /// Confirms that this player changed the color of one of their assets.
    YouChangedAssetColor {
        /// The data of the old asset that should be updated. When no card was selected before this
        /// action, this value will be `None`.
        old_asset_data: Option<ChangeAssetColorData>,
        /// The data of the new asset that should be updated. When a card was deselected, this value
        /// will be `None`.
        new_asset_data: Option<ChangeAssetColorData>,
        /// The updated player score.
        new_score: f64,
    },
    /// Confirms that this player confirmed an asset ability's choice.
    YouConfirmedAssetAbility {
        /// The asset the player confirmed their choice for.
        asset_idx: usize,
    },
}

impl From<ResponseError> for DirectResponse {
    fn from(error: ResponseError) -> Self {
        DirectResponse::Error {
            message: error.to_string(),
            source: error,
        }
    }
}

impl From<GameError> for DirectResponse {
    fn from(error: GameError) -> Self {
        DirectResponse::Error {
            message: error.to_string(),
            source: error.into(),
        }
    }
}

/// A response type that is meant for every other player when one player performs an action.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = game::SHARED_TS_DIR))]
#[cfg_attr(feature = "ts", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum UniqueResponse {
    /// When someone joins or leaves, this response is sent which includes information on the
    /// updated list of players.
    PlayersInLobby {
        /// The name of the player that joined/left.
        changed_player: String,
        /// The new list of usernames.
        usernames: Vec<String>,
    },
    /// Sent when the game was started.
    StartGame {
        /// This player's personal id.
        id: PlayerId,
        /// The amount of cash this player gets.
        cash: u8,
        /// The player's hand.
        #[cfg_attr(
            feature = "ts",
            ts(as = "Vec<serde_asset_liability::EitherAssetLiability>")
        )]
        #[serde(with = "serde_asset_liability::vec")]
        hand: Vec<Either<Asset, Liability>>,
        /// Public info about every other player.
        player_info: Vec<PlayerInfo>,
        /// The market at the start of the game.
        initial_market: Market,
    },
    /// Sent when a [`SelectingCharacters`](game::game::SelectingCharacters) stage begins.
    SelectingCharacters {
        /// The id of the chairman, or the person who selects a character first.
        chairman_id: PlayerId,
        /// If it's this player's turn, a list of characters that can be selected.
        selectable_characters: Option<Vec<Character>>,
        /// A list of characters that cannot be selected by anyone.
        open_characters: Vec<Character>,
        /// A character that only the chairman can see, but not select.
        closed_character: Option<Character>,
        /// The order each player selects a character in.
        turn_order: Vec<PlayerId>,
    },
    /// Sent when someone selected a character.
    SelectedCharacter {
        /// The id of the player that's currently selecting.
        currently_picking_id: Option<PlayerId>,
        /// If it's this player's turn, a list of characters that can be selected.
        selectable_characters: Option<Vec<Character>>,
        /// A character that only the chairman can see, but not select.
        closed_character: Option<Character>,
    },
    /// Sent when someone's turn starts.
    TurnStarts {
        /// Id of the player whose turn it is
        player_turn: PlayerId,
        /// Extra cash received by the player whose turn it is
        player_turn_cash: u8,
        /// The amount of cards this player draws.
        draws_n_cards: u8,
        /// The amount of cards this player gives back.
        gives_back_n_cards: u8,
        /// The amount of assets this player can play, where each color asset has a different 'unit
        /// cost' attached to it.
        playable_assets: PlayableAssets,
        /// The amount of liabilities this player can play.
        playable_liabilities: u8,
        /// The character of this player.
        player_character: Character,
        /// A list of characters which were called but were not available.
        skipped_characters: Vec<Character>,
    },
    PlayerTargetedByBanker {
        /// Id of the player whose turn it is
        player_turn: PlayerId,
        /// Amount of Cash to be paid to Banker
        cash_to_be_paid: u8,
        /// Amount of Cash to be paid to Banker
        is_possible_to_pay_banker: bool,
    },
    SelectedCardsBankerTarget {
        assets: Vec<SoldAssetToPayBanker>,
        liability_count: usize,
    },
    /// Sent when someone drew a card.
    DrewCard {
        /// The id of the player who drew a card.
        player_id: PlayerId,
        /// The type of card this player drew.
        card_type: CardType,
    },
    /// Sent when someone put back a card.
    PutBackCard {
        /// The id of the player who put back a card.
        player_id: PlayerId,
        /// The type of card this player put back.
        card_type: CardType,
    },
    /// Sent when a player bought an asset.
    BoughtAsset {
        /// The id of the player who bought an asset.
        player_id: PlayerId,
        /// The asset this player bought.
        asset: Asset,
        /// If buying the asset changed the market, sends a list of events as well as the new
        /// market.
        market_change: Option<MarketChange>,
    },
    /// Sent when a player issued a liability.
    IssuedLiability {
        /// The id of the player who issued a liability
        player_id: PlayerId,
        /// The liability this player issued.
        liability: Liability,
    },
    /// Sent when a player
    RedeemedLiability {
        /// The id of the player who
        player_id: PlayerId,
        /// The index of the liability this player redeemed.
        liability_idx: usize,
    },
    /// Sent when the shareholder is in the process of firing someone.
    ShareholderIsFiring {},
    /// Sent when the shareholder fired a character.
    FiredCharacter {
        /// The id of the player who fired someone.
        player_id: PlayerId,
        /// The character which was fired.
        character: Character,
    },
    /// sent when a characters credit line has been terminated
    TerminatedCreditCharacter {
        /// The id of the player who teminated the credit line someone.
        player_id: PlayerId,
        /// The character who's credit line was terminated.
        character: Character,
    },
    PlayerPaidBanker {
        banker_id: PlayerId,
        player_id: PlayerId,
        new_banker_cash: u8,
        new_target_cash: u8,
        paid_amount: u8,
        sold_assets: Vec<SoldAssetToPayBanker>,
        issued_liabilities: Vec<IssuedLiabilityToPayBanker>,
    },
    /// Sent when the regulator swapped their hand with this player.
    RegulatorSwappedYourCards {
        /// This player's new hand.
        #[cfg_attr(
            feature = "ts",
            ts(as = "Vec<serde_asset_liability::EitherAssetLiability>")
        )]
        #[serde(with = "serde_asset_liability::vec")]
        new_cards: Vec<Either<Asset, Liability>>,
    },
    /// Sent when the regulator swapped their hand with another player.
    SwappedWithPlayer {
        /// The id of the regulator.
        regulator_id: PlayerId,
        /// The id of the player the regulator swapped their hands with.
        target_id: PlayerId,
    },
    /// Sent when the regulator swapped a number of cards with the deck.
    SwappedWithDeck {
        /// The amount of assets the regulator drew from the deck.
        asset_count: usize,
        /// The amount of liabilities the regulator drew from the deck.
        liability_count: usize,
    },
    /// Sent when the stakeholder forced another player to divest an asset.
    AssetDivested {
        /// The id of the stakeholder.
        player_id: PlayerId,
        /// The id of the player who is forced to divest one of their assets.
        target_id: PlayerId,
        /// The index of the asset they are forced to divest.
        asset_idx: usize,
        /// The amount of gold the stakeholder paid to divest this asset.
        paid_gold: u8,
    },
    /// Sent when someone's turn ended
    TurnEnded {
        /// The id of the player whose turn ended.
        player_id: PlayerId,
    },
    /// Sent when the game ended.
    GameEnded {
        /// A list of player scores.
        scores: Vec<PlayerScore>,
    },
    /// Confirms that a player changed one of their market colors.
    MinusedIntoPlus {
        /// The id of the player which changed one of their market colors.
        player_id: PlayerId,
        /// The new market for the player that performed the action,
        new_market: Market,
        /// The updated player score.
        new_score: f64,
    },
    /// Confirms that a player changed the silver of one of their cards into gold.
    SilveredIntoGold {
        /// The id of the player which changed the silver of one of their cards into gold.
        player_id: PlayerId,
        /// The data of the old asset that should be updated. When no card was selected before this
        /// action, this value will be `None`.
        old_asset_data: Option<SilverIntoGoldData>,
        /// The data of the new asset that should be updated. When a card was deselected, this value
        /// will be `None`.
        new_asset_data: Option<SilverIntoGoldData>,
        /// The updated player score.
        new_score: f64,
    },
    /// Confirms that a player changed the color of one of their assets.
    ChangedAssetColor {
        /// The id of the player which changed the color of one of their assets.
        player_id: PlayerId,
        /// The data of the old asset that should be updated. When no card was selected before this
        /// action, this value will be `None`.
        old_asset_data: Option<ChangeAssetColorData>,
        /// The data of the new asset that should be updated. When a card was deselected, this value
        /// will be `None`.
        new_asset_data: Option<ChangeAssetColorData>,
        /// The updated player score.
        new_score: f64,
    },
    /// Confirms that a player confirmed an asset ability's choice.
    ConfirmedAssetAbility {
        /// The id of the player which confirmed an asset ability's choice.
        player_id: PlayerId,
        /// The asset the player confirmed their choice for.
        asset_idx: usize,
    },
}

/// The general error type that can be sent back in a response.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = game::SHARED_TS_DIR))]
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum ResponseError {
    /// A [`GameError`](game::errors::GameError)
    #[error(transparent)]
    Game(#[from] GameError),
    /// An error sent when tha game has not yet started.
    #[error("Game has not yet started")]
    GameNotYetStarted,
    /// An error sent when a player tries to join when the game is already in progress.
    #[error("Game has already started")]
    GameAlreadyStarted,
    /// An error sent when the data the player sent in invalid.
    #[error("Data is not valid for this state")]
    InvalidData,
}
