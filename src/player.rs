use std::sync::Arc;

use either::Either;
use serde::{Deserialize, Serialize};

use crate::{
    errors::*,
    game::{Market, MarketCondition},
    utility::serde_asset_liability,
};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerId(pub u8);

impl<I: Into<u8>> From<I> for PlayerId {
    fn from(value: I) -> Self {
        Self(value.into())
    }
}

impl From<PlayerId> for usize {
    fn from(value: PlayerId) -> Self {
        value.0 as usize
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Color {
    Red,
    Green,
    Purple,
    Yellow,
    Blue,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Character {
    Shareholder,
    Banker,
    Regulator,
    CEO,
    CFO,
    CSO,
    HeadRnD,
    Stakeholder,
}

impl Character {
    pub const CHARACTERS: [Character; 8] = [
        Self::Shareholder,
        Self::Banker,
        Self::Regulator,
        Self::CEO,
        Self::CFO,
        Self::CSO,
        Self::HeadRnD,
        Self::Stakeholder,
    ];

    pub fn color(&self) -> Option<Color> {
        use Color::*;

        match self {
            Self::Shareholder => None,
            Self::Banker => None,
            Self::Regulator => None,
            Self::CEO => Some(Yellow),
            Self::CFO => Some(Blue),
            Self::CSO => Some(Green),
            Self::HeadRnD => Some(Purple),
            Self::Stakeholder => Some(Red),
        }
    }

    pub fn next(&self) -> Option<Self> {
        use Character::*;

        match self {
            Shareholder => Some(Banker),
            Banker => Some(Regulator),
            Regulator => Some(CEO),
            CEO => Some(CFO),
            CFO => Some(CSO),
            CSO => Some(HeadRnD),
            HeadRnD => Some(Stakeholder),
            Stakeholder => None,
        }
    }

    pub fn first(characters: &[Self]) -> Option<Self> {
        characters.iter().max().copied()
    }

    pub fn playable_assets(&self) -> usize {
        match self {
            Self::CEO => 3,
            _ => 1,
        }
    }

    pub fn playable_liabilities(&self) -> usize {
        match self {
            Self::CFO => 3,
            _ => 1,
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum AssetPowerup {
    #[serde(rename = "At the end of the game, for one color, turn - into 0 or 0 into +")]
    MinusIntoPlus,
    #[serde(rename = "At the end of the game, turn silver into gold on one asset card")]
    SilverIntoGold,
    #[serde(rename = "At the end of the game, count one of your assets as any color")]
    CountAsAnyColor,
}

impl std::fmt::Display for AssetPowerup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MinusIntoPlus => write!(
                f,
                "At the end of the game, for one color, turn - into 0 or 0 into +"
            ),
            Self::SilverIntoGold => write!(
                f,
                "At the end of the game, turn silver into gold on one asset card"
            ),
            Self::CountAsAnyColor => write!(
                f,
                "At the end of the game, count one of your assets as any color"
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub title: String,
    pub gold_value: u8,
    pub silver_value: u8,
    pub color: Color,
    pub ability: Option<AssetPowerup>,
    pub image_front_url: String,
    pub image_back_url: Arc<String>,
}

impl std::fmt::Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\ngold: {}\nsilver: {}\ncolor: {:?}",
            self.title, self.gold_value, self.silver_value, self.color
        )
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LiabilityType {
    #[serde(rename = "Trade Credit")]
    TradeCredit,
    #[serde(rename = "Bank Loan")]
    BankLoan,
    Bonds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Liability {
    pub value: u8,
    pub rfr_type: LiabilityType,
    pub image_front_url: String,
    pub image_back_url: Arc<String>,
}

impl Liability {
    pub fn rfr_percentage(&self) -> u8 {
        match self.rfr_type {
            LiabilityType::TradeCredit => 1,
            LiabilityType::BankLoan => 2,
            LiabilityType::Bonds => 3,
        }
    }
}

impl std::fmt::Display for Liability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let title = serde_json::to_string(&self.rfr_type).unwrap();
        write!(
            f,
            "{title} - {}%\nvalue: {}\n",
            self.rfr_percentage(),
            self.value
        )
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CardType {
    Asset,
    Liability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub name: String,
    pub id: PlayerId,
    pub hand: Vec<CardType>,
    pub assets: Vec<Asset>,
    pub liabilities: Vec<Liability>,
    pub cash: u8,
    pub character: Option<Character>,
}

impl From<&Player> for PlayerInfo {
    fn from(player: &Player) -> Self {
        let hand = player
            .hand
            .iter()
            .map(|e| match e {
                Either::Left(_) => CardType::Asset,
                Either::Right(_) => CardType::Liability,
            })
            .collect();

        Self {
            hand,
            name: player.name.clone(),
            assets: player.assets.clone(),
            liabilities: player.liabilities.clone(),
            id: player.id,
            cash: player.cash,
            character: player.character,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub cash: u8,
    pub assets: Vec<Asset>,
    pub liabilities: Vec<Liability>,
    pub character: Option<Character>,
    #[serde(with = "serde_asset_liability::vec")]
    pub hand: Vec<Either<Asset, Liability>>,
    pub cards_drawn: Vec<usize>,
    pub assets_to_play: u8,
    pub liabilities_to_play: u8,
    pub total_cards_drawn: u8,
    pub total_cards_given_back: u8,
}

impl Player {
    pub fn new(
        name: &str,
        id: u8,
        assets: [Asset; 2],
        liabilities: [Liability; 2],
        cash: u8,
    ) -> Player {
        let hand = assets
            .into_iter()
            .map(Either::Left)
            .chain(liabilities.into_iter().map(Either::Right))
            .collect();

        Player {
            id: PlayerId(id),
            name: name.to_string(),
            cash,
            assets: vec![],
            liabilities: vec![],
            character: None,
            hand,
            cards_drawn: vec![],
            assets_to_play: 1,
            liabilities_to_play: 1,
            total_cards_drawn: 0,
            total_cards_given_back: 0,
        }
    }

    pub fn info(&self) -> PlayerInfo {
        self.into()
    }

    fn update_cards_drawn(&mut self, card_idx: usize) {
        self.cards_drawn = self
            .cards_drawn
            .iter()
            .copied()
            .filter(|&i| i != card_idx)
            .collect();
    }

    fn can_play_asset(&self) -> bool {
        match self.character {
            Some(_) => self.assets_to_play > 0,
            None => false,
        }
    }

    fn can_play_liability(&self) -> bool {
        match self.character {
            Some(_) => self.liabilities_to_play > 0,
            None => false,
        }
    }

    /// Plays card in players hand with index `card_idx`. If that index is valid, the card is played
    /// if
    pub fn play_card(
        &mut self,
        card_idx: usize,
    ) -> Result<Either<Asset, Liability>, PlayCardError> {
        use PlayCardError::*;

        if let Some(card) = self.hand.get(card_idx) {
            match card {
                Either::Left(a) if self.can_play_asset() && self.cash >= a.gold_value => {
                    let asset = self.hand.remove(card_idx).left().unwrap();
                    self.cash -= asset.gold_value;
                    self.assets_to_play -= 1;
                    self.assets.push(asset.clone());
                    self.update_cards_drawn(card_idx);
                    Ok(Either::Left(asset))
                }
                Either::Left(_) if !self.can_play_asset() => Err(ExceedsMaximumAssets),
                Either::Left(a) if self.cash < a.gold_value => Err(CannotAffordAsset {
                    cash: self.cash,
                    cost: a.gold_value,
                }),
                Either::Right(_) if self.can_play_liability() => {
                    let liability = self.hand.remove(card_idx).right().unwrap();
                    self.cash += liability.value;
                    self.liabilities_to_play -= 1;
                    self.liabilities.push(liability.clone());
                    self.update_cards_drawn(card_idx);
                    Ok(Either::Right(liability))
                }
                Either::Right(_) if !self.can_play_liability() => Err(ExceedsMaximumLiabilities),
                _ => unreachable!(),
            }
        } else {
            Err(InvalidCardIndex(card_idx as u8))
        }
    }

    pub fn draw_card(&mut self, card: Either<Asset, Liability>) {
        self.total_cards_drawn += 1;
        self.cards_drawn.push(self.hand.len());
        self.hand.push(card);
    }

    pub fn give_back_card(
        &mut self,
        card_idx: usize,
    ) -> Result<Either<Asset, Liability>, GiveBackCardError> {
        self.total_cards_given_back += 1;

        match self.hand.get(card_idx) {
            Some(_) => {
                self.update_cards_drawn(card_idx);
                Ok(self.hand.remove(card_idx))
            }
            None => Err(GiveBackCardError::InvalidCardIndex(card_idx as u8)),
        }
    }

    pub fn should_give_back_cards(&self) -> bool {
        // TODO: add head rnd ability
        match self.character {
            Some(_) => self.total_cards_drawn - self.total_cards_given_back >= 3,
            None => false,
        }
    }

    pub fn can_draw_cards(&self) -> bool {
        // TODO: add head rnd ability
        self.total_cards_drawn < 3
    }

    pub fn select_character(&mut self, character: Character) {
        use Character::*;

        self.character = Some(character);

        match character {
            Shareholder => {}
            Banker => {}
            Regulator => {}
            CEO => {}
            CFO => {}
            CSO => {}
            HeadRnD => {}
            Stakeholder => {}
        }
    }

    pub fn total_gold(&self) -> u8 {
        self.assets.iter().map(|a| a.gold_value).sum()
    }

    pub fn total_silver(&self) -> u8 {
        self.assets.iter().map(|a| a.silver_value).sum()
    }

    fn calc_loan(&self, rfr_type: LiabilityType) -> u8 {
        self.liabilities
            .iter()
            .filter_map(|l| (l.rfr_type == rfr_type).then_some(l.value))
            .sum()
    }

    pub fn trade_credit(&self) -> u8 {
        self.calc_loan(LiabilityType::TradeCredit)
    }

    pub fn bank_loan(&self) -> u8 {
        self.calc_loan(LiabilityType::BankLoan)
    }

    pub fn bonds(&self) -> u8 {
        self.calc_loan(LiabilityType::Bonds)
    }

    pub fn color_value(&self, color: Color, market: &Market) -> f64 {
        let market_condition = match color {
            Color::Red => market.red,
            Color::Green => market.green,
            Color::Purple => market.purple,
            Color::Yellow => market.yellow,
            Color::Blue => market.blue,
        };

        let mul = match market_condition {
            MarketCondition::Plus => 1.0,
            MarketCondition::Minus => 0.0,
            MarketCondition::Zero => -1.0,
        };

        self.assets
            .iter()
            .filter_map(|a| {
                color
                    .eq(&a.color)
                    .then_some(a.gold_value as f64 + (a.silver_value as f64) * mul)
            })
            .sum()
    }
}
