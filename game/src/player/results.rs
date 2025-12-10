//! This file contains the implementation of [`ResultsPlayer`].

use either::Either;

use crate::{game::*, player::*};

/// The player type that corresponds to the [`Results`](crate::game::Results) stage of the game.
/// During the results stage, each player can calculate and see their score.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultsPlayer {
    id: PlayerId,
    name: String,
    cash: u8,
    assets: Vec<Asset>,
    liabilities: Vec<Liability>,
    hand: Vec<Either<Asset, Liability>>,
    market: Market,
}

impl ResultsPlayer {
    /// Creates a new `ResultsPlayer` based on a [`RoundPlayer`] and a given [`Market`]. Because of
    /// the [`MinusIntoPlus`](crate::player::AssetPowerup) asset ability, each player keeps track of
    /// their own market.
    pub fn new(player: RoundPlayer, market: &Market) -> Self {
        Self {
            id: player.id,
            name: player.name,
            cash: player.cash,
            assets: player.assets,
            liabilities: player.liabilities,
            hand: player.hand,
            market: market.clone(),
        }
    }

    /// Gets the id of the player
    pub fn id(&self) -> PlayerId {
        self.id
    }

    /// Gets the name of the player
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets the amount of cash of the player
    pub fn cash(&self) -> u8 {
        self.cash
    }

    /// Gets a list of bought assets of the player
    pub fn assets(&self) -> &[Asset] {
        &self.assets
    }

    /// Gets a list of issued liabilities of the player
    pub fn liabilities(&self) -> &[Liability] {
        &self.liabilities
    }

    /// Gets the hand of this player.
    pub fn hand(&self) -> &[Either<Asset, Liability>] {
        &self.hand
    }

    /// Resets back to the passed `final_market` and then turns the minus of a certain color into a
    /// zero or a zero into a plus.
    pub fn minus_into_plus(&mut self, color: Color, final_market: &Market) -> &Market {
        self.market = final_market.clone();

        match color {
            Color::Red => self.market.red.make_higher(),
            Color::Green => self.market.green.make_higher(),
            Color::Purple => self.market.purple.make_higher(),
            Color::Yellow => self.market.yellow.make_higher(),
            Color::Blue => self.market.blue.make_higher(),
        }

        &self.market
    }

    /// Gets tho total gold value of all assets this player owns
    pub fn total_gold(&self) -> u8 {
        self.assets.iter().map(|a| a.gold_value).sum()
    }

    /// Gets tho total silver value of all assets this player owns
    pub fn total_silver(&self) -> u8 {
        self.assets.iter().map(|a| a.silver_value).sum()
    }

    /// Gets the amount of debt this player has of a certain [`LiabilityType`].
    fn calc_loan(&self, rfr_type: LiabilityType) -> u8 {
        self.liabilities
            .iter()
            .filter_map(|l| (l.rfr_type == rfr_type).then_some(l.value))
            .sum()
    }

    /// Gets the amount of trade credit debt this player has.
    pub fn trade_credit(&self) -> u8 {
        self.calc_loan(LiabilityType::TradeCredit)
    }

    /// Gets the amount of bank loan debt this player has.
    pub fn bank_loan(&self) -> u8 {
        self.calc_loan(LiabilityType::BankLoan)
    }

    /// Gets the amount of bonds debt this player has.
    pub fn bonds(&self) -> u8 {
        self.calc_loan(LiabilityType::Bonds)
    }

    /// Gets the value of all assets of a certain color this player has
    pub fn color_value(&self, color: Color) -> f64 {
        let market_condition = match color {
            Color::Red => self.market.red,
            Color::Green => self.market.green,
            Color::Purple => self.market.purple,
            Color::Yellow => self.market.yellow,
            Color::Blue => self.market.blue,
        };

        let mul = match market_condition {
            MarketCondition::Plus => 1.0,
            MarketCondition::Minus => -1.0,
            MarketCondition::Zero => 0.0,
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

    /// Gets the final score for this player
    pub fn score(&self) -> f64 {
        let gold = self.total_gold() as f64;
        let silver = self.total_silver() as f64;

        let trade_credit = self.trade_credit() as f64;
        let bank_loan = self.bank_loan() as f64;
        let bonds = self.bonds() as f64;
        let debt = trade_credit + bank_loan + bonds;

        let beta = silver / gold;

        // TODO: end of game bonuses
        let drp = (trade_credit + bank_loan * 2.0 + bonds * 3.0) / gold;

        let wacc = self.market.rfr as f64 + drp + beta * self.market.mrp as f64;

        let red = self.color_value(Color::Red);
        let green = self.color_value(Color::Green);
        let yellow = self.color_value(Color::Yellow);
        let purple = self.color_value(Color::Purple);
        let blue = self.color_value(Color::Blue);

        let fcf = red + green + yellow + purple + blue;

        (fcf / (10.0 * wacc)) + (debt / 3.0) + self.cash() as f64
    }
}

impl From<&ResultsPlayer> for PlayerInfo {
    fn from(player: &ResultsPlayer) -> Self {
        Self {
            name: player.name.clone(),
            id: player.id,
            hand: Self::hand(&player.hand),
            assets: player.assets.clone(),
            liabilities: player.liabilities.clone(),
            cash: player.cash,
            character: None,
        }
    }
}
