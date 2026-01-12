//! This file contains the implementation of [`RoundPlayer`].

use either::Either;
use itertools::Itertools;

use crate::{errors::*, game::*, player::*};

/// The player type that corresponds to the [`Round`](crate::game::Round) stage of the game. During
/// the round stage, each player has selected a character.
#[derive(Debug, Clone, PartialEq)]
pub struct RoundPlayer {
    pub(super) id: PlayerId,
    pub(super) name: String,
    pub(super) cash: u8,
    pub(super) assets: Vec<Asset>,
    pub(super) liabilities: Vec<Liability>,
    pub(super) character: Character,
    pub(super) hand: Vec<Either<Asset, Liability>>,
    pub(super) cards_drawn: Vec<usize>,
    pub(super) bonus_draw_cards: u8,
    pub(super) assets_to_play: u8,
    pub(super) playable_assets: PlayableAssets,
    pub(super) liabilities_to_play: u8,
    pub(super) total_cards_drawn: u8,
    pub(super) total_cards_given_back: u8,
    pub(super) has_used_ability: bool,
    pub(super) has_gotten_bonus_cash: bool,
    pub(super) was_first_to_six_assets: bool,
    pub(super) is_human: bool,
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

    /// The first player to get six assets gets a cash bonus of 2.
    pub(crate) fn enable_first_to_six_assets_bonus(&mut self) {
        self.was_first_to_six_assets = true;
    }

    /// Gets the human state of this player
    pub fn is_human(&self) -> bool {
        self.is_human
    }

    /// Sets the human state of this player
    pub fn set_is_human(&mut self, human : bool) {
        self.is_human = human;
    }

    /// Returns true if the player has used their ability already
    pub fn has_used_ability(&self) -> bool {
        self.has_used_ability
    }

    /// Returns the amount of cards already drawn by the player
    pub fn total_cards_drawn(&self) -> u8 {
        self.total_cards_drawn
    }

    /// Returns the amount of cards already given back by the player
    pub fn total_cards_given_back(&self) -> u8 {
        self.total_cards_given_back
    }

    /// Returns the list of drawn cards
    pub fn cards_drawn(&self) -> &[usize] {
        &self.cards_drawn
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

    fn can_afford_asset(&self, asset: &Asset) -> bool {
        self.cash >= asset.gold_value
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

    /// Returns the budget for assets this player can still play.
    pub fn assets_to_play(&self) -> u8 {
        self.assets_to_play
    }

    /// Returns the number of liabilities this player can still issue.
    pub fn liabilities_to_play(&self) -> u8 {
        self.liabilities_to_play
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

    /// Tries to terminate credit line of character. If succesful, returns that character.
    pub fn terminate_credit(
        &mut self,
        character: Character,
    ) -> Result<Character, TerminateCreditCharacterError> {
        if self.character == Character::Banker {
            if !self.has_used_ability {
                if character.can_be_fired() {
                    // list of firable characters is the same for the banker
                    self.has_used_ability = true;
                    Ok(character)
                } else {
                    Err(TerminateCreditCharacterError::InvalidCharacter)
                }
            } else {
                Err(TerminateCreditCharacterError::AlreadyFiredThisTurn)
            }
        } else {
            Err(TerminateCreditCharacterError::InvalidPlayerCharacter)
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
                    for card_idx in card_idxs.into_iter().rev() {
                        // PANIC: we know each card_idx to be a valid index, so removing them cannot
                        // crash. Clarification: Sorting puts the highest index last, and we check
                        // if the last index is within the bounds of the player's hand.
                        match self.hand.remove(card_idx) {
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
            // PANIC: We verified that asset_idx is a valid index, so this cannot crash.
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
                Either::Left(a) if self.can_play_asset(a.color) && self.can_afford_asset(a) => {
                    // PANIC: self.hand[card_idx] exists and has been verified to be an asset, so
                    // this is safe to unwrap
                    let asset = self.hand.remove(card_idx).left().unwrap();
                    self.cash -= asset.gold_value;
                    self.assets_to_play -= self.playable_assets.color_cost(asset.color);
                    self.assets.push(asset.clone());
                    self.update_cards_drawn(card_idx);
                    Ok(Either::Left(asset))
                }
                Either::Left(a) if !self.can_play_asset(a.color) => Err(ExceedsMaximumAssets),
                Either::Left(a) if !self.can_afford_asset(a) => Err(CannotAffordAsset {
                    cash: self.cash,
                    cost: a.gold_value,
                }),
                Either::Right(_) if self.can_play_liability() => {
                    // PANIC: self.hand[card_idx] exists and has been verified to be a liability, so
                    // this is safe to unwrap
                    let liability = self.hand.remove(card_idx).right().unwrap();
                    self.cash += liability.value;
                    self.liabilities_to_play -= 1;
                    self.liabilities.push(liability.clone());
                    self.update_cards_drawn(card_idx);
                    Ok(Either::Right(liability))
                }
                Either::Right(_) if !self.can_play_liability() => Err(ExceedsMaximumLiabilities),
                _ => {
                    // PANIC: the compiler cannot verify that all cases are covered, but we can:
                    // Left() if we can both play and buy asset is checked,
                    // Left() if we can either not play or not buy asset is checked
                    // -- this covers all possible paths when it comes to the Left path
                    // Right if we can play liability is checked
                    // Right if we can't play liability is checked
                    // -- again we have full coverage of the Right path, so this is safe.
                    unreachable!()
                }
            }
        } else {
            Err(InvalidCardIndex(card_idx as u8))
        }
    }

    /// Makes the player draw a new card to their hand.
    fn draw_card(&mut self, card: Either<Asset, Liability>) -> Either<&Asset, &Liability> {
        self.total_cards_drawn += 1;
        self.cards_drawn.push(self.hand.len());
        self.hand.push(card);
        // PANIC: because we just pushed to the hand, we know this to be safe.
        self.hand.last().unwrap().as_ref()
    }

    /// Draws a new asset from the deck, if they are allowed. If succesful, a reference to this
    /// asset is returned.
    pub(crate) fn draw_asset(&mut self, deck: &mut Deck<Asset>) -> Result<&Asset, DrawCardError> {
        if self.can_draw_cards() {
            let asset = Either::Left(deck.draw());
            let card = self.draw_card(asset);

            // PANIC: because we just drew an asset, we know this to be safe.
            Ok(card.left().unwrap())
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
            let liability = Either::Right(deck.draw());
            let card = self.draw_card(liability);

            // PANIC: because we just drew a liability, we know this to be safe.
            Ok(card.right().unwrap())
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
                    // PANIC: we just verified that there is a card at this index, so removing it
                    // cannot crash.
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
    pub fn turn_start_cash(&self) -> u8 {
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
    pub fn turn_cash(&self) -> u8 {
        self.turn_start_cash()
    }

    /// Get bonus gold a player can get on their turn based on their characters color and their bought assets
    pub fn get_bonus_cash_character(
        &mut self,
        current_market: &Market,
    ) -> Result<u8, GetBonusCashError> {
        if self.has_gotten_bonus_cash {
            return Err(GetBonusCashError::AlreadyGottenBonusCashThisTurn);
        }
        if self.character.color().is_none() {
            return Err(GetBonusCashError::InvalidCharacter);
        }
        let asset_bonus = self.asset_bonus();
        let market_condition_bonus = self.market_condition_bonus(current_market);
        let bonus_cash = asset_bonus + market_condition_bonus;
        if bonus_cash < 0 {
            self.has_gotten_bonus_cash = true;
            Ok(0)
        } else {
            self.has_gotten_bonus_cash = true;
            self.cash += bonus_cash as u8;
            Ok(bonus_cash as u8)
        }
    }

    /// Starts this player's turn by givinig them their turn gold.
    pub(crate) fn start_turn(&mut self) {
        self.cash += self.turn_cash();
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
                    has_gotten_bonus_cash: false,
                    was_first_to_six_assets: false,
                    is_human: player.is_human
                })
            }
            None => Err(GameError::PlayerMissingCharacter),
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
            is_human : player.is_human,
        }
    }
}

impl From<&RoundPlayer> for BankerTargetPlayer {
    fn from(player: &RoundPlayer) -> Self {
        Self {
            id: player.id(),
            name: player.name().into(),
            cash: player.cash(),
            assets: player.assets.clone(),
            liabilities: player.liabilities.clone(),
            character: player.character(),
            hand: player.hand.clone(),
            liabilities_to_play: player.liabilities_to_play,
            was_first_to_six_assets: player.was_first_to_six_assets,
        }
    }
}

impl From<&BankerTargetPlayer> for RoundPlayer {
    fn from(player: &BankerTargetPlayer) -> Self {
        let playable_assets = player.character.playable_assets();
        Self {
            id: player.id(),
            name: player.name().into(),
            cash: player.cash,
            assets: player.assets.clone(),
            liabilities: player.liabilities.clone(),
            character: player.character,
            hand: player.hand.clone(),
            cards_drawn: vec![],
            bonus_draw_cards: 0,
            assets_to_play: playable_assets.total(),
            playable_assets,
            liabilities_to_play: player.liabilities_to_play,
            total_cards_drawn: 0,
            total_cards_given_back: 0,
            has_used_ability: false,
            has_gotten_bonus_cash: false,
            was_first_to_six_assets: player.was_first_to_six_assets,
        }
    }
}
#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use claim::*;
    use itertools::Itertools;

    pub(crate) fn asset(color: Color) -> Asset {
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

    pub(crate) fn liability(value: u8) -> Liability {
        Liability {
            value,
            rfr_type: LiabilityType::BankLoan,
            image_front_url: Default::default(),
            image_back_url: Default::default(),
        }
    }

    pub(crate) fn hand_asset(color: Color) -> Vec<Either<Asset, Liability>> {
        vec![Either::Left(asset(color))]
    }

    pub(crate) fn hand_liability(value: u8) -> Vec<Either<Asset, Liability>> {
        vec![Either::Right(liability(value))]
    }

    fn selecting_characters_player(
        character: Option<Character>,
        cash: u8,
    ) -> SelectingCharactersPlayer {
        SelectingCharactersPlayer {
            id: Default::default(),
            name: Default::default(),
            assets: Default::default(),
            liabilities: Default::default(),
            cash,
            character,
            hand: Default::default(),
            is_human: Default::default(),
        }
    }

    fn round_player(character: Character, cash: u8) -> RoundPlayer {
        selecting_characters_player(Some(character), cash)
            .try_into()
            .unwrap()
    }

    #[test]
    fn select_character() {
        for character in Character::CHARACTERS {
            let mut player = selecting_characters_player(None, 0);

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

        let round_player = round_player(Character::HeadRnD, 0);

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
            let round_player = round_player(character, 0);

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

        let mut player = round_player(CHARACTER, 0);

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
    fn get_bonus_cash_colored_characters() {
        let market_plus = Market {
            title: "".into(),
            rfr: 0,
            mrp: 0,
            blue: MarketCondition::Plus,
            green: MarketCondition::Plus,
            purple: MarketCondition::Plus,
            red: MarketCondition::Plus,
            yellow: MarketCondition::Plus,
        };

        let market_minus = Market {
            title: "".into(),
            rfr: 0,
            mrp: 0,
            blue: MarketCondition::Minus,
            green: MarketCondition::Minus,
            purple: MarketCondition::Minus,
            red: MarketCondition::Minus,
            yellow: MarketCondition::Minus,
        };

        let market = Market {
            title: "".into(),
            rfr: 0,
            mrp: 0,
            blue: MarketCondition::Zero,
            green: MarketCondition::Zero,
            purple: MarketCondition::Zero,
            red: MarketCondition::Zero,
            yellow: MarketCondition::Zero,
        };
        for character in Character::CHARACTERS.into_iter().filter(|c| {
            *c != Character::Shareholder && *c != Character::Banker && *c != Character::Regulator
        }) {
            let mut player = round_player(character, 0);
            // basic test with a neutral market and no player assets
            assert_matches!(player.get_bonus_cash_character(&market), Ok(0));

            player = round_player(character, 0);
            // Test with a Positive market and no player assets
            assert_matches!(player.get_bonus_cash_character(&market_plus), Ok(1));

            player = round_player(character, 0);
            // Test with a Negative market and no player assets
            assert_matches!(player.get_bonus_cash_character(&market_minus), Ok(0));

            player = round_player(character, 0);
            // add an asset of characters color to player
            if let Some(c) = character.color() {
                player.assets.push(asset(c));
            }
            // test 1 colored asset and neutral market
            assert_matches!(player.get_bonus_cash_character(&market), Ok(1));

            player = round_player(character, 0);
            // add an asset of characters color to player
            if let Some(c) = character.color() {
                player.assets.push(asset(c));
            }
            // test 1 colored asset and positive market
            assert_matches!(player.get_bonus_cash_character(&market_plus), Ok(2));

            player = round_player(character, 0);
            // add an asset of characters color to player
            if let Some(c) = character.color() {
                player.assets.push(asset(c));
            }
            // Test 1 colored asset and negative market
            assert_matches!(player.get_bonus_cash_character(&market_minus), Ok(0));
        }
    }

    #[test]
    fn fire_character_not_shareholder() {
        for character in Character::CHARACTERS
            .into_iter()
            .filter(|c| *c != Character::Shareholder)
        {
            let mut player = round_player(character, 0);

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

        let mut player = round_player(CHARACTER, 0);

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
            let mut player = round_player(character, 0);

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
        let mut round_player = round_player(Character::HeadRnD, 0);

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
                let mut round_player = round_player(character, 100);
                round_player.assets = vec![asset(color), asset(color), asset(different_color)];

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
                let round_player = round_player(character, 100);

                let mut market = Market::default();

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
            let round_player = round_player(character, STARTING_CASH);

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

        let round_player = round_player(Character::CEO, STARTING_CASH);

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

        let round_player = round_player(Character::CSO, STARTING_CASH);

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
                    is_human: Default::default(),
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
            let mut player = round_player(character, 100);
            player.hand = hand_liability(LIABILITY_VALUE);

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
