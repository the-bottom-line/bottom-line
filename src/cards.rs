use std::{collections::HashSet, fs::read_to_string, path::Path};

use either::Either;
use nanorand::{Rng, WyRand};
use serde::{Deserialize, Serialize};

use crate::{
    Color, Market,
    game::{Asset, AssetPowerup, Deck, Event, Liability, LiabilityType, MarketCondition},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LoadedCards {
    metadata: LoadedCardsMetadata,
    deck_list: DeckList,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LoadedCardsMetadata {
    version: String,
    gamemode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeckList {
    asset_deck: Deck<AssetCard>,
    liability_deck: Deck<LiabilityCard>,
    market_events_deck: Deck<MarketEventCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AssetCard {
    title: String,
    color: Color,
    gold_value: u8,
    silver_value: u8,
    copies: u8,
    card_image_url: String,
    ability: Option<AssetPowerup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LiabilityCard {
    liability_type: LiabilityType,
    gold_value: u8,
    copies: u8,
    card_image_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MarketEventCard {
    pub title: String,
    pub card_image_url: String,
    pub copies: u32,

    #[serde(flatten)]
    pub details: MarketEventDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum MarketEventDetails {
    MarketStatus { market_status: MarketStatusCard },
    Event { event: EventCard },
}

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

#[derive(Debug, Clone)]
pub struct GameData {
    pub assets: Deck<Asset>,
    pub liabilities: Deck<Liability>,
    pub market_deck: Deck<Either<Market, Event>>,
}

impl GameData {
    pub fn new<P: AsRef<Path>>(cards_json_path: P) -> anyhow::Result<Self> {
        let json = read_to_string(cards_json_path)?;

        let cards = serde_json::from_str::<LoadedCards>(&json)?;

        Ok(Self::from(cards))
    }

    pub fn shuffle_all(&mut self) {
        let mut rng = WyRand::new();

        rng.shuffle(&mut self.assets.deck);
        rng.shuffle(&mut self.liabilities.deck);
        rng.shuffle(&mut self.market_deck.deck);
    }
}

impl From<Deck<AssetCard>> for Deck<Asset> {
    fn from(cards: Deck<AssetCard>) -> Self {
        let image_back_url = cards.image_back_url.clone();
        let deck = cards
            .deck
            .into_iter()
            .map(|c| {
                // keep borrow checker happy about moving an Rc into each Asset
                let image_back_url = image_back_url.clone();

                (0..c.copies).into_iter().map(move |_| Asset {
                    title: c.title.clone(),
                    gold_value: c.gold_value,
                    silver_value: c.silver_value,
                    color: c.color,
                    ability: c.ability,
                    image_front_url: c.card_image_url.clone(),
                    image_back_url: image_back_url.clone(),
                })
            })
            .flatten()
            .collect::<Vec<_>>();

        Deck {
            image_back_url,
            deck,
        }
    }
}

impl From<Deck<LiabilityCard>> for Deck<Liability> {
    fn from(cards: Deck<LiabilityCard>) -> Self {
        let image_back_url = cards.image_back_url;
        let deck = cards
            .deck
            .into_iter()
            .map(|c| {
                // keep borrow checker happy about moving an Rc into each Liability
                let image_back_url = image_back_url.clone();

                (0..c.copies).into_iter().map(move |_| Liability {
                    value: c.gold_value,
                    rfr_type: c.liability_type,
                    image_front_url: c.card_image_url.clone(),
                    image_back_url: image_back_url.clone(),
                })
            })
            .flatten()
            .collect::<Vec<_>>();

        Deck {
            image_back_url,
            deck,
        }
    }
}

impl From<Deck<MarketEventCard>> for Deck<Either<Market, Event>> {
    fn from(cards: Deck<MarketEventCard>) -> Self {
        let image_back_url = cards.image_back_url;
        let deck = cards
            .deck
            .into_iter()
            .map(|c| {
                // keep borrow checker happy about moving an Rc into each Liability
                let image_back_url = image_back_url.clone();

                (0..c.copies)
                    .into_iter()
                    .map(move |_| match c.details.clone() {
                        MarketEventDetails::MarketStatus { market_status } => {
                            Either::Left(Market {
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
                            })
                        }
                        MarketEventDetails::Event { event } => Either::Right(Event {
                            title: c.title.clone(),
                            description: event.description.clone(),
                            plus_gold: HashSet::new(),
                            minus_gold: HashSet::new(),
                            skip_turn: None,
                        }),
                    })
            })
            .flatten()
            .collect::<Vec<_>>();

        Deck {
            image_back_url,
            deck,
        }
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
    fn thing() {
        let json = r#"{
            "title": "Thing",
            "color": "Purple",
            "gold_value": 2,
            "silver_value": 3,
            "copies": 1,
            "card_image_url": "assets/patent_1-2.webp"
        }"#;
        let card: AssetCard = serde_json::from_str(json).unwrap();
        dbg!(card);
    }

    #[test]
    fn thing2() {
        let json = include_str!("../assets/cards/boardgame.json");

        let parsed = serde_json::from_str::<LoadedCards>(json).unwrap();

        let data = GameData::from(parsed);

        println!("data: {data:#?}")
    }
}
