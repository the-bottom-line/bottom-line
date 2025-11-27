use std::collections::HashMap;

use either::Either;
use game::{
    errors::GameError,
    game::{Market, MarketChange},
    player::*,
    utility::serde_asset_liability,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum Connect {
    Connect { username: String, channel: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum FrontendRequest {
    StartGame,
    SelectCharacter {
        character: Character,
    },
    DrawCard {
        card_type: CardType,
    },
    PutBackCard {
        card_idx: usize,
    },
    BuyAsset {
        card_idx: usize,
    },
    IssueLiability {
        card_idx: usize,
    },
    RedeemLiability {
        liability_idx: usize,
    },
    UseAbility,
    FireCharacter {
        character: Character,
    },
    SwapWithDeck {
        card_idxs: Vec<usize>,
    },
    SwapWithPlayer {
        target_player_id: PlayerId,
    },
    DivestAsset {
        target_player_id: PlayerId,
        card_idx: usize,
    },
    EndTurn,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum DirectResponse {
    Error(ResponseError),
    YouStartedGame,
    YouSelectedCharacter {
        character: Character,
    },
    YouFiredCharacter {
        character: Character,
    },
    YouRegulatorOptions {
        options: Vec<RegulatorSwapPlayer>,
        character: Character,
        perk: String,
    },
    YouSwapDeck {
        cards_to_draw: usize,
    },
    YouSwapPlayer {
        #[serde(with = "serde_asset_liability::vec")]
        new_cards: Vec<Either<Asset, Liability>>,
    },
    YouAreDivesting {
        options: Vec<DivestPlayer>,
        character: Character,
        perk: String,
    },
    YouDrewCard {
        #[serde(with = "serde_asset_liability::value")]
        card: Either<Asset, Liability>,
        can_draw_cards: bool,
        can_give_back_cards: bool,
    },
    YouPutBackCard {
        card_idx: usize,
        can_draw_cards: bool,
        can_give_back_cards: bool,
    },
    YouCharacterAbility {
        character: Character,
        perk: String,
    },
    YouBoughtAsset {
        asset: Asset,
        market_change: Option<MarketChange>,
    },
    YouIssuedLiability {
        liability: Liability,
    },
    YouAreFiringSomeone {
        characters: Vec<Character>,
        character: Character,
        perk: String,
    },
    YouDivestedAnAsset {
        gold_cost: u8,
    },
    YouAreTerminatingSomeone {
        characters: Vec<Character>,
        character: Character,
        perk: String,
    },
    YouRedeemedLiability {
        liability_idx: usize,
    },
    YouEndedTurn,
}

impl From<GameError> for DirectResponse {
    fn from(error: GameError) -> Self {
        DirectResponse::Error(error.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum UniqueResponse {
    PlayersInLobby {
        changed_player: String,
        usernames: Vec<String>,
    },
    StartGame {
        id: PlayerId,
        cash: u8,
        #[serde(with = "serde_asset_liability::vec")]
        hand: Vec<Either<Asset, Liability>>,
        player_info: Vec<PlayerInfo>,
        initial_market: Market,
    },
    SelectingCharacters {
        chairman_id: PlayerId,
        selectable_characters: Option<Vec<Character>>,
        open_characters: Vec<Character>,
        closed_character: Option<Character>,
        turn_order: Vec<PlayerId>,
    },
    SelectedCharacter {
        currently_picking_id: Option<PlayerId>,
        selectable_characters: Option<Vec<Character>>,
        closed_character: Option<Character>,
    },
    TurnStarts {
        /// Id of the player whose turn it is
        player_turn: PlayerId,
        /// Extra cash received by the player whose turn it is
        player_turn_cash: u8,
        draws_n_cards: u8,
        gives_back_n_cards: u8,
        playable_assets: PlayableAssets,
        playable_liabilities: u8,
        player_character: Character,
        skipped_characters: Vec<Character>,
    },
    DrewCard {
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
        market_change: Option<MarketChange>,
    },
    IssuedLiability {
        player_id: PlayerId,
        liability: Liability,
    },
    RedeemedLiability {
        player_id: PlayerId,
        liability_idx: usize,
    },
    ShareholderIsFiring {},
    FiredCharacter {
        player_id: PlayerId,
        character: Character,
    },
    RegulatorSwapedYourCards {
        #[serde(with = "serde_asset_liability::vec")]
        new_cards: Vec<Either<Asset, Liability>>,
    },
    SwapedWithPlayer {
        regulator_id: PlayerId,
        target_id: PlayerId,
    },
    SwapedWithDeck {
        card_count: usize,
    },
    AssetDivested {
        player_id: PlayerId,
        target_id: PlayerId,
        card_idx: usize,
        paid_gold: u8,
    },
    TurnEnded {
        player_id: PlayerId,
    },
    GameEnded {
        scores: HashMap<PlayerId, f64>,
    },
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum ResponseError {
    #[error(transparent)]
    Game(#[from] GameError),
    #[error("Game has not yet started")]
    GameNotYetStarted,
    #[error("Game has already started")]
    GameAlreadyStarted,
    #[error("Data is not valid for this state")]
    InvalidData,
}
