//! This file contains all four player states, as well as functionality for those players and ways
//! to interact with them.

use either::Either;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ts")]
use ts_rs::TS;

use std::sync::Arc;

use crate::{
    errors::*,
    game::{Deck, Market, MarketCondition},
};

/// The player type corresponding to the [`Lobby`](crate::game::Lobby) state of the game.
#[derive(Debug, Clone, PartialEq)]
pub struct LobbyPlayer {
    id: PlayerId,
    name: String,
}

impl LobbyPlayer {
    /// Instantiates a new lobby player based on an id and a name.
    pub fn new(id: PlayerId, name: String) -> Self {
        Self { id, name }
    }

    /// Gets the id of the player
    pub fn id(&self) -> PlayerId {
        self.id
    }

    /// Sets the id of the player
    pub fn set_id(&mut self, id: PlayerId) {
        self.id = id;
    }

    /// Gets the name of the player
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// The player type that corresponds to the
/// [`SelectingCharacters`](crate::game::SelectingCharacters) stage of the game. In this stage,
/// players may not have a selected character yet.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectingCharactersPlayer {
    id: PlayerId,
    name: String,
    cash: u8,
    assets: Vec<Asset>,
    liabilities: Vec<Liability>,
    character: Option<Character>,
    hand: Vec<Either<Asset, Liability>>,
}

impl SelectingCharactersPlayer {
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

    /// Gets the character for this player, if they've selected a chararacter yet
    pub fn character(&self) -> Option<Character> {
        self.character
    }

    /// Gets the hand of this player.
    pub fn hand(&self) -> &[Either<Asset, Liability>] {
        &self.hand
    }

    /// Constructs a new player id and a name, a certain hand and starting cash.
    pub(crate) fn new(
        name: String,
        id: PlayerId,
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
            id,
            name,
            cash,
            assets: vec![],
            liabilities: vec![],
            character: None,
            hand,
        }
    }

    /// Tries to select a character for the player. If the player has not selected a character yet,
    /// changes their character to `character`.
    pub fn select_character(
        &mut self,
        character: Character,
    ) -> Result<(), SelectingCharactersError> {
        match self.character {
            Some(c) => Err(SelectingCharactersError::AlreadySelectedCharacter(c)),
            None => {
                self.character = Some(character);
                Ok(())
            }
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

/// The player type that corresponds to the [`Round`](crate::game::Round) stage of the game. During
/// the round stage, each player has selected a character.
#[derive(Debug, Clone, PartialEq)]
pub struct RoundPlayer {
    id: PlayerId,
    name: String,
    cash: u8,
    assets: Vec<Asset>,
    liabilities: Vec<Liability>,
    character: Character,
    hand: Vec<Either<Asset, Liability>>,
    cards_drawn: Vec<usize>,
    bonus_draw_cards: u8,
    assets_to_play: u8,
    playable_assets: PlayableAssets,
    liabilities_to_play: u8,
    total_cards_drawn: u8,
    total_cards_given_back: u8,
    has_used_ability: bool,
}

impl RoundPlayer {
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

    // TODO: Temporarily used in tests, remove when tests update
    pub(crate) fn _set_cash(&mut self, cash: u8) {
        self.cash = cash;
    }

    /// Gets a list of bought assets of the player
    pub fn assets(&self) -> &[Asset] {
        &self.assets
    }

    /// Gets a list of issued liabilities of the player
    pub fn liabilities(&self) -> &[Liability] {
        &self.liabilities
    }

    /// Gets the character for this player
    pub fn character(&self) -> Character {
        self.character
    }

    /// Gets the hand of this player.
    pub fn hand(&self) -> &[Either<Asset, Liability>] {
        &self.hand
    }

    /// Adds a new `card_idx` to the list of cards drawn this round.
    fn update_cards_drawn(&mut self, card_idx: usize) {
        self.cards_drawn = self
            .cards_drawn
            .iter()
            .copied()
            .filter(|&i| i != card_idx)
            .collect();
    }

    /// Checks whether or not a player can play an asset of a certain color.
    fn can_play_asset(&self, color: Color) -> bool {
        self.assets_to_play
            .checked_sub(self.playable_assets.color_cost(color))
            .is_some()
    }

    /// Checks whether or not this player can still issue a liability.
    pub fn can_play_liability(&self) -> bool {
        self.liabilities_to_play > 0
    }

    /// Redeems a liability for a player by paying for it in cash. If succesful, returns the
    /// liability that was redeemed.
    pub(crate) fn redeem_liability(
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

    /// Tries to fire a character. If succesful, returns that character.
    pub fn fire_character(
        &mut self,
        character: Character,
    ) -> Result<Character, FireCharacterError> {
        if self.character == Character::Shareholder {
            if !self.has_used_ability {
                if character.can_be_fired() {
                    self.has_used_ability = true;
                    Ok(character)
                } else {
                    Err(FireCharacterError::InvalidCharacter)
                }
            } else {
                Err(FireCharacterError::AlreadyFiredThisTurn)
            }
        } else {
            Err(FireCharacterError::InvalidPlayerCharacter)
        }
    }

    /// Swaps a list of card indexes `card_idxs` with the deck. Each asset that is swapped is put
    /// back into the deck and replaced by drawing a new asset, and each liability that is swapped
    /// is put back into the liability deck and replaced by drawing a new liability. If succesful,
    /// returns the total number of cards swapped with the deck.
    pub fn swap_with_deck(
        &mut self,
        mut card_idxs: Vec<usize>,
        asset_deck: &mut Deck<Asset>,
        liability_deck: &mut Deck<Liability>,
    ) -> Result<Vec<usize>, SwapError> {
        if card_idxs.is_empty() {
            return Ok(vec![0, 0]);
        }

        if self.character == Character::Regulator {
            if !self.has_used_ability {
                card_idxs.sort();
                if card_idxs.last().copied().unwrap_or_default() <= self.hand.len()
                    && card_idxs.iter().all_unique()
                {
                    let removed_card_len = card_idxs.len();

                    // TODO: actually draw new cards for player?
                    let mut asset_count: usize = 0;
                    let mut liability_count: usize = 0;
                    for card in card_idxs.into_iter().rev() {
                        match self.hand.remove(card) {
                            Either::Left(a) => {
                                asset_deck.put_back(a);
                                asset_count += 1;
                            }
                            Either::Right(l) => {
                                liability_deck.put_back(l);
                                liability_count += 1;
                            }
                        }
                    }
                    self.has_used_ability = true;
                    self.bonus_draw_cards += removed_card_len as u8;
                    Ok(vec![asset_count, liability_count])
                } else {
                    Err(SwapError::InvalidCardIdxs)
                }
            } else {
                Err(SwapError::AlreadySwapedThisTurn)
            }
        } else {
            Err(SwapError::AlreadySwapedThisTurn)
        }
    }

    /// Tries to swap the hands of this player, if they are the
    /// [`Regulator`](Character::Regulator), with the hands of the target player.
    pub fn regulator_swap_with_player(
        &mut self,
        target: &mut RoundPlayer,
    ) -> Result<(), SwapError> {
        if self.character == Character::Regulator {
            if !self.has_used_ability {
                self.has_used_ability = true;
                std::mem::swap(&mut self.hand, &mut target.hand);
                Ok(())
            } else {
                Err(SwapError::AlreadySwapedThisTurn)
            }
        } else {
            Err(SwapError::AlreadySwapedThisTurn)
        }
    }

    /// Removes an asset from this player at index `asset_idx`. If succesful, returns the asset that
    /// was removed from the player.
    pub fn remove_asset(&mut self, asset_idx: usize) -> Result<Asset, DivestAssetError> {
        if self.assets.get(asset_idx).is_some() {
            Ok(self.assets.remove(asset_idx))
        } else {
            Err(DivestAssetError::InvalidCardIdx)
        }
    }

    /// Checks if this player can divest an asset at index `asset_idx` from the target player. If
    /// they can, the cost of doing so is returned.
    pub fn divest_asset(
        &mut self,
        player: &RoundPlayer,
        asset_idx: usize,
        market: &Market,
    ) -> Result<u8, DivestAssetError> {
        if self.character == Character::Stakeholder {
            if !self.has_used_ability {
                if player.character.can_be_forced_to_divest() {
                    if asset_idx < player.assets.len() {
                        let asset = &player.assets[asset_idx];
                        if asset.color != Color::Red && asset.color != Color::Green {
                            let cost = asset.divest_cost(market);
                            if cost <= self.cash {
                                self.has_used_ability = true;
                                self.cash -= cost;
                                Ok(cost)
                            } else {
                                Err(DivestAssetError::NotEnoughCash)
                            }
                        } else {
                            Err(DivestAssetError::CantDivestAssetType)
                        }
                    } else {
                        Err(DivestAssetError::InvalidCardIdx)
                    }
                } else {
                    Err(DivestAssetError::InvalidCharacter)
                }
            } else {
                Err(DivestAssetError::AlreadyDivestedThisTurn)
            }
        } else {
            Err(DivestAssetError::InvalidPlayerCharacter)
        }
    }

    /// Plays card in players hand with index `card_idx`. If that index is valid and they are
    /// allowed to play that card, it is returned.
    pub(crate) fn play_card(
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

    /// Makes the player draw a new card to their hand.
    fn draw_card(&mut self, card: Either<Asset, Liability>) {
        self.total_cards_drawn += 1;
        self.cards_drawn.push(self.hand.len());
        self.hand.push(card);
    }

    /// Draws a new asset from the deck, if they are allowed. If succesful, a reference to this
    /// asset is returned.
    pub(crate) fn draw_asset(&mut self, deck: &mut Deck<Asset>) -> Result<&Asset, DrawCardError> {
        if self.can_draw_cards() {
            let card = Either::Left(deck.draw());
            self.draw_card(card);

            Ok(self.hand.last().unwrap().as_ref().left().unwrap())
        } else {
            Err(DrawCardError::MaximumCardsDrawn(self.total_cards_drawn))
        }
    }

    /// Draws a new liability from the deck, if they are allowed. If succesful, a reference to this
    /// liability is returned.
    pub(crate) fn draw_liability(
        &mut self,
        deck: &mut Deck<Liability>,
    ) -> Result<&Liability, DrawCardError> {
        if self.can_draw_cards() {
            let card = Either::Right(deck.draw());
            self.draw_card(card);

            Ok(self.hand.last().unwrap().as_ref().right().unwrap())
        } else {
            Err(DrawCardError::MaximumCardsDrawn(self.total_cards_drawn))
        }
    }

    /// Makes this player give back a card from the cards they drew this round at index `card_idx`.
    /// If succesful, the card that was given back is returned.
    pub(crate) fn give_back_card(
        &mut self,
        card_idx: usize,
    ) -> Result<Either<Asset, Liability>, GiveBackCardError> {
        if self.should_give_back_cards() {
            match self.hand.get(card_idx) {
                Some(_) => {
                    self.total_cards_given_back += 1;
                    self.update_cards_drawn(card_idx);
                    Ok(self.hand.remove(card_idx))
                }
                None => Err(GiveBackCardError::InvalidCardIndex(card_idx as u8)),
            }
        } else {
            Err(GiveBackCardError::Unnecessary)
        }
    }

    /// Checks whether or not this player should still give back cards.
    pub fn should_give_back_cards(&self) -> bool {
        // For every 3 cards drawn one needs to give one back. Subtract any bonus drawing cards a
        // player may draw.
        match (self.total_cards_drawn.saturating_sub(self.bonus_draw_cards) / 3)
            .checked_sub(self.total_cards_given_back)
        {
            Some(v) => v > 0,
            None => false,
        }
    }

    /// Checks whether or not this player can still draw any more cards
    pub fn can_draw_cards(&self) -> bool {
        self.total_cards_drawn < self.draws_n_cards() + self.bonus_draw_cards
    }

    /// Gets the number of cards this player can draw in total
    pub fn draws_n_cards(&self) -> u8 {
        self.character.draws_n_cards()
    }

    /// Gets the number of cards this player should give back in total.
    pub fn gives_back_n_cards(&self) -> u8 {
        // Give back one card for every 3 drawn
        self.draws_n_cards() / 3
    }

    /// Gets this player's [`PlayableAssets`], which is a representation of how many assets of each
    /// color this player can buy this round.
    pub fn playable_assets(&self) -> PlayableAssets {
        self.playable_assets
    }

    /// Gets the amount of liabilities this player can play this round.
    pub fn playable_liabilities(&self) -> u8 {
        self.character.playable_liabilities()
    }

    /// Gets the amount of cash this player gets to start their turn.
    pub fn turn_start_cash(&self) -> i16 {
        1
    }

    /// Gets the amount of cash this player gets based on the character they chose and the assets
    /// they own.
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

    /// Gets the amount of cash this player gets based on the character they chose and the market
    /// condition of the color of that character.
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

    /// Gets the total amount of cash this player receives at the start of their turn.
    pub fn turn_cash(&self, current_market: &Market) -> u8 {
        let start = self.turn_start_cash();
        let asset_bonus = self.asset_bonus();
        let market_condition_bonus = self.market_condition_bonus(current_market);

        (start + asset_bonus * (market_condition_bonus + 1)) as u8
    }

    /// Starts this player's turn by givinig them their turn gold.
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
                    bonus_draw_cards: 0,
                    total_cards_given_back: 0,
                    has_used_ability: false,
                })
            }
            None => Err(GameError::PlayerMissingCharacter),
        }
    }
}

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
}

impl ResultsPlayer {
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
    pub fn score(&self, final_market: &Market) -> f64 {
        let gold = self.total_gold() as f64;
        let silver = self.total_silver() as f64;

        let trade_credit = self.trade_credit() as f64;
        let bank_loan = self.bank_loan() as f64;
        let bonds = self.bonds() as f64;
        let debt = trade_credit + bank_loan + bonds;

        let beta = silver / gold;

        // TODO: end of game bonuses
        let drp = (trade_credit + bank_loan * 2.0 + bonds * 3.0) / gold;

        let wacc = final_market.rfr as f64 + drp + beta * final_market.mrp as f64;

        let red = self.color_value(Color::Red, final_market);
        let green = self.color_value(Color::Green, final_market);
        let yellow = self.color_value(Color::Yellow, final_market);
        let purple = self.color_value(Color::Purple, final_market);
        let blue = self.color_value(Color::Blue, final_market);

        let fcf = red + green + yellow + purple + blue;

        (fcf / (10.0 * wacc)) + (debt / 3.0) + self.cash() as f64
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

impl std::fmt::Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\ngold: {}\nsilver: {}\ncolor: {:?}",
            self.title, self.gold_value, self.silver_value, self.color
        )
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
    fn select_character() {
        for character in Character::CHARACTERS {
            let mut player = SelectingCharactersPlayer {
                id: Default::default(),
                name: Default::default(),
                assets: Default::default(),
                liabilities: Default::default(),
                cash: Default::default(),
                character: None,
                hand: Default::default(),
            };

            assert_ok!(player.select_character(character));
            assert_eq!(player.character, Some(character));

            for character2 in Character::CHARACTERS {
                assert_matches!(
                    player.select_character(character2),
                    Err(SelectingCharactersError::AlreadySelectedCharacter(c)) if c == character
                );
            }
        }
    }

    #[test]
    fn draw_cards_head_rnd() {
        let liability_value = 10;

        let selecting_player = SelectingCharactersPlayer {
            id: Default::default(),
            name: Default::default(),
            assets: Default::default(),
            liabilities: Default::default(),
            cash: Default::default(),
            character: Some(Character::HeadRnD),
            hand: Default::default(),
        };
        let round_player = RoundPlayer::try_from(selecting_player).unwrap();

        std::iter::repeat_n([CardType::Asset, CardType::Liability], 7)
            .multi_cartesian_product()
            .map(|v| ([v[0], v[1], v[2], v[3], v[4], v[5]], v[6]))
            .for_each(|(types, extra)| {
                let mut player = round_player.clone();
                for t in types {
                    let hand_len = player.hand.len();
                    let total_cards_drawn = player.total_cards_drawn;
                    match t {
                        CardType::Asset => {
                            let mut assets = Deck::new(vec![asset(Color::Red)]);
                            let asset = assert_ok!(player.draw_asset(&mut assets)).clone();
                            let cmp = player.hand[*player.cards_drawn.last().unwrap()]
                                .as_ref()
                                .left()
                                .unwrap();
                            assert_eq!(&asset, cmp);
                        }
                        CardType::Liability => {
                            let mut liabilities = Deck::new(vec![liability(liability_value)]);
                            let liability =
                                assert_ok!(player.draw_liability(&mut liabilities)).clone();
                            let cmp = player.hand[*player.cards_drawn.last().unwrap()]
                                .as_ref()
                                .right()
                                .unwrap();
                            assert_eq!(&liability, cmp);
                        }
                    }
                    assert_eq!(hand_len, player.hand.len() - 1);
                    assert_eq!(total_cards_drawn, player.total_cards_drawn - 1);
                }

                let hand_len = player.hand.len();
                let cards_drawn = player.total_cards_drawn;
                match extra {
                    CardType::Asset => {
                        let mut assets = Deck::new(vec![asset(Color::Red)]);
                        assert_matches!(
                            player.draw_asset(&mut assets),
                            Err(DrawCardError::MaximumCardsDrawn(_))
                        );
                    }
                    CardType::Liability => {
                        let mut liabilities = Deck::new(vec![liability(liability_value)]);
                        assert_matches!(
                            player.draw_liability(&mut liabilities),
                            Err(DrawCardError::MaximumCardsDrawn(_))
                        );
                    }
                }
                assert_eq!(hand_len, player.hand.len());
                assert_eq!(cards_drawn, player.total_cards_drawn);
            });
    }

    #[test]
    fn draw_cards_default() {
        let liability_value = 10;

        for character in Character::CHARACTERS
            .into_iter()
            .filter(|c| *c != Character::HeadRnD)
        {
            let selecting_player = SelectingCharactersPlayer {
                id: Default::default(),
                name: Default::default(),
                assets: Default::default(),
                liabilities: Default::default(),
                cash: Default::default(),
                character: Some(character),
                hand: Default::default(),
            };
            let round_player = RoundPlayer::try_from(selecting_player).unwrap();

            std::iter::repeat_n([CardType::Asset, CardType::Liability], 4)
                .multi_cartesian_product()
                .map(|v| ([v[0], v[1], v[2]], v[3]))
                .for_each(|(types, extra)| {
                    let mut player = round_player.clone();
                    for t in types {
                        match t {
                            CardType::Asset => {
                                let mut assets = Deck::new(vec![asset(Color::Red)]);
                                let asset = assert_ok!(player.draw_asset(&mut assets)).clone();
                                let cmp = player.hand[*player.cards_drawn.last().unwrap()]
                                    .as_ref()
                                    .left()
                                    .unwrap();
                                assert_eq!(&asset, cmp);
                            }
                            CardType::Liability => {
                                let mut liabilities = Deck::new(vec![liability(liability_value)]);
                                let liability =
                                    assert_ok!(player.draw_liability(&mut liabilities)).clone();
                                let cmp = player.hand[*player.cards_drawn.last().unwrap()]
                                    .as_ref()
                                    .right()
                                    .unwrap();
                                assert_eq!(&liability, cmp);
                            }
                        }
                    }

                    let hand_len = player.hand.len();
                    let cards_drawn = player.total_cards_drawn;
                    match extra {
                        CardType::Asset => {
                            let mut assets = Deck::new(vec![asset(Color::Red)]);
                            assert_matches!(
                                player.draw_asset(&mut assets),
                                Err(DrawCardError::MaximumCardsDrawn(_))
                            );
                        }
                        CardType::Liability => {
                            let mut liabilities = Deck::new(vec![liability(liability_value)]);
                            assert_matches!(
                                player.draw_liability(&mut liabilities),
                                Err(DrawCardError::MaximumCardsDrawn(_))
                            );
                        }
                    }
                    assert_eq!(hand_len, player.hand.len());
                    assert_eq!(cards_drawn, player.total_cards_drawn);
                });
        }
    }

    #[test]
    fn fire_character_shareholder() {
        const CHARACTER: Character = Character::Shareholder;

        let selecting_player = SelectingCharactersPlayer {
            id: Default::default(),
            name: Default::default(),
            assets: Default::default(),
            liabilities: Default::default(),
            cash: Default::default(),
            character: Some(CHARACTER),
            hand: Default::default(),
        };
        let mut player = RoundPlayer::try_from(selecting_player).unwrap();

        //test firing unfireable characters
        assert_matches!(
            player.fire_character(Character::Shareholder),
            Err(FireCharacterError::InvalidCharacter)
        );
        assert_matches!(
            player.fire_character(Character::Banker),
            Err(FireCharacterError::InvalidCharacter)
        );
        assert_matches!(
            player.fire_character(Character::Regulator),
            Err(FireCharacterError::InvalidCharacter)
        );

        //test regular fire functionality
        assert_matches!(player.fire_character(Character::CEO), Ok(Character::CEO));

        //test already fired this round
        assert_matches!(
            player.fire_character(Character::CEO),
            Err(FireCharacterError::AlreadyFiredThisTurn)
        );
        assert_matches!(
            player.fire_character(Character::Stakeholder),
            Err(FireCharacterError::AlreadyFiredThisTurn)
        );
    }

    #[test]
    fn fire_character_not_shareholder() {
        for character in Character::CHARACTERS
            .into_iter()
            .filter(|c| *c != Character::Shareholder)
        {
            let selecting_player = SelectingCharactersPlayer {
                id: Default::default(),
                name: Default::default(),
                assets: Default::default(),
                liabilities: Default::default(),
                cash: Default::default(),
                character: Some(character),
                hand: Default::default(),
            };

            let mut player = RoundPlayer::try_from(selecting_player).unwrap();

            //test firing unfireable characters
            assert_matches!(
                player.fire_character(Character::CEO),
                Err(FireCharacterError::InvalidPlayerCharacter)
            );
        }
    }

    #[test]
    fn give_back_cards_head_rnd() {
        const CHARACTER: Character = Character::HeadRnD;

        let selecting_player = SelectingCharactersPlayer {
            id: Default::default(),
            name: Default::default(),
            assets: Default::default(),
            liabilities: Default::default(),
            cash: Default::default(),
            character: Some(CHARACTER),
            hand: Default::default(),
        };
        let mut player = RoundPlayer::try_from(selecting_player).unwrap();

        let asset_vec = std::iter::repeat_with(|| asset(Color::Blue))
            .take(6)
            .collect();
        let mut assets = Deck::new(asset_vec);
        for _ in 0..assets.len() {
            assert_ok!(player.draw_asset(&mut assets));
        }

        assert!(player.should_give_back_cards());
        assert_eq!(player.total_cards_given_back, 0);
        assert_eq!(CHARACTER.draws_n_cards(), player.hand.len() as u8);

        assert_err!(player.give_back_card(123));
        assert_eq!(player.total_cards_given_back, 0);
        assert_eq!(CHARACTER.draws_n_cards(), player.hand.len() as u8);

        assert_ok!(player.give_back_card(0));
        assert_eq!(player.total_cards_given_back, 1);
        assert_eq!(CHARACTER.draws_n_cards() - 1, player.hand.len() as u8);

        assert!(player.should_give_back_cards());

        assert_ok!(player.give_back_card(0));
        assert_eq!(player.total_cards_given_back, 2);
        assert_eq!(CHARACTER.draws_n_cards() - 2, player.hand.len() as u8);

        assert!(!player.should_give_back_cards());
        assert_err!(player.give_back_card(0));
        assert_eq!(player.total_cards_given_back, 2);
        assert_eq!(CHARACTER.draws_n_cards() - 2, player.hand.len() as u8);
    }

    #[test]
    fn give_back_cards_default() {
        for character in Character::CHARACTERS
            .into_iter()
            .filter(|c| *c != Character::HeadRnD)
        {
            let selecting_player = SelectingCharactersPlayer {
                id: Default::default(),
                name: Default::default(),
                assets: Default::default(),
                liabilities: Default::default(),
                cash: Default::default(),
                character: Some(character),
                hand: Default::default(),
            };
            let mut player = RoundPlayer::try_from(selecting_player).unwrap();

            let asset_vec = std::iter::repeat_with(|| asset(Color::Blue))
                .take(3)
                .collect();
            let mut assets = Deck::new(asset_vec);
            for _ in 0..assets.len() {
                assert_ok!(player.draw_asset(&mut assets));
            }

            assert!(player.should_give_back_cards());
            assert_eq!(player.total_cards_given_back, 0);
            assert_eq!(character.draws_n_cards(), player.hand.len() as u8);

            assert_err!(player.give_back_card(123));
            assert_eq!(player.total_cards_given_back, 0);
            assert_eq!(character.draws_n_cards(), player.hand.len() as u8);

            assert_ok!(player.give_back_card(0));
            assert_eq!(player.total_cards_given_back, 1);
            assert_eq!(character.draws_n_cards() - 1, player.hand.len() as u8);

            assert!(!player.should_give_back_cards());
            assert_err!(player.give_back_card(0));
            assert_eq!(player.total_cards_given_back, 1);
            assert_eq!(character.draws_n_cards() - 1, player.hand.len() as u8);
        }
    }

    #[test]
    fn should_give_back_cards() {
        let selecting_player = SelectingCharactersPlayer {
            id: Default::default(),
            name: Default::default(),
            assets: Default::default(),
            liabilities: Default::default(),
            cash: Default::default(),
            character: Some(Character::HeadRnD),
            hand: Default::default(),
        };
        let mut round_player = RoundPlayer::try_from(selecting_player).unwrap();

        for total_cards_drawn in 0..100u8 {
            for total_cards_given_back in 0..33u8 {
                let cmp = match (total_cards_drawn / 3).checked_sub(total_cards_given_back) {
                    Some(v) => v > 0,
                    None => false,
                };
                round_player.total_cards_drawn = total_cards_drawn;
                round_player.total_cards_given_back = total_cards_given_back;
                assert_eq!(round_player.should_give_back_cards(), cmp);
            }
        }
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
        const STARTING_CASH: u8 = 100;

        for character in Character::CHARACTERS
            .into_iter()
            .filter(|c| ![Character::CEO, Character::CSO].contains(c))
        {
            let selecting_player = SelectingCharactersPlayer {
                id: Default::default(),
                name: Default::default(),
                assets: Default::default(),
                liabilities: Default::default(),
                cash: STARTING_CASH,
                character: Some(character),
                hand: vec![],
            };
            let round_player = RoundPlayer::try_from(selecting_player).unwrap();

            // All permutations of any 2 colors
            std::iter::repeat_n(Color::COLORS, 2)
                .multi_cartesian_product()
                .map(|v| (v[0], v[1]))
                .for_each(|(c1, c2)| {
                    let mut player = round_player.clone();
                    let cash = player.cash;

                    player.hand = hand_asset(c1);
                    assert_ok!(player.play_card(0));

                    assert_eq!(player.cash, cash - 1);
                    assert_eq!(player.hand.len(), 0);
                    assert_eq!(player.assets.len(), 1);

                    assert!(!player.can_play_asset(c2));

                    player.hand = hand_asset(c2);
                    assert_matches!(
                        player.play_card(0),
                        Err(PlayCardError::ExceedsMaximumAssets)
                    );
                    assert_eq!(player.cash, cash - 1);
                    assert_eq!(player.hand.len(), 1);
                    assert_eq!(player.assets.len(), 1);
                });
        }
    }

    #[test]
    fn playable_assets_ceo() {
        const STARTING_CASH: u8 = 100;

        let selecting_player = SelectingCharactersPlayer {
            id: Default::default(),
            name: Default::default(),
            assets: Default::default(),
            liabilities: Default::default(),
            cash: 100,
            character: Some(Character::CEO),
            hand: vec![],
        };
        let round_player = RoundPlayer::try_from(selecting_player).unwrap();

        // All permutations of 4 colors
        std::iter::repeat_n(Color::COLORS, 4)
            .multi_cartesian_product()
            .map(|v| ([v[0], v[1], v[2]], v[3]))
            .for_each(|(colors, extra)| {
                let mut player = round_player.clone();

                for (i, c) in colors.into_iter().enumerate() {
                    player.hand = hand_asset(c);
                    assert_ok!(player.play_card(0), "bought assets: {i}");
                    assert_eq!(player.assets.len(), i + 1);
                    assert_eq!(player.cash, STARTING_CASH - 1 - i as u8);
                }

                assert!(!player.can_play_asset(extra));

                player.hand = hand_asset(extra);
                assert_matches!(
                    player.play_card(0),
                    Err(PlayCardError::ExceedsMaximumAssets)
                );
                assert_eq!(player.assets.len(), 3);
                assert_eq!(player.cash, STARTING_CASH - 3);
            });
    }

    #[test]
    fn playable_assets_cso() {
        const STARTING_CASH: u8 = 100;

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
                let mut player = round_player.clone();

                for (i, c) in colors.into_iter().enumerate() {
                    player.hand = hand_asset(c);
                    assert_ok!(player.play_card(0));
                    assert_eq!(player.assets.len(), i + 1);
                    assert_eq!(player.cash, STARTING_CASH - 1 - i as u8);
                }

                player.hand = hand_asset(extra);
                assert_matches!(
                    player.play_card(0),
                    Err(PlayCardError::ExceedsMaximumAssets)
                );
                assert_eq!(player.assets.len(), 2);
                assert_eq!(player.cash, STARTING_CASH - 2);
            });

        // All permutations of any color followed by blue, yellow or purple
        std::iter::repeat_n(Color::COLORS, 2)
            .multi_cartesian_product()
            .map(|v| (v[0], v[1]))
            .filter(|(_, c2)| [Color::Blue, Color::Yellow, Color::Purple].contains(c2))
            .for_each(|(c1, c2)| {
                let mut player = round_player.clone();
                player.hand = hand_asset(c1);
                assert_ok!(player.play_card(0));
                assert_eq!(player.assets.len(), 1);
                assert_eq!(player.cash, STARTING_CASH - 1);

                player.hand = hand_asset(c2);
                assert_matches!(
                    player.play_card(0),
                    Err(PlayCardError::ExceedsMaximumAssets)
                );
                assert_eq!(player.assets.len(), 1);
                assert_eq!(player.cash, STARTING_CASH - 1);
            });
    }

    #[test]
    fn issue_liabilities_cfo() {
        const LIABILITY_VALUE: u8 = 10;

        #[derive(Copy, Clone, Debug)]
        enum IR {
            Issue,
            Redeem,
        }

        std::iter::repeat_n([IR::Issue, IR::Redeem], 4)
            .multi_cartesian_product()
            .map(|v| ([v[0], v[1], v[2]], v[3]))
            .for_each(|(irs, extra)| {
                let selecting_player = SelectingCharactersPlayer {
                    id: Default::default(),
                    name: Default::default(),
                    assets: Default::default(),
                    liabilities: vec![
                        liability(LIABILITY_VALUE),
                        liability(LIABILITY_VALUE),
                        liability(LIABILITY_VALUE),
                    ],
                    cash: 100,
                    character: Some(Character::CFO),
                    hand: vec![
                        Either::Right(liability(LIABILITY_VALUE)),
                        Either::Right(liability(LIABILITY_VALUE)),
                        Either::Right(liability(LIABILITY_VALUE)),
                    ],
                };
                let mut player = RoundPlayer::try_from(selecting_player).unwrap();

                for ir in irs {
                    let player_cash = player.cash;
                    let hand_len = player.hand.len();
                    let liabilities_len = player.liabilities.len();
                    match ir {
                        IR::Issue => {
                            let liability = assert_ok!(player.play_card(0)).right().unwrap();
                            assert_eq!(liability.value, LIABILITY_VALUE);
                            assert_eq!(player.cash, player_cash + LIABILITY_VALUE);
                            assert_eq!(player.hand.len(), hand_len - 1);
                            assert_eq!(player.liabilities.len(), liabilities_len + 1);
                        }
                        IR::Redeem => {
                            let liability = assert_ok!(player.redeem_liability(0));
                            assert_eq!(liability.value, LIABILITY_VALUE);
                            assert_eq!(player.cash, player_cash - LIABILITY_VALUE);
                            assert_eq!(player.liabilities.len(), liabilities_len - 1);
                        }
                    }
                }

                match extra {
                    IR::Issue => {
                        let player_cash = player.cash;
                        player.hand = vec![];
                        assert_matches!(
                            player.play_card(0),
                            Err(PlayCardError::InvalidCardIndex(_))
                        );
                        assert_eq!(player.cash, player_cash);

                        player.hand = hand_liability(LIABILITY_VALUE);
                        assert_matches!(
                            player.play_card(0),
                            Err(PlayCardError::ExceedsMaximumLiabilities)
                        );
                        assert_eq!(player.cash, player_cash);
                    }
                    IR::Redeem => {
                        player.liabilities = vec![liability(LIABILITY_VALUE)];
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
        const LIABILITY_VALUE: u8 = 10;

        for character in Character::CHARACTERS
            .into_iter()
            .filter(|c| *c != Character::CFO)
        {
            let selecting_player = SelectingCharactersPlayer {
                id: Default::default(),
                name: Default::default(),
                assets: Default::default(),
                liabilities: Default::default(),
                cash: 100,
                character: Some(character),
                hand: hand_liability(LIABILITY_VALUE),
            };
            let mut player = RoundPlayer::try_from(selecting_player).unwrap();

            let player_cash = player.cash;
            let hand_len = player.hand.len();
            let liabilities_len = player.liabilities.len();

            let liability = assert_ok!(player.play_card(0)).right().unwrap();

            assert_eq!(liability.value, LIABILITY_VALUE);
            assert_eq!(player.cash, player_cash + LIABILITY_VALUE);
            assert_eq!(player.hand.len(), hand_len - 1);
            assert_eq!(player.liabilities.len(), liabilities_len + 1);

            let player_cash = player.cash;

            assert_matches!(player.play_card(0), Err(PlayCardError::InvalidCardIndex(_)));
            assert_eq!(player.cash, player_cash);

            player.hand = hand_liability(LIABILITY_VALUE);
            assert_matches!(
                player.play_card(0),
                Err(PlayCardError::ExceedsMaximumLiabilities)
            );
            assert_eq!(player.cash, player_cash);

            assert_matches!(
                player.redeem_liability(0),
                Err(RedeemLiabilityError::NotAllowedToRedeemLiability(_))
            );
            assert_eq!(player.cash, player_cash);
        }
    }
}
