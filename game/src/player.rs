use either::Either;
use serde::{Deserialize, Serialize};

use std::sync::Arc;

use crate::{
    errors::*,
    game::{Market, MarketCondition},
};

#[derive(Debug, Clone)]
pub struct LobbyPlayer {
    pub id: PlayerId,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct SelectingCharactersPlayer {
    pub id: PlayerId,
    pub name: String,
    pub cash: u8,
    pub assets: Vec<Asset>,
    pub liabilities: Vec<Liability>,
    pub character: Option<Character>,
    pub hand: Vec<Either<Asset, Liability>>,
}

impl SelectingCharactersPlayer {
    pub fn new(
        name: &str,
        id: u8,
        assets: [Asset; 2],
        liabilities: [Liability; 2],
        cash: u8,
    ) -> Self {
        let hand = assets
            .into_iter()
            .map(Either::Left)
            .chain(liabilities.into_iter().map(Either::Right))
            .collect();

        SelectingCharactersPlayer {
            id: PlayerId(id),
            name: name.to_string(),
            cash,
            assets: vec![],
            liabilities: vec![],
            character: None,
            hand,
        }
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
}

impl From<RoundPlayer> for SelectingCharactersPlayer {
    fn from(player: RoundPlayer) -> Self {
        Self {
            id: player.id,
            name: player.name,
            cash: player.cash,
            assets: player.assets,
            liabilities: player.liabilities,
            character: None,
            hand: player.hand,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RoundPlayer {
    pub id: PlayerId,
    pub name: String,
    pub cash: u8,
    pub assets: Vec<Asset>,
    pub liabilities: Vec<Liability>,
    pub character: Character,
    pub hand: Vec<Either<Asset, Liability>>,
    pub cards_drawn: Vec<usize>,
    pub assets_to_play: u8,
    pub playable_assets: PlayableAssets,
    pub liabilities_to_play: u8,
    pub total_cards_drawn: u8,
    pub total_cards_given_back: u8,
}

impl RoundPlayer {
    fn update_cards_drawn(&mut self, card_idx: usize) {
        self.cards_drawn = self
            .cards_drawn
            .iter()
            .copied()
            .filter(|&i| i != card_idx)
            .collect();
    }

    fn can_play_asset(&self, color: Color) -> bool {
        match self
            .assets_to_play
            .checked_sub(self.playable_assets.color_cost(color))
        {
            Some(_) => true,
            None => false,
        }
    }

    fn can_play_liability(&self) -> bool {
        self.liabilities_to_play > 0
    }

    pub fn redeem_liability(
        &mut self,
        liability_idx: usize,
    ) -> Result<Liability, RedeemLiabilityError> {
        if self.character.can_redeem_liabilities() {
            if self.can_play_liability() {
                if let Some(liability) = self.liabilities.get(liability_idx) {
                    if liability.value <= self.cash {
                        self.liabilities_to_play -= 1;
                        self.cash -= liability.value;
                        Ok(self.liabilities.remove(liability_idx))
                    } else {
                        Err(RedeemLiabilityError::NotEnoughCash {
                            cash: self.cash,
                            cost: liability.value,
                        })
                    }
                } else {
                    Err(RedeemLiabilityError::InvalidLiabilityIndex(
                        liability_idx as u8,
                    ))
                }
            } else {
                Err(RedeemLiabilityError::ExceedsMaximumLiabilities)
            }
        } else {
            Err(RedeemLiabilityError::NotAllowedToRedeemLiability(
                self.character,
            ))
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
                Either::Left(a) if self.can_play_asset(a.color) && self.cash >= a.gold_value => {
                    let asset = self.hand.remove(card_idx).left().unwrap();
                    self.cash -= asset.gold_value;
                    self.assets_to_play -= self.playable_assets.color_cost(asset.color);
                    self.assets.push(asset.clone());
                    self.update_cards_drawn(card_idx);
                    Ok(Either::Left(asset))
                }
                Either::Left(a) if !self.can_play_asset(a.color) => Err(ExceedsMaximumAssets),
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
        self.total_cards_drawn - self.total_cards_given_back >= 3
    }

    pub fn can_draw_cards(&self) -> bool {
        // TODO: add head rnd ability
        self.total_cards_drawn < 3
    }

    pub fn draws_n_cards(&self) -> u8 {
        self.character.draws_n_cards()
    }

    pub fn playable_assets(&self) -> PlayableAssets {
        self.playable_assets
    }

    pub fn playable_liabilities(&self) -> u8 {
        self.character.playable_liabilities()
    }

    pub fn turn_start_cash(&self) -> i16 {
        1
    }

    pub fn asset_bonus(&self) -> i16 {
        match self.character.color() {
            Some(color) => self
                .assets
                .iter()
                .flat_map(|a| (a.color == color).then_some(1))
                .sum(),
            None => 0,
        }
    }

    pub fn market_condition_bonus(&self, current_market: &Market) -> i16 {
        match self.character.color() {
            Some(color) => match current_market.color_condition(color) {
                MarketCondition::Plus => 1,
                MarketCondition::Zero => 0,
                MarketCondition::Minus => -1,
            },
            None => 0,
        }
    }

    pub fn turn_cash(&self, current_market: &Market) -> u8 {
        let start = self.turn_start_cash();
        let asset_bonus = self.asset_bonus();
        let market_condition_bonus = self.market_condition_bonus(current_market);

        (start + asset_bonus + market_condition_bonus) as u8
    }

    pub(crate) fn start_turn(&mut self, current_market: &Market) {
        self.cash += self.turn_cash(current_market);
    }
}

impl TryFrom<SelectingCharactersPlayer> for RoundPlayer {
    type Error = GameError;

    fn try_from(player: SelectingCharactersPlayer) -> Result<Self, Self::Error> {
        match player.character {
            Some(character) => {
                let playable_assets = character.playable_assets();
                Ok(Self {
                    id: player.id,
                    name: player.name,
                    cash: player.cash,
                    assets: player.assets,
                    liabilities: player.liabilities,
                    character,
                    hand: player.hand,
                    cards_drawn: Vec::new(),
                    assets_to_play: playable_assets.total(),
                    playable_assets,
                    liabilities_to_play: character.playable_liabilities(),
                    total_cards_drawn: 0,
                    total_cards_given_back: 0,
                })
            }
            None => Err(GameError::PlayerMissingCharacter),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResultsPlayer {
    pub id: PlayerId,
    pub name: String,
    pub cash: u8,
    pub assets: Vec<Asset>,
    pub liabilities: Vec<Liability>,
    pub hand: Vec<Either<Asset, Liability>>,
}

impl ResultsPlayer {
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

impl From<RoundPlayer> for ResultsPlayer {
    fn from(player: RoundPlayer) -> Self {
        Self {
            id: player.id,
            name: player.name,
            cash: player.cash,
            assets: player.assets,
            liabilities: player.liabilities,
            hand: player.hand,
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
pub enum LiabilityType {
    #[serde(rename = "Trade Credit")]
    TradeCredit,
    #[serde(rename = "Bank Loan")]
    BankLoan,
    Bonds,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CardType {
    Asset,
    Liability,
}

pub trait GetPlayerInfo {
    fn info(&self) -> PlayerInfo;
}

impl<T> GetPlayerInfo for T
where
    for<'a> PlayerInfo: From<&'a T>,
{
    fn info(&self) -> PlayerInfo {
        PlayerInfo::from(self)
    }
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

impl PlayerInfo {
    fn hand(hand: &[Either<Asset, Liability>]) -> Vec<CardType> {
        hand.iter()
            .map(|e| match e {
                Either::Left(_) => CardType::Asset,
                Either::Right(_) => CardType::Liability,
            })
            .collect()
    }
}

impl Default for PlayerInfo {
    fn default() -> Self {
        Self {
            name: Default::default(),
            id: PlayerId(0),
            hand: Default::default(),
            assets: Default::default(),
            liabilities: Default::default(),
            cash: Default::default(),
            character: Default::default(),
        }
    }
}

impl From<&LobbyPlayer> for PlayerInfo {
    fn from(player: &LobbyPlayer) -> Self {
        Self {
            name: player.name.clone(),
            id: player.id,
            ..Default::default()
        }
    }
}

impl From<&SelectingCharactersPlayer> for PlayerInfo {
    fn from(player: &SelectingCharactersPlayer) -> Self {
        Self {
            name: player.name.clone(),
            id: player.id,
            hand: Self::hand(&player.hand),
            assets: player.assets.clone(),
            liabilities: player.liabilities.clone(),
            cash: player.cash,
            character: player.character,
        }
    }
}

impl From<&RoundPlayer> for PlayerInfo {
    fn from(player: &RoundPlayer) -> Self {
        Self {
            name: player.name.clone(),
            id: player.id,
            hand: Self::hand(&player.hand),
            assets: player.assets.clone(),
            liabilities: player.liabilities.clone(),
            cash: player.cash,
            character: Some(player.character),
        }
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

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Color {
    Red,
    Green,
    Purple,
    Yellow,
    Blue,
}

impl Color {
    pub const COLORS: [Color; 5] = [
        Self::Red,
        Self::Green,
        Self::Purple,
        Self::Yellow,
        Self::Blue,
    ];
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

    pub fn playable_assets(&self) -> PlayableAssets {
        match self {
            // TODO: fix for CEO in CEO branch
            Self::CEO => PlayableAssets::default(),
            Self::CSO => PlayableAssets {
                total: 2,
                red_cost: 1,
                green_cost: 1,
                purple_cost: 2,
                yellow_cost: 2,
                blue_cost: 2,
            },
            _ => PlayableAssets::default(),
        }
    }

    pub fn playable_liabilities(&self) -> u8 {
        match self {
            Self::CFO => 3,
            _ => 1,
        }
    }

    pub fn draws_n_cards(&self) -> u8 {
        // TODO: fix head rnd ability when ready
        match self {
            Self::HeadRnD => 3,
            _ => 3,
        }
    }

    pub fn can_redeem_liabilities(&self) -> bool {
        matches!(self, Self::CFO)
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct PlayableAssets {
    total: u8,
    red_cost: u8,
    green_cost: u8,
    purple_cost: u8,
    yellow_cost: u8,
    blue_cost: u8,
}

impl PlayableAssets {
    pub fn total(&self) -> u8 {
        self.total
    }

    pub fn color_cost(&self, color: Color) -> u8 {
        let cost = match color {
            Color::Red => self.red_cost,
            Color::Green => self.green_cost,
            Color::Purple => self.purple_cost,
            Color::Yellow => self.yellow_cost,
            Color::Blue => self.blue_cost,
        };

        debug_assert!(cost > 0);
        debug_assert_eq!(self.total % cost, 0);

        cost
    }
}

impl Default for PlayableAssets {
    fn default() -> Self {
        Self {
            total: 1,
            red_cost: 1,
            green_cost: 1,
            purple_cost: 1,
            yellow_cost: 1,
            blue_cost: 1,
        }
    }
}

#[derive(
    Debug, Copy, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use claim::*;
    use itertools::Itertools;

    fn asset(color: Color) -> Asset {
        Asset {
            color,
            title: "Asset".to_owned(),
            gold_value: 1,
            silver_value: 1,
            ability: None,
            image_front_url: Default::default(),
            image_back_url: Default::default(),
        }
    }

    fn liability(value: u8) -> Liability {
        Liability {
            value,
            rfr_type: LiabilityType::BankLoan,
            image_front_url: Default::default(),
            image_back_url: Default::default(),
        }
    }

    fn hand_asset(color: Color) -> Vec<Either<Asset, Liability>> {
        vec![Either::Left(asset(color))]
    }

    fn hand_liability(value: u8) -> Vec<Either<Asset, Liability>> {
        vec![Either::Right(liability(value))]
    }

    #[test]
    fn asset_bonus() {
        for character in Character::CHARACTERS {
            for color in Color::COLORS {
                // bit awkward: get color that's not the same as either the tested color or the
                // character color. This could be different to test for asset_bonus() values of 1
                let different_color = Color::COLORS
                    .into_iter()
                    .find(|c| color.ne(c) && Some(*c).ne(&character.color()))
                    .unwrap();
                let assets = vec![asset(color), asset(color), asset(different_color)];
                let selecting_player = SelectingCharactersPlayer {
                    id: Default::default(),
                    name: Default::default(),
                    assets,
                    liabilities: Default::default(),
                    cash: 100,
                    character: Some(character),
                    hand: Default::default(),
                };
                let round_player = RoundPlayer::try_from(selecting_player).unwrap();

                match character.color() {
                    Some(character_color) if character_color == color => {
                        assert_eq!(round_player.asset_bonus(), 2, "{character:?}")
                    }
                    Some(_) => assert_eq!(round_player.asset_bonus(), 0),
                    None => assert_eq!(round_player.asset_bonus(), 0),
                }
            }
        }
    }

    #[test]
    fn market_condition_bonus() {
        use MarketCondition::*;

        for character in Character::CHARACTERS {
            for condition in [Minus, Zero, Plus] {
                let selecting_player = SelectingCharactersPlayer {
                    id: Default::default(),
                    name: Default::default(),
                    assets: Default::default(),
                    liabilities: Default::default(),
                    cash: 100,
                    character: Some(character),
                    hand: Default::default(),
                };
                let round_player = RoundPlayer::try_from(selecting_player).unwrap();

                let mut market = Market {
                    title: Default::default(),
                    rfr: Default::default(),
                    mrp: Default::default(),
                    yellow: Zero,
                    blue: Zero,
                    green: Zero,
                    purple: Zero,
                    red: Zero,
                    image_front_url: Default::default(),
                    image_back_url: Default::default(),
                };

                match character.color() {
                    Some(Color::Red) => market.red = condition,
                    Some(Color::Green) => market.green = condition,
                    Some(Color::Yellow) => market.yellow = condition,
                    Some(Color::Purple) => market.purple = condition,
                    Some(Color::Blue) => market.blue = condition,
                    None => {
                        market.red = condition;
                        market.green = condition;
                        market.yellow = condition;
                        market.purple = condition;
                        market.blue = condition;
                    }
                }

                let bonus = match character.color() {
                    Some(color) => match market.color_condition(color) {
                        MarketCondition::Plus => 1,
                        MarketCondition::Zero => 0,
                        MarketCondition::Minus => -1,
                    },
                    None => 0,
                };

                assert_eq!(
                    round_player.market_condition_bonus(&market),
                    bonus,
                    "{character:?}, {condition:?}"
                );
            }
        }
    }

    #[test]
    fn playable_assets_default() {
        for character in Character::CHARACTERS
            .into_iter()
            .filter(|c| ![Character::CEO, Character::CSO].contains(c))
        {
            let selecting_player = SelectingCharactersPlayer {
                id: Default::default(),
                name: Default::default(),
                assets: Default::default(),
                liabilities: Default::default(),
                cash: 100,
                character: Some(character),
                hand: vec![],
            };
            let round_player = RoundPlayer::try_from(selecting_player).unwrap();

            // All permutations of any color followed by blue, yellow or purple
            std::iter::repeat_n(Color::COLORS, 2)
                .multi_cartesian_product()
                .map(|v| (v[0], v[1]))
                .for_each(|(c1, c2)| {
                    let mut player = round_player.clone();
                    player.hand = hand_asset(c1);
                    assert_ok!(player.play_card(0));

                    player.hand = hand_asset(c2);
                    assert_matches!(
                        player.play_card(0),
                        Err(PlayCardError::ExceedsMaximumAssets)
                    );
                });
        }
    }

    #[test]
    fn playable_assets_cso() {
        let selecting_player = SelectingCharactersPlayer {
            id: Default::default(),
            name: Default::default(),
            assets: Default::default(),
            liabilities: Default::default(),
            cash: 100,
            character: Some(Character::CSO),
            hand: vec![],
        };
        let round_player = RoundPlayer::try_from(selecting_player).unwrap();

        // All permutations of 3 red/green colors
        std::iter::repeat_n([Color::Red, Color::Green], 3)
            .multi_cartesian_product()
            .map(|v| ([v[0], v[1]], v[2]))
            .for_each(|(colors, extra)| {
                for c in colors {
                    let mut player = round_player.clone();
                    player.hand = hand_asset(c);
                    assert_ok!(player.play_card(0));

                    player.hand = hand_asset(c);
                    assert_ok!(player.play_card(0));

                    player.hand = hand_asset(extra);
                    assert_matches!(
                        player.play_card(0),
                        Err(PlayCardError::ExceedsMaximumAssets)
                    );
                }
            });

        // All permutations of any color followed by blue, yellow or purple
        std::iter::repeat_n(Color::COLORS, 2)
            .multi_cartesian_product()
            .map(|v| (v[0], v[1]))
            .filter(|(_, c2)| [Color::Blue, Color::Yellow, Color::Purple].contains(&c2))
            .for_each(|(c1, c2)| {
                let mut player = round_player.clone();
                player.hand = hand_asset(c1);
                assert_ok!(player.play_card(0));

                player.hand = hand_asset(c2);
                assert_matches!(
                    player.play_card(0),
                    Err(PlayCardError::ExceedsMaximumAssets)
                );
            });
    }

    #[test]
    fn issue_liabilities_cfo() {
        #[derive(Copy, Clone, Debug)]
        enum IR {
            Issue,
            Redeem,
        }

        std::iter::repeat_n([IR::Issue, IR::Redeem], 4)
            .multi_cartesian_product()
            .map(|v| ([v[0], v[1], v[2]], v[3]))
            .for_each(|(irs, extra)| {
                let liability_value = 10;

                let selecting_player = SelectingCharactersPlayer {
                    id: Default::default(),
                    name: Default::default(),
                    assets: Default::default(),
                    liabilities: vec![
                        liability(liability_value),
                        liability(liability_value),
                        liability(liability_value),
                    ],
                    cash: 100,
                    character: Some(Character::CFO),
                    hand: vec![
                        Either::Right(liability(liability_value)),
                        Either::Right(liability(liability_value)),
                        Either::Right(liability(liability_value)),
                    ],
                };
                let mut player = RoundPlayer::try_from(selecting_player).unwrap();

                for (i, ir) in irs.into_iter().enumerate() {
                    match ir {
                        IR::Issue => {
                            assert_matches!(
                                player.play_card(0),
                                Ok(Either::Right(l)) if l.value == liability_value,
                                "i: {i} => {ir:?}"
                            );
                        }
                        IR::Redeem => {
                            assert_ok!(player.redeem_liability(0));
                        }
                    }
                }

                match extra {
                    IR::Issue => {
                        player.hand = vec![];
                        assert_matches!(
                            player.play_card(0),
                            Err(PlayCardError::InvalidCardIndex(_))
                        );

                        player.hand = hand_liability(liability_value);
                        assert_matches!(
                            player.play_card(0),
                            Err(PlayCardError::ExceedsMaximumLiabilities)
                        );
                    }
                    IR::Redeem => {
                        player.liabilities = vec![liability(liability_value)];
                        assert_matches!(
                            player.redeem_liability(0),
                            Err(RedeemLiabilityError::ExceedsMaximumLiabilities)
                        );
                    }
                }
            });
    }

    #[test]
    fn issue_liabilities_default() {
        for character in Character::CHARACTERS
            .into_iter()
            .filter(|c| *c != Character::CFO)
        {
            let liability_value = 10;

            let selecting_player = SelectingCharactersPlayer {
                id: Default::default(),
                name: Default::default(),
                assets: Default::default(),
                liabilities: Default::default(),
                cash: 100,
                character: Some(character),
                hand: hand_liability(liability_value),
            };
            let mut player = RoundPlayer::try_from(selecting_player).unwrap();

            assert_matches!(
                player.play_card(0),
                Ok(Either::Right(l)) if l.value == liability_value
            );

            assert_matches!(player.play_card(0), Err(PlayCardError::InvalidCardIndex(_)));

            player.hand = hand_liability(liability_value);
            assert_matches!(
                player.play_card(0),
                Err(PlayCardError::ExceedsMaximumLiabilities)
            );

            assert_matches!(
                player.redeem_liability(0),
                Err(RedeemLiabilityError::NotAllowedToRedeemLiability(_))
            );
        }
    }
}
