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
    old_silver_into_gold: Option<SilverIntoGoldData>,
    old_change_asset_color: Option<ChangeAssetColorData>,
    confirmed_asset_ability_idxs: Vec<usize>,
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
            old_silver_into_gold: None,
            old_change_asset_color: None,
            confirmed_asset_ability_idxs: vec![],
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

    /// Gets the player's personal market.
    pub fn market(&self) -> &Market {
        &self.market
    }

    fn asset_ability_prechecks(&self, asset_idx: usize) -> Result<(), GameError> {
        if self.assets.get(asset_idx).is_none() {
            return Err(GameError::InvalidAssetIndex(asset_idx as u8));
        }
        if self.confirmed_asset_ability_idxs.contains(&asset_idx) {
            return Err(AssetAbilityError::AlreadyConfirmedAssetIndex(asset_idx as u8).into());
        }

        Ok(())
    }

    /// Resets back to the passed `final_market` and then turns the minus of a certain color into a
    /// zero or a zero into a plus.
    pub fn minus_into_plus(&mut self, color: Color, final_market: &Market) -> &Market {
        // TODO: handle confirmation for this action
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

    /// Turns the silver value of one of this player's assets into gold.
    pub fn toggle_silver_into_gold(
        &mut self,
        asset_idx: usize,
    ) -> Result<ToggleSilverIntoGold, GameError> {
        self.asset_ability_prechecks(asset_idx)?;

        if let Some(old) = self.old_silver_into_gold {
            match self.assets.get_disjoint_mut([asset_idx, old.asset_idx]) {
                Ok([asset, old_asset]) => {
                    old_asset.gold_value -= old.silver_value;
                    old_asset.silver_value = old.silver_value;

                    asset.gold_value += asset.silver_value;
                    asset.silver_value = 0;

                    let old_data = SilverIntoGoldData::new(
                        old.asset_idx,
                        old_asset.gold_value,
                        old_asset.silver_value,
                    );
                    let new_data =
                        SilverIntoGoldData::new(asset_idx, asset.gold_value, asset.silver_value);

                    self.old_silver_into_gold = Some(new_data);

                    Ok(ToggleSilverIntoGold::new(Some(old_data), Some(new_data)))
                }
                Err(_) => {
                    // PANIC: we control old.asset_idx and know it is always valid because when it's
                    // set it's always valid.
                    let old_asset = self.assets.get_mut(old.asset_idx).unwrap();
                    let silver_value = old.silver_value;

                    old_asset.gold_value -= silver_value;
                    old_asset.silver_value = silver_value;

                    self.old_silver_into_gold = None;

                    let old_data = SilverIntoGoldData::new(
                        old.asset_idx,
                        old_asset.gold_value,
                        old_asset.silver_value,
                    );

                    Ok(ToggleSilverIntoGold::new(Some(old_data), None))
                }
            }
        } else {
            // PANIC: we already validated the index, so this is safe to do.
            let asset = self.assets.get_mut(asset_idx).unwrap();

            asset.gold_value += asset.silver_value;
            asset.silver_value = 0;

            let new_data = SilverIntoGoldData::new(asset_idx, asset.gold_value, asset.silver_value);

            self.old_silver_into_gold = Some(new_data);

            Ok(ToggleSilverIntoGold::new(None, Some(new_data)))
        }
    }

    /// Turns the color of one of this player's assets into another color. If they already did this,
    /// Also resets that asset's color.
    pub fn toggle_change_asset_color(
        &mut self,
        asset_idx: usize,
        color: Color,
    ) -> Result<ToggleChangeAssetColor, GameError> {
        self.asset_ability_prechecks(asset_idx)?;

        if let Some(old) = self.old_change_asset_color {
            match self.assets.get_disjoint_mut([asset_idx, old.asset_idx]) {
                Ok([asset, old_asset]) => {
                    old_asset.color = old.color;
                    asset.color = color;

                    let old_data = ChangeAssetColorData::new(old.asset_idx, old_asset.color);
                    let new_data = ChangeAssetColorData::new(asset_idx, asset.color);

                    self.old_change_asset_color = Some(new_data);

                    Ok(ToggleChangeAssetColor::new(Some(old_data), Some(new_data)))
                }
                Err(_) => {
                    // PANIC: we control old.asset_idx and know it is always valid because when it's
                    // set it's always valid.
                    let old_asset = self.assets.get_mut(old.asset_idx).unwrap();

                    old_asset.color = old.color;

                    self.old_silver_into_gold = None;

                    let old_data = ChangeAssetColorData::new(old.asset_idx, old_asset.color);

                    Ok(ToggleChangeAssetColor::new(Some(old_data), None))
                }
            }
        } else {
            // PANIC: we already validated the index, so this is safe to do.
            let asset = self.assets.get_mut(asset_idx).unwrap();

            asset.gold_value += asset.silver_value;
            asset.silver_value = 0;

            let new_data = ChangeAssetColorData::new(asset_idx, asset.color);

            self.old_change_asset_color = Some(new_data);

            Ok(ToggleChangeAssetColor::new(None, Some(new_data)))
        }
    }

    /// Asset abilities are toggleable by default. This function confirms the current configuration,
    /// after which a player cannot toggle this particular index anymore.
    pub fn confirm_asset_ability(&mut self, asset_idx: usize) -> Result<(), GameError> {
        self.asset_ability_prechecks(asset_idx)?;

        self.confirmed_asset_ability_idxs.push(asset_idx);

        Ok(())
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

    /// Calculates the fcf (total market value of all player's assets) for this player.
    pub fn fcf(&self) -> f64 {
        Color::COLORS
            .into_iter()
            .map(|color| self.color_value(color))
            .sum()
    }

    /// Gets the final score for this player
    pub fn score(&self) -> f64 {
        let cash = self.cash() as f64;
        let gold = self.total_gold() as f64;
        let silver = self.total_silver() as f64;

        let trade_credit = self.trade_credit() as f64;
        let bank_loan = self.bank_loan() as f64;
        let bonds = self.bonds() as f64;

        let beta = silver / (1.0 + gold);

        // TODO: end of game bonuses
        let drp = (trade_credit + bank_loan * 2.0 + bonds * 3.0) / (gold + cash);

        let rfr = self.market.rfr as f64;
        let mrp = self.market.mrp as f64;

        let fcf = self.fcf();
        let wacc = rfr + drp + beta * mrp;
        let debt = trade_credit + bank_loan + bonds;

        (fcf / (10.0 * wacc)) + (debt / 3.0) + cash
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

/// The representation of the result of toggling with [`SilverIntoGold`].
pub struct ToggleSilverIntoGold {
    /// The data for the new asset.
    pub old_asset_data: Option<SilverIntoGoldData>,
    /// The data for the old asset.
    pub new_asset_data: Option<SilverIntoGoldData>,
}

impl ToggleSilverIntoGold {
    /// Instantiates a new ToggleSilverIntoGold.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::player::{SilverIntoGoldData, ToggleSilverIntoGold};
    /// let new_data = SilverIntoGoldData::new(1, 2, 3);
    /// let old_data = SilverIntoGoldData::new(6, 7, 8);
    /// let toggled = ToggleSilverIntoGold::new(Some(old_data), Some(new_data));
    ///
    /// assert_eq!(toggled.new_asset_data.unwrap().asset_idx, 1);
    /// assert_eq!(toggled.new_asset_data.unwrap().gold_value, 2);
    /// assert_eq!(toggled.old_asset_data.unwrap().silver_value, 8);
    /// ```
    pub fn new(
        old_asset_data: Option<SilverIntoGoldData>,
        new_asset_data: Option<SilverIntoGoldData>,
    ) -> Self {
        Self {
            old_asset_data,
            new_asset_data,
        }
    }
}

/// A type that represents the changes made with the [`SilverIntoGold`] asset ability. It contains
/// the index of the asset that was changed, as well as its original silver value.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SilverIntoGoldData {
    /// The index of the asset in question.
    pub asset_idx: usize,
    /// The gold value of the asset in question.
    pub gold_value: u8,
    /// The silver value of the asset in question.
    pub silver_value: u8,
}

impl SilverIntoGoldData {
    /// Instantiates a new SilverIntoGoldData.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::player::SilverIntoGoldData;
    /// let data = SilverIntoGoldData::new(5, 6, 3);
    /// assert_eq!(data.asset_idx, 5);
    /// assert_eq!(data.gold_value, 6);
    /// assert_eq!(data.silver_value, 3);
    /// ```
    pub fn new(asset_idx: usize, gold_value: u8, silver_value: u8) -> Self {
        Self {
            asset_idx,
            gold_value,
            silver_value,
        }
    }
}

/// The representation of the result of toggling with [`ChangeAssetColor`].
pub struct ToggleChangeAssetColor {
    /// The data for the new asset.
    pub old_asset_data: Option<ChangeAssetColorData>,
    /// The data for the old asset.
    pub new_asset_data: Option<ChangeAssetColorData>,
}

impl ToggleChangeAssetColor {
    /// Instantiates a new ToggleChangeAssetColor.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::player::{ChangeAssetColorData, Color, ToggleChangeAssetColor};
    /// let new_data = ChangeAssetColorData::new(1, Color::Green);
    /// let old_data = ChangeAssetColorData::new(6, Color::Blue);
    /// let toggled = ToggleChangeAssetColor::new(Some(old_data), Some(new_data));
    ///
    /// assert_eq!(toggled.new_asset_data.unwrap().asset_idx, 1);
    /// assert_eq!(toggled.new_asset_data.unwrap().color, Color::Green);
    /// assert_eq!(toggled.old_asset_data.unwrap().color, Color::Blue);
    /// ```
    pub fn new(
        old_asset_data: Option<ChangeAssetColorData>,
        new_asset_data: Option<ChangeAssetColorData>,
    ) -> Self {
        Self {
            old_asset_data,
            new_asset_data,
        }
    }
}

/// A type that represents the changes made with the [`ChangeAssetColor`] asset ability. It contains
/// the index of the asset that was changed, as well as its original color.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeAssetColorData {
    /// The index of the asset in question.
    pub asset_idx: usize,
    /// The color of the asset in question.
    pub color: Color,
}

impl ChangeAssetColorData {
    /// Instantiates a new ChangeAssetColorData.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::player::{ChangeAssetColorData, Color};
    /// let data = ChangeAssetColorData::new(5, Color::Red);
    /// assert_eq!(data.asset_idx, 5);
    /// assert_eq!(data.color, Color::Red);
    /// ```
    pub fn new(asset_idx: usize, color: Color) -> Self {
        Self { asset_idx, color }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use assert_approx_eq::assert_approx_eq;
    use itertools::Itertools;

    use super::round::tests::*;
    use super::*;

    fn market(
        yellow: MarketCondition,
        blue: MarketCondition,
        green: MarketCondition,
        purple: MarketCondition,
        red: MarketCondition,
        rfr: u8,
        mrp: u8,
    ) -> Market {
        Market {
            title: Default::default(),
            rfr,
            mrp,
            yellow,
            blue,
            green,
            purple,
            red,
        }
    }

    fn results_player(
        cash: u8,
        assets: Vec<Asset>,
        liabilities: Vec<Liability>,
        market: Market,
    ) -> ResultsPlayer {
        ResultsPlayer {
            id: PlayerId(0),
            name: Default::default(),
            cash,
            assets,
            liabilities,
            hand: vec![],
            market,
            old_silver_into_gold: None,
            old_change_asset_color: None,
            confirmed_asset_ability_idxs: vec![],
        }
    }

    fn default_results_player() -> ResultsPlayer {
        let player = results_player(0, vec![], vec![], Market::default());

        assert!(player.assets().is_empty());
        assert!(player.liabilities().is_empty());

        player
    }

    fn liability_with_type(value: u8, rfr_type: LiabilityType) -> Liability {
        Liability {
            value,
            rfr_type,
            image_front_url: Default::default(),
            image_back_url: Default::default(),
        }
    }

    #[test]
    pub fn total_gold() {
        for i in 0..10 {
            let mut player = default_results_player();
            for _ in i..10 {
                // asset(Color) has 1 gold and 1 silver
                player.assets.push(asset(Color::Blue));
            }
            let total_gold = player.assets.iter().map(|a| a.gold_value).sum::<u8>();
            assert_eq!(total_gold, player.total_gold());
        }
    }

    #[test]
    pub fn total_silver() {
        for i in 0..10 {
            let mut player = default_results_player();
            for _ in i..10 {
                // asset(Color) has 1 gold and 1 silver
                player.assets.push(asset(Color::Blue));
            }
            let total_gold = player.assets.iter().map(|a| a.silver_value).sum::<u8>();
            assert_eq!(total_gold, player.total_silver());
        }
    }

    #[test]
    fn calc_loan() {
        let liability_value = 10;

        let rfr_types = [
            LiabilityType::TradeCredit,
            LiabilityType::BankLoan,
            LiabilityType::Bonds,
        ];

        for rfr_type in rfr_types {
            for i in 0..10 {
                let mut player = default_results_player();
                for _ in i..10 {
                    player
                        .liabilities
                        .push(liability_with_type(liability_value, rfr_type));
                }

                let total_value = (10 - i) * liability_value;
                assert_eq!(player.calc_loan(rfr_type), total_value, "{i}: {rfr_type:?}");

                let trade_credit = player.trade_credit();
                let bank_loan = player.bank_loan();
                let bonds = player.bonds();

                match rfr_type {
                    LiabilityType::TradeCredit => {
                        assert_eq!(trade_credit, total_value);

                        assert_eq!(bank_loan, 0);
                        assert_eq!(bonds, 0);
                    }
                    LiabilityType::BankLoan => {
                        assert_eq!(bank_loan, total_value);

                        assert_eq!(trade_credit, 0);
                        assert_eq!(bonds, 0);
                    }
                    LiabilityType::Bonds => {
                        assert_eq!(bonds, total_value);

                        assert_eq!(trade_credit, 0);
                        assert_eq!(bank_loan, 0);
                    }
                }
            }
        }
    }

    #[test]
    pub fn color_value() {
        let market_conditions = [
            MarketCondition::Minus,
            MarketCondition::Zero,
            MarketCondition::Plus,
        ];

        std::iter::repeat_n(market_conditions, 5)
            .multi_cartesian_product()
            .cartesian_product(std::iter::repeat_n(Color::COLORS, 6).multi_cartesian_product())
            .map(|(m, colors)| {
                let market = market(m[0], m[1], m[2], m[3], m[4], 0, 0);
                let mut player = results_player(10, vec![], vec![], market);
                for &c in colors.iter().take(5) {
                    player.assets.push(asset(c));
                }
                (player, colors[5])
            })
            .for_each(|(player, color_to_check)| {
                let market_condition = match color_to_check {
                    Color::Red => player.market().red,
                    Color::Green => player.market().green,
                    Color::Purple => player.market().purple,
                    Color::Yellow => player.market().yellow,
                    Color::Blue => player.market().blue,
                };

                let mul = match market_condition {
                    MarketCondition::Plus => 1.0,
                    MarketCondition::Zero => 0.0,
                    MarketCondition::Minus => -1.0,
                };

                let color_value = player
                    .assets
                    .iter()
                    .filter_map(|a| {
                        a.color.eq(&color_to_check).then(|| {
                            let gold = a.gold_value as f64;
                            let silver = a.silver_value as f64;
                            gold + silver * mul
                        })
                    })
                    .sum::<f64>();

                assert_approx_eq!(color_value, player.color_value(color_to_check));
            });
    }

    #[test]
    pub fn score() {
        let market_conditions = [
            MarketCondition::Minus,
            MarketCondition::Zero,
            MarketCondition::Plus,
        ];

        std::iter::repeat_n(market_conditions, 5)
            .multi_cartesian_product()
            .cartesian_product(std::iter::repeat_n(Color::COLORS, 5).multi_cartesian_product())
            .map(|(m, colors)| {
                let market = market(m[0], m[1], m[2], m[3], m[4], 1, 1);
                let mut player = results_player(10, vec![], vec![], market);
                for c in colors {
                    player.assets.push(asset(c));
                }
                player
            })
            .for_each(|player| {
                let cash = player.cash() as f64;
                let gold = player.total_gold() as f64;
                let silver = player.total_silver() as f64;

                let trade_credit = player.trade_credit() as f64;
                let bank_loan = player.bank_loan() as f64;
                let bonds = player.bonds() as f64;
                let debt = trade_credit + bank_loan + bonds;

                let beta = silver / (1.0 + gold);

                // TODO: end of game bonuses
                let drp = (trade_credit + bank_loan * 2.0 + bonds * 3.0) / (gold + cash);

                let rfr = player.market.rfr as f64;
                let mrp = player.market.mrp as f64;

                let wacc = rfr + drp + beta * mrp;

                let red = player.color_value(Color::Red);
                let green = player.color_value(Color::Green);
                let yellow = player.color_value(Color::Yellow);
                let purple = player.color_value(Color::Purple);
                let blue = player.color_value(Color::Blue);

                let fcf = red + green + yellow + purple + blue;

                let score = (fcf / (10.0 * wacc)) + (debt / 3.0) + cash;

                assert_approx_eq!(score, player.score());
            });
    }
}
