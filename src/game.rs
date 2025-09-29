use std::collections::{HashMap, HashSet};

use either::Either;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Color {
    Red,
    Green,
    Purple,
    Yellow,
    Blue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Role {
    Shareholder,
    Banker,
    Regulator,
    CEO,
    CFO,
    CSO,
    HeadRnD,
    Stakeholder,
}

impl Role {
    pub fn color(&self) -> Option<Color> {
        use Color::*;

        match self {
            Role::Shareholder => None,
            Role::Banker => None,
            Role::Regulator => None,
            Role::CEO => Some(Yellow),
            Role::CFO => Some(Blue),
            Role::CSO => Some(Green),
            Role::HeadRnD => Some(Purple),
            Role::Stakeholder => Some(Red),
        }
    }

    pub fn next(&self) -> Option<Role> {
        match self {
            Role::Shareholder => Some(Role::Banker),
            Role::Banker => Some(Role::Regulator),
            Role::Regulator => Some(Role::CEO),
            Role::CEO => Some(Role::CFO),
            Role::CFO => Some(Role::CSO),
            Role::CSO => Some(Role::HeadRnD),
            Role::HeadRnD => Some(Role::Stakeholder),
            Role::Stakeholder => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetPowerup {
    MinusIntoPlus,
    SilverIntoGold,
    CountAsAnyColor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub gold_value: u8,
    pub silver_value: u8,
    pub color: Color,
    pub asset_powerup: Option<AssetPowerup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LiabilityType {
    TradeCredit,
    BankLoan,
    Bonds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Liability {
    value: u8,
    rfr_type: LiabilityType,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub cash: u8,
    pub assets: Vec<Asset>,
    pub liabilities: Vec<Liability>,
    pub hand: Vec<Either<Asset, Liability>>,
    pub gold: u8, // could be unnecessary
    pub silver: u8, // could be unnecessary
    pub cards_to_grab: u8,
    pub assets_to_play: u8,
    pub liabilities_to_play: u8,
}

impl Player {
    pub fn new(name: &str) -> Player {
        Player {
            name: name.to_string(),
            cash: 1,
            assets: Vec::new(),
            liabilities: Vec::new(),
            hand: Vec::new(),
            gold: 0,
            silver: 0,
            cards_to_grab: 3,
            assets_to_play: 1,
            liabilities_to_play: 1,
        }
    }
    
    pub fn play_hand(&mut self, idx: usize) {
        if let Some(card) = self.hand.get(idx) {
            match card {
                Either::Left(_) if self.assets_to_play > 0 => {
                    let asset = self.hand.remove(idx).left().unwrap();
                    self.assets_to_play -= 1;
                    self.assets.push(asset)
                },
                Either::Right(_) if self.liabilities_to_play > 0 => {
                    let liability = self.hand.remove(idx).right().unwrap();
                    self.liabilities_to_play -= 1;
                    self.liabilities.push(liability)
                },
                _ => {}
            }
        }
    }
    
    pub fn draw_card(&mut self, ) {
        
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {
    skip_turn: Option<Role>,
    plus_gold: HashSet<Color>,
    minus_gold: HashSet<Color>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    title: String,
    text: String,
    minus_gold: HashSet<Color>,
    plus_gold: HashSet<Color>,
    event: EventData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketCondition {
    Plus,
    Zero,
    Minus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    rfr: u8,
    mrp: u8,
    red: MarketCondition,
    green: MarketCondition,
    blue: MarketCondition,
    yellow: MarketCondition,
    purple: MarketCondition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CardType {
    Asset,
    Liability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    asset_deck: Vec<Asset>,
    liability_deck: Vec<Liability>,
    market_deck: Vec<Either<Market, Event>>,
    players: HashMap<Role, Player>,
    current_turn: Role,
    current_market: Market,
    current_events: Vec<Event>,
    highest_amount_of_assets: u8,
    is_end_of_round: bool
}

impl Game {
    pub fn play_turn(&mut self) {
        match self.current_turn {
            Role::Shareholder => {}
            Role::Banker => todo!(),
            Role::Regulator => todo!(),
            Role::CEO => todo!(),
            Role::CFO => todo!(),
            Role::CSO => todo!(),
            Role::HeadRnD => todo!(),
            Role::Stakeholder => todo!(),
        }
        
        match self.current_turn.next() {
            Some(role) => {
                self.current_turn = role;
            },
            None => {
                self.is_end_of_round = true;
            },
        }
    }

    pub fn draw_asset_card(&mut self) -> Asset {
        // we know assets cannot run out in a normal game so this is safe
        self.asset_deck.pop().unwrap()
    }

    pub fn draw_liability_card(&mut self) -> Liability {
        // we know liabilities cannot run out in a normal game so this is safe
        self.liability_deck.pop().unwrap()
    }

    pub fn draw_market_card(&mut self) -> Either<Market, Event> {
        self.market_deck.pop().unwrap()
    }
}
