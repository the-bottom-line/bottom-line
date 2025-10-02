use std::{
    collections::{HashMap, HashSet},
    path::Path,
    rc::Rc,
};

use either::Either;
use nanorand::{Rng, WyRand};
use serde::{Deserialize, Serialize};

use crate::cards::GameData;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Color {
    Red,
    Green,
    Purple,
    Yellow,
    Blue,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
        match self {
            Self::Shareholder => Some(Self::Banker),
            Self::Banker => Some(Self::Regulator),
            Self::Regulator => Some(Self::CEO),
            Self::CEO => Some(Self::CFO),
            Self::CFO => Some(Self::CSO),
            Self::CSO => Some(Self::HeadRnD),
            Self::HeadRnD => Some(Self::Stakeholder),
            Self::Stakeholder => None,
        }
    }

    pub const fn first() -> Self {
        Self::Shareholder
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
    pub image_back_url: Rc<String>,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
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
    pub image_back_url: Rc<String>,
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

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum CardType {
    Asset,
    Liability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub cash: u8,
    pub assets: Vec<Asset>,
    pub liabilities: Vec<Liability>,
    pub hand: Vec<Either<Asset, Liability>>,
    pub cards_drawn: Vec<usize>,
    pub assets_to_play: u8,
    pub liabilities_to_play: u8,
}

impl Player {
    pub fn new(name: &str, assets: [Asset; 2], liabilities: [Liability; 2], cash: u8) -> Player {
        let hand = assets
            .into_iter()
            .map(Either::Left)
            .chain(liabilities.into_iter().map(Either::Right))
            .collect();
        
        Player {
            name: name.to_string(),
            cash,
            assets: vec![],
            liabilities: vec![],
            hand,
            cards_drawn: vec![],
            assets_to_play: 1,
            liabilities_to_play: 1,
        }
    }

    pub fn gold(&self) -> u8 {
        self.assets.iter().map(|a| a.gold_value).sum()
    }

    pub fn silver(&self) -> u8 {
        self.assets.iter().map(|a| a.silver_value).sum()
    }

    pub fn play_card(&mut self, idx: usize) {
        if let Some(card) = self.hand.get(idx) {
            match card {
                Either::Left(a) if self.assets_to_play > 0 && self.cash >= a.gold_value => {
                    let asset = self.hand.remove(idx).left().unwrap();
                    self.cash -= asset.gold_value;
                    self.assets_to_play -= 1;
                    self.assets.push(asset)
                }
                Either::Right(_) if self.liabilities_to_play > 0 => {
                    let liability = self.hand.remove(idx).right().unwrap();
                    self.liabilities_to_play -= 1;
                    self.liabilities.push(liability)
                }
                _ => {}
            }
        }
    }

    pub fn draw_card(&mut self, card: Either<Asset, Liability>) {
        self.cards_drawn.push(self.hand.len());
        self.hand.push(card);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub title: String,
    pub description: String,
    pub plus_gold: HashSet<Color>,
    pub minus_gold: HashSet<Color>,
    pub skip_turn: Option<Character>,
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize)]
pub enum MarketCondition {
    #[serde(rename = "up")]
    Plus,
    #[serde(rename = "down")]
    Minus,
    #[default]
    #[serde(other)]
    Zero,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub title: String,
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
    pub image_front_url: String,
    pub image_back_url: Rc<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck<T> {
    #[serde(rename = "card_image_back_url")]
    pub image_back_url: Rc<String>,
    #[serde(rename = "card_list")]
    pub deck: Vec<T>,
}

impl<T> Deck<T> {
    pub fn new() -> Self {
        Self {
            deck: vec![],
            image_back_url: String::new().into(),
        }
    }

    /// Panics if no more cards are in the deck, for now. Decks don't run out in regular games.
    pub fn draw(&mut self) -> T {
        self.deck.pop().unwrap()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    assets: Deck<Asset>,
    liabilities: Deck<Liability>,
    market_deck: Deck<Either<Market, Event>>,
    players: HashMap<Character, Player>,
    current_turn: Character,
    current_market: Market,
    current_events: Vec<Event>,
    highest_amount_of_assets: u8,
}

impl Game {
    pub fn new(player_count: usize, mut game_data: GameData) -> Self {
        game_data.shuffle_all();

        let current_market = Self::get_first_market(&mut game_data.market_deck)
            .expect("The default deck should have a market");

        Game {
            players: Self::get_players(
                player_count,
                &mut game_data.assets,
                &mut game_data.liabilities,
            ),
            assets: game_data.assets,
            liabilities: game_data.liabilities,
            market_deck: game_data.market_deck,
            current_turn: Character::first(),
            current_market,
            current_events: vec![],
            highest_amount_of_assets: 0,
        }
    }

    /// Grab market card if available and reshuffles the rest of the deck.
    fn get_first_market(deck: &mut Deck<Either<Market, Event>>) -> Option<Market> {
        let mut rng = WyRand::new();

        if let Some(pos) = deck.deck.iter().position(|c| c.is_left()) {
            let market = deck.deck.swap_remove(pos).left();
            rng.shuffle(&mut deck.deck);
            market
        } else {
            None
        }
    }

    pub fn get_players(
        player_count: usize,
        assets: &mut Deck<Asset>,
        liabilites: &mut Deck<Liability>,
    ) -> HashMap<Character, Player> {
        assert!(
            player_count >= 4 && player_count <= 7,
            "This game supports playing with 4 to 7 players"
        );

        (0..player_count)
            .into_iter()
            .zip(Character::CHARACTERS)
            .map(|(i, character)| {
                let assets = [assets.draw(), assets.draw()];
                let liabilities = [liabilites.draw(), liabilites.draw()];
                let player = Player::new(&format!("Player {i}"), assets, liabilities, 1);
                (character, player)
            })
            .collect()
    }

    pub fn player_play_card(&mut self, character: Character, card_idx: usize) {
        if character == self.current_turn {
            if let Some(player) = self.players.get_mut(&character) {
                player.play_card(card_idx)
            }
        }
    }

    pub fn player_draw_card(&mut self, character: Character, card_type: CardType) {
        if character == self.current_turn {
            if let Some(player) = self.players.get_mut(&character) {
                if player.cards_drawn.len() < 3 {
                    let card = match card_type {
                        CardType::Asset => Either::Left(self.assets.draw()),
                        CardType::Liability => Either::Right(self.liabilities.draw()),
                    };
                    player.cards_drawn.push(player.hand.len());
                    player.hand.push(card);
                }
            }
        }
    }

    pub fn end_player_turn(&mut self) {
        if let Some(role) = self.current_turn.next() {
            self.current_turn = role;
        } else {
            // end of round, reshuffle cards
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draw_cards() {
        let data = GameData::new("assets/cards/boardgame.json").expect("this should exist");
        let game = Game::new(4, data);

        println!("{game:#?}");
    }
}
