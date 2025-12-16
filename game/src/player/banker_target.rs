//! This file contains the implementation of [`BankerTargetPlayer`].

use either::Either;
use itertools::Itertools;

use crate::{
    errors::*,
    game::{self, Deck, Market, MarketCondition},
    player::*,
};

#[derive(Debug, Clone, PartialEq)]
pub struct BankerTargetPlayer {
    pub(super) id: PlayerId,
    pub(super) name: String,
    pub(super) cash: u8,
    pub(super) assets: Vec<Asset>,
    pub(super) liabilities: Vec<Liability>,
    pub(super) character: Character,
    pub(super) hand: Vec<Either<Asset, Liability>>,
    pub(super) liabilities_to_play: u8,
}

impl BankerTargetPlayer {
    /// Gets the id for this player
    pub fn id(&self) -> PlayerId {
        self.id
    }

    /// Gets the name of the player
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets the character for this player
    pub fn character(&self) -> Character {
        self.character
    }

    /// Pays the banker in the round the requested amount of gold
    pub fn pay_banker(
        &mut self,
        cash: u8,
        selected_cards: &SelectedAssetsAndLiabilities,
        banker: &mut BankerTargetPlayer,
    ) -> Result<PayBankerPlayer, PayBankerError> {
        let extra_asset_cash: u8 = selected_cards.assets.iter().map(|a| a.1).sum();
        let extra_liability_cash: u8 = selected_cards.liabilities.iter().map(|l| l.1).sum();
        if self.cash + extra_asset_cash + extra_liability_cash >= cash {
            banker.cash += cash;
            self.cash += extra_asset_cash + extra_liability_cash;
            self.cash -= cash;

            let mut asset_ids: Vec<usize> = selected_cards.assets.iter().map(|a| *a.0).collect();
            asset_ids.sort();
            for id in asset_ids.iter().rev() {
                self.assets.remove(*id);
            }

            let mut liability_ids: Vec<usize> =
                selected_cards.liabilities.iter().map(|l| *l.0).collect();
            liability_ids.sort();
            for id in liability_ids.iter().rev() {
                self.hand.remove(*id);
                self.liabilities_to_play -= 1;
            }

            Ok(PayBankerPlayer {
                new_banker_cash: banker.cash,
                new_target_cash: self.cash,
                target_id: self.id,
                banker_id: banker.id,
                selected_cards: selected_cards.clone(),
            })
        } else {
            Err(PayBankerError::NotEnoughCash)
        }
    }

    /// Select an asset to divest later when paying the banker
    pub fn select_divest_asset(
        &mut self,
        asset_id: usize,
        market: &Market,
        mut selected: SelectedAssetsAndLiabilities,
    ) -> Result<SelectedAssetsAndLiabilities, BankerTargetSelectError> {
        if let Some(asset) = self.assets.get(asset_id) {
            if !selected.assets.contains_key(&asset_id) {
                if asset.market_value(market) <= 0 {
                    let v = asset.market_value(market);
                    selected.assets.insert(asset_id, v as u8);
                    Ok(selected.clone())
                } else {
                    Err(BankerTargetSelectError::AssetValueToLow)
                }
            } else {
                Err(BankerTargetSelectError::AssetAlreadySelected)
            }
        } else {
            Err(BankerTargetSelectError::InvalidAssetId)
        }
    }

    /// Unselect an asset to remove it from divest asset list when paying the banker
    pub fn unselect_divest_asset(
        &mut self,
        asset_id: usize,
        mut selected: SelectedAssetsAndLiabilities,
    ) -> Result<SelectedAssetsAndLiabilities, BankerTargetSelectError> {
        if self.assets.get(asset_id).is_some() {
            if selected.assets.contains_key(&asset_id) {
                selected.assets.remove(&asset_id);
                Ok(selected.clone())
            } else {
                Err(BankerTargetSelectError::AssetNotSelected)
            }
        } else {
            Err(BankerTargetSelectError::InvalidAssetId)
        }
    }

    /// Select an liability to add it to the issue liability list when paying the banker
    pub fn select_issue_iability(
        &mut self,
        liability_id: usize,
        mut selected: SelectedAssetsAndLiabilities,
    ) -> Result<SelectedAssetsAndLiabilities, BankerTargetSelectError> {
        if self.character == Character::CFO {
            if let Some(liability) = self.hand.get(liability_id) {
                if !selected.assets.contains_key(&liability_id) {
                    if let Some(l) = liability.clone().right() {
                        selected.liabilities.insert(liability_id, l.value);
                        Ok(selected.clone())
                    } else {
                        Err(BankerTargetSelectError::InvalidLiabilityId)
                    }
                } else {
                    Err(BankerTargetSelectError::LiabilityAlreadySelected)
                }
            } else {
                Err(BankerTargetSelectError::InvalidLiabilityId)
            }
        } else {
            Err(BankerTargetSelectError::NotCFO)
        }
    }

    /// Unselect an liability to remove it from the issueliability list when paying the banker
    pub fn unselect_issue_iability(
        &mut self,
        liability_id: usize,
        mut selected: SelectedAssetsAndLiabilities,
    ) -> Result<SelectedAssetsAndLiabilities, BankerTargetSelectError> {
        if let Some(liability) = self.hand.get(liability_id) {
            if liability.is_right() {
                if selected.liabilities.contains_key(&liability_id) {
                    selected.liabilities.remove(&liability_id);
                    Ok(selected.clone())
                } else {
                    Err(BankerTargetSelectError::LiabilityNotSelected)
                }
            } else {
                Err(BankerTargetSelectError::InvalidLiabilityId)
            }
        } else {
            Err(BankerTargetSelectError::InvalidLiabilityId)
        }
    }
}

impl From<BankerTargetPlayer> for RoundPlayer {
    fn from(player: BankerTargetPlayer) -> Self {
        let playable_assets = player.character.playable_assets();
        Self {
            id: player.id,
            name: player.name,
            cash: player.cash,
            assets: player.assets,
            liabilities: player.liabilities,
            character: player.character,
            hand: player.hand,
            liabilities_to_play: player.liabilities_to_play,
            cards_drawn: vec![],
            bonus_draw_cards: 0,
            assets_to_play: playable_assets.total(),
            playable_assets,
            total_cards_drawn: 0,
            total_cards_given_back: 0,
            has_used_ability: false,
        }
    }
}
