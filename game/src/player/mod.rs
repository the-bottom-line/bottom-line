//! This file contains all four player states, as well as functionality for those players and ways
//! to interact with them.

mod banker_target;
mod lobby;
mod results;
mod round;
mod selecting_characters;

pub use banker_target::*;
pub use lobby::*;
pub use results::*;
pub use round::*;
pub use selecting_characters::*;

use either::Either;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ts")]
use ts_rs::TS;

use std::sync::Arc;

use crate::{errors::*, game::*};

/// Representation of an asset card. Each asset has a gold and a silver value, as well as an
/// associated color. Some cards alse have an [`AssetPowerup`].
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(rename = "AssetCard"))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    /// Title of the asset card.
    pub title: String,
    /// The gold value of the asset.
    pub gold_value: u8,
    /// The silver value of the asset.
    pub silver_value: u8,
    /// The color of the asset.
    pub color: Color,
    /// Whether or not this asset has an [`AssetPowerup`].
    pub ability: Option<AssetPowerup>,
    /// Url containing the relative location of the card in the assets folder
    pub image_front_url: String,
    /// Url containing the relative location of the back of the card in the assets folder
    pub image_back_url: Arc<String>,
}

impl Asset {
    /// Gets the current value of the asset based on the given market condition. Note that this
    /// value can be negative.
    pub fn market_value(&self, market: &Market) -> i8 {
        let mul: i8 = match market.color_condition(self.color) {
            MarketCondition::Plus => 1,
            MarketCondition::Minus => -1,
            MarketCondition::Zero => 0,
        };
        self.gold_value as i8 + self.silver_value as i8 * mul
    }

    /// Calculates what it costs to divest this asset based on the current market. Note that this
    /// number as zero at its lowest.
    pub fn divest_cost(&self, market: &Market) -> u8 {
        let mv = self.market_value(market);

        // match mv {
        //     ..=1 => 0,
        //     n => n as u8 - 1
        // }
        // mv.max(1) as u8 - 1
        if mv <= 1 { 0 } else { (mv - 1) as u8 }
    }
}

/// A certain powerup some assets have. These specify special actions this asset allows a player to
/// take at the end of the game.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum AssetPowerup {
    /// At the end of the game, for one color, turn that market color's - into 0 or 0 into +.
    #[serde(rename = "At the end of the game, for one color, turn - into 0 or 0 into +")]
    MinusIntoPlus,
    /// At the end of the game, turn one asset's silver value into gold.
    #[serde(rename = "At the end of the game, turn silver into gold on one asset card")]
    SilverIntoGold,
    /// At the end of the game, count one of your assets as any color.
    #[serde(rename = "At the end of the game, count one of your assets as any color")]
    CountAsAnyColor,
}

/// Representation of a liability card. Each liability has an associated gold value as well as a
/// [`LiabilityType`], which determines how expensive it is to issue this liability.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(rename = "LiabilityCard"))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Liability {
    /// Gold value of this liability
    pub value: u8,
    /// The card's [`LiabilityType`], which determines how expensive it is to issue this liability.
    pub rfr_type: LiabilityType,
    /// Url containing the relative location of the card in the assets folder.
    pub image_front_url: String,
    /// Url containing the relative location of the back of the card in the assets folder.
    pub image_back_url: Arc<String>,
}

impl Liability {
    /// Gets the associated rfr% for this liability. This can either be 1, 2 or 3.
    pub fn rfr_percentage(&self) -> u8 {
        match self.rfr_type {
            LiabilityType::TradeCredit => 1,
            LiabilityType::BankLoan => 2,
            LiabilityType::Bonds => 3,
        }
    }
}

/// The liability type determines the cost of lending for that particular liability.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LiabilityType {
    /// The cheapest type of liability.
    #[serde(rename = "Trade Credit")]
    TradeCredit,
    /// A slightly more expensive type of liability.
    #[serde(rename = "Bank Loan")]
    BankLoan,
    /// The most expensive type of liability.
    Bonds,
}

/// A card type used in relation to actions taken with player's hands. Can either be `Asset` or
/// `Liability`.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CardType {
    /// The [`Asset`] card type.
    Asset,
    /// The [`Liability`] card type.
    Liability,
}

/// Trait that should be implemented for each player type to be able to transform its internal data
/// into publicly displayable [`PlayerInfo`].
pub trait GetPlayerInfo {
    /// Gets the publicly available info of this particular player.
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

/// Publicly available information for each player. This contains information that you would be able
/// to see from another player if you were looking at what they have on the table. You cannot see
/// their hand, but you can see the amount of asset cards and liability cards they have.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    /// The name of the player.
    pub name: String,
    /// The id of the player.
    pub id: PlayerId,
    /// The hand of the player, represented as different [`CardType`]s.
    pub hand: Vec<CardType>,
    /// The assets this player has bought.
    pub assets: Vec<Asset>,
    /// The liabilities this player has issued.
    pub liabilities: Vec<Liability>,
    /// The amount of cash this player has.
    pub cash: u8,
    /// The character this player has chosen, if applicable.
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

/// Represtation of the colors associated with all assets as well as some selectable characters.
#[allow(missing_docs)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Color {
    Red,
    Green,
    Purple,
    Yellow,
    Blue,
}

impl Color {
    /// All available colors in this enum.
    pub const COLORS: [Color; 5] = [
        Self::Red,
        Self::Green,
        Self::Purple,
        Self::Yellow,
        Self::Blue,
    ];
}

/// Utility struct used to represent the amount of asset cards and liability cards a certain player
/// has.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegulatorSwapPlayer {
    /// The id of the particular player.
    pub player_id: PlayerId,
    /// The amount of asset cards this player has.
    pub asset_count: usize,
    /// The amount of liability cards this player has.
    pub liability_count: usize,
}

/// Utility struct used to represent each asset that can be divested from a player including the
/// cost of doing so.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DivestPlayer {
    /// The id of the particular player.
    pub player_id: PlayerId,
    /// The list of [`DivestAsset`]s for this player, which are all assets that can be divested
    /// from this player including the cost of doing so.
    pub assets: Vec<DivestAsset>,
}

/// Represents an asset that can be divested from a certain player including the cost of doing so.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DivestAsset {
    /// The asset in question.
    pub asset: Asset,
    /// The cost of divisting this asset based.
    pub divest_cost: u8,
    /// Whether or not this asset is divestable.
    pub is_divestable: bool,
}

/// An enum containing all characters currently in the game in the order in which they are called.
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(rename = "CharacterType"))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
pub enum Character {
    /// This character can fire any other character during their turn, excluding the
    /// [`Banker`](Character::Banker) and the [`Regulator`](Character::Regulator).
    Shareholder,
    /// This character can terminate the credit of any other character except for the
    /// [`Shareholder`](Character::Shareholder) and the [`Regulator`](Character::Regulator). This
    /// means that at the start of their turn they pay one gold to the banker plus one gold per
    /// unique color of assets they own.
    Banker,
    /// This character can choose to swap either their hand with the hand of any other player, or to
    /// swap any number of cards in their hand with random cards of the same type in the deck.
    Regulator,
    /// This character can buy up to three assets of any color during their turn.
    CEO,
    /// This character can issue up to three liabilities each turn. Alternatively, they can also
    /// choose to redeem liabilities, where they pay the liability's gold in cash to get it off
    /// their balance sheet.
    CFO,
    /// This character can buy up to two red or green assets.
    CSO,
    /// This character is allowed to draw six cards, giving back two.
    HeadRnD,
    /// This character can force any player except for the [`CSO`](Character::CSO) to divest one of
    /// their assets at market value minus one. This value cannot be negative and is paid for by
    /// this character.
    Stakeholder,
}

impl Character {
    /// A list of all characters in this enum in the order they are called.
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

    /// Gets the [`Color`] which some characters have associated with them.
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

    /// Gets the character who is called after this character
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

    /// Gets the character that is called first from a list of characters
    pub fn first(characters: &[Self]) -> Option<Self> {
        characters.iter().min().copied()
    }

    /// Gets this character's [`PlayableAssets`], which is a representation of how many assets of
    /// each color this character can buy this round.
    pub fn playable_assets(&self) -> PlayableAssets {
        match self {
            Self::CEO => PlayableAssets {
                total: 3,
                ..Default::default()
            },
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

    /// Gets the amount of liabilities this character can issue.
    pub fn playable_liabilities(&self) -> u8 {
        match self {
            Self::CFO => 3,
            _ => 1,
        }
    }

    /// Gets the amount of cards this character is allowed to draw.
    pub fn draws_n_cards(&self) -> u8 {
        // TODO: fix head rnd ability when ready
        match self {
            Self::HeadRnD => 6,
            _ => 3,
        }
    }

    /// Returns `true` if this character is allowed to redeem liabilities
    pub fn can_redeem_liabilities(&self) -> bool {
        matches!(self, Self::CFO)
    }

    /// Returns `true` if this character can be fired
    pub fn can_be_fired(&self) -> bool {
        matches!(
            self,
            Self::CEO | Self::CSO | Self::CFO | Self::HeadRnD | Self::Stakeholder
        )
    }

    /// Returns true if this character can be forced to divest.
    pub fn can_be_forced_to_divest(&self) -> bool {
        !matches!(self, Self::CSO)
    }
}

/// a representation of how many assets of each color a certain player is allowed to buy this round.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayableAssets {
    total: u8,
    red_cost: u8,
    green_cost: u8,
    purple_cost: u8,
    yellow_cost: u8,
    blue_cost: u8,
}

impl PlayableAssets {
    /// The total unit value of assets a player can buy
    pub fn total(&self) -> u8 {
        self.total
    }

    /// The unit cost of buying an asset of a certain color.
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

/// A wrapper around `u8` which represents a player's `id`.
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
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
