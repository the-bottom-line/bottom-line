//! This file contains all information to be able to load in `boardgame.json`, which is a json
//! representation of the game assets of The Bottom Line.
//!
//! Example:
//! ```not_rust
//! {
//!   "metadata": {
//!     "version": "0.1",
//!     "gamemode": "board_version"
//!   },
//!   "deck_list": {
//!     "asset_deck": {
//!       "card_image_back_url": "asset_back.webp",
//!       "card_list": [
//!         {
//!           "title": "R&D Lab",
//!           "color": "Purple",
//!           "gold_value": 3,
//!           "silver_value": 1,
//!           "copies": 1,
//!           "ability": "At the end of the game, for one color, turn - into 0 or 0 into +",
//!           "card_image_url": "assets/rndLab_3-1.webp"
//!         },
//!       ]
//!     },
//!     "liability_deck": {
//!       "card_image_back_url": "liability_back.webp",
//!       "card_list": [
//!         {
//!           "liability_type": "Trade Credit",
//!           "gold_value": 1,
//!           "copies": 4,
//!           "card_image_url": "liabilities/tradeCredit_1.webp"
//!         }
//!       ]
//!     },
//!     "market_events_deck": {
//!       "card_image_back_url": "market_back.webp",
//!       "card_list": [
//!         {
//!           "title": "Global Treaty on Climate Change",
//!           "event": {
//!             "description": "Negotiations have succeeded and all UN-member states have agreed on a global treaty to fight climate change.",
//!             "effect": "The CSO has to skip their turn to formulate the corporate alignment strategy. All Green assets gain one gold."
//!           },
//!           "copies": 1,
//!           "card_image_url": "events/climateChange.webp"
//!         },
//!         {
//!           "title": "Stable Market",
//!           "market_status": {
//!             "rfr": 4,
//!             "mrp": 4
//!           },
//!           "copies": 1,
//!           "card_image_url": "events/stable_01.webp"
//!         }
//!       ]
//!     }
//!   }
//! }
//! ```
//!

use either::Either;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use std::{collections::HashSet, fs::read_to_string, path::Path};

use crate::{game::*, player::*};

/// Errors that can occur when parsing or loading data.
#[derive(Debug, Error)]
pub enum DataParseError {
    /// A std::io::Error
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// a serde_json::Error
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

/// Represents the json in its entirety
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LoadedCards {
    /// Has all information related to versioning and game mode
    metadata: LoadedCardsMetadata,
    /// Has the asset deck, liability deck and market and events deck
    deck_list: DeckList,
}

/// Card metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LoadedCardsMetadata {
    /// The json version
    version: String,
    /// The name of the gamemode
    gamemode: String,
}

/// The list of decks in the json: assets, liabilities and markets/events.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeckList {
    /// List of all asset cards in the game
    asset_deck: Deck<AssetCard>,
    /// List of all liability cards in the game
    liability_deck: Deck<LiabilityCard>,
    /// List of all market/event cards in the game
    market_events_deck: Deck<MarketEventCard>,
}

/// Representation of an asset in the json
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AssetCard {
    /// Title of the card
    title: String,
    /// Color of the card
    color: Color,
    /// Gold value of the card
    gold_value: u8,
    /// Silver value of the card
    silver_value: u8,
    /// Amount of times the card appears in the deck
    copies: u8,
    /// Url containing the relative location of the card in the assets folder
    card_image_url: String,
    /// Possible ability of the card
    ability: Option<AssetPowerup>,
}

/// Representation of a liability card as it appears in the json
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LiabilityCard {
    /// Type of liability: Trade Credit, Bank Loan or Bonds
    liability_type: LiabilityType,
    /// Gold value of the liability
    gold_value: u8,
    /// Amount of times the card appears in the deck
    copies: u8,
    /// Url containing the relative location of the card in the assets folder
    card_image_url: String,
}

/// Representation of either a market or event card as it appears in the json
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MarketEventCard {
    /// Title of the card
    pub title: String,
    /// Amount of times the card appears in the deck
    pub copies: u32,
    /// Url containing the relative location of the card in the assets folder
    pub card_image_url: String,

    /// Representation of either the market or event specific fields, because both are in this list
    #[serde(flatten)]
    pub details: MarketEventDetails,
}

/// Enum representing the fields only found on either a market or an event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum MarketEventDetails {
    MarketStatus { market_status: MarketStatusCard },
    Event { event: EventCard },
}

/// Card representing the non-shared fields with event as it appears in the json
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MarketStatusCard {
    pub rfr: u8,
    pub mrp: u8,
    #[serde(rename = "Yellow", default)]
    pub yellow: MarketCondition,
    #[serde(rename = "Blue", default)]
    pub blue: MarketCondition,
    #[serde(rename = "Green", default)]
    pub green: MarketCondition,
    #[serde(rename = "Purple", default)]
    pub purple: MarketCondition,
    #[serde(rename = "Red", default)]
    pub red: MarketCondition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventCard {
    pub description: String,
    pub effect: String,
}

/// Overarching struct which contains the asset deck, the liability deck and the market/event deck.
/// This can be used by the game or anything else that might want to get all cards.
#[derive(Debug, Clone)]
pub struct GameData {
    /// Deck containing all assets
    pub assets: Deck<Asset>,
    /// Deck containing all liabilities
    pub liabilities: Deck<Liability>,
    /// Deck containing all markets and events
    pub market_deck: Deck<Either<Market, Event>>,
}

impl GameData {
    /// Tries loading a json at `cards_json_path`. It reads the file to string and tries to parse
    /// that string into a [`GameData`] struct using `serde_json`.
    pub fn new<P: AsRef<Path>>(cards_json_path: P) -> Result<GameData, DataParseError> {
        let json = read_to_string(cards_json_path)?;

        let cards = serde_json::from_str::<LoadedCards>(&json)?;

        Ok(Self::from(cards))
    }

    /// Shuffles each individual deck.
    #[cfg(feature = "shuffle")]
    pub fn shuffle_all(&mut self) {
        self.assets.shuffle();
        self.liabilities.shuffle();
        self.market_deck.shuffle();
    }
}

impl From<Deck<AssetCard>> for Deck<Asset> {
    fn from(cards: Deck<AssetCard>) -> Self {
        let image_back_url = cards.image_back_url.clone();
        let deck = cards
            .deck
            .into_iter()
            .flat_map(|c| {
                // keep borrow checker happy about moving an Arc into each Asset
                let image_back_url = image_back_url.clone();

                (0..c.copies).map(move |_| Asset {
                    title: c.title.clone(),
                    gold_value: c.gold_value,
                    silver_value: c.silver_value,
                    color: c.color,
                    ability: c.ability,
                    image_front_url: c.card_image_url.clone(),
                    image_back_url: image_back_url.clone(),
                })
            })
            .collect::<Vec<_>>();

        Deck::new_with_url(deck, &image_back_url)
    }
}

impl From<Deck<LiabilityCard>> for Deck<Liability> {
    fn from(cards: Deck<LiabilityCard>) -> Self {
        let image_back_url = cards.image_back_url;
        let deck = cards
            .deck
            .into_iter()
            .flat_map(|c| {
                // keep borrow checker happy about moving an Arc into each Liability
                let image_back_url = image_back_url.clone();

                (0..c.copies).map(move |_| Liability {
                    value: c.gold_value,
                    rfr_type: c.liability_type,
                    image_front_url: c.card_image_url.clone(),
                    image_back_url: image_back_url.clone(),
                })
            })
            .collect::<Vec<_>>();

        Deck::new_with_url(deck, &image_back_url)
    }
}

impl From<Deck<MarketEventCard>> for Deck<Either<Market, Event>> {
    fn from(cards: Deck<MarketEventCard>) -> Self {
        let image_back_url = cards.image_back_url;
        let deck = cards
            .deck
            .into_iter()
            .flat_map(|c| {
                // keep borrow checker happy about moving an Rc into each Liability
                let image_back_url = image_back_url.clone();

                (0..c.copies).map(move |_| match c.details.clone() {
                    MarketEventDetails::MarketStatus { market_status } => Either::Left(Market {
                        title: c.title.clone(),
                        rfr: market_status.rfr,
                        mrp: market_status.mrp,
                        red: market_status.red,
                        green: market_status.green,
                        blue: market_status.blue,
                        yellow: market_status.yellow,
                        purple: market_status.purple,
                        image_front_url: c.card_image_url.clone(),
                        image_back_url: image_back_url.clone(),
                    }),
                    MarketEventDetails::Event { event } => Either::Right(Event {
                        title: c.title.clone(),
                        description: event.description.clone(),
                        plus_gold: HashSet::new(),
                        minus_gold: HashSet::new(),
                        skip_turn: None,
                    }),
                })
            })
            .collect::<Vec<_>>();

        Deck::new_with_url(deck, &image_back_url)
    }
}

impl From<LoadedCards> for GameData {
    fn from(cards: LoadedCards) -> Self {
        GameData {
            assets: cards.deck_list.asset_deck.into(),
            liabilities: cards.deck_list.liability_deck.into(),
            market_deck: cards.deck_list.market_events_deck.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_counts() {
        let data = GameData::new("../assets/cards/boardgame.json").expect("could not load data");

        assert_eq!(data.assets.len(), 60);
        assert_eq!(data.liabilities.len(), 50);
        assert_eq!(data.market_deck.len(), 25);
    }
}
