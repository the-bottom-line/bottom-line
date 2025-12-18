//! This file contains the implementation of [`BankerTargetPlayer`].

use crate::{errors::*, game::*, player::*};

use either::Either;
use std::collections::{HashMap, hash_map::Entry};

/// The player type that corresponds to the [`BankerTargetRound`](crate::game::BankerTargetRound)
/// stage of the game.
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
    pub(super) was_first_to_six_assets: bool,
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

    /// Gets an asset at a particular index from this player.
    pub fn asset(&self, asset_idx: usize) -> Result<&Asset, GameError> {
        self.assets
            .get(asset_idx)
            .ok_or(GameError::InvalidAssetIndex(asset_idx as u8))
    }

    /// Gets the hand with cards of this player.
    pub fn hand(&self) -> &[Either<Asset, Liability>] {
        &self.hand
    }

    /// Pays the banker in the round with everything the player owns that are worth anything. This
    /// means that this function ignores assets that are worth zero or negative cash in the current
    /// market.
    pub fn go_bankrupt_for_banker(
        &mut self,
        cash: u8,
        banker: &mut BankerTargetPlayer,
        market: Market,
    ) -> Result<PayBankerPlayer, PayBankerError> {
        let mut new_selected_cards: SelectedAssetsAndLiabilities = SelectedAssetsAndLiabilities {
            sold_assets: vec![],
            issued_liabilities: vec![],
        };
        for (index, asset) in self.assets.clone().into_iter().enumerate() {
            if asset.market_value(&market) > 0 {
                new_selected_cards.sold_assets.push(SoldAssetToPayBanker {
                    asset_idx: index,
                    market_value: asset.market_value(&market) as u8,
                });
            }
        }
        //get top 3 most valueble liabilities if player is CFO
        if self.character == Character::CFO {
            for (index, liability) in self
                .hand
                .clone()
                .into_iter()
                .filter(|l| l.is_right())
                .enumerate()
            {
                if let Some(lib) = liability.right() {
                    new_selected_cards
                        .issued_liabilities
                        .push(IssuedLiabilityToPayBanker {
                            card_idx: index,
                            liability: lib,
                        });
                }
            }
        }
        let mut len = new_selected_cards.issued_liabilities.iter().count();

        if len > 3 {
            len -= 3;
        } else {
            len = 0;
        }
        //remove smallest libilities if there are more as 3 in hand
        // TODO: use smallest_k.
        for _ in 0..len {
            let mut _smallest_k: usize = 100;
            let mut smallest_v = 0;
            let mut index = 0;
            for l in &new_selected_cards.issued_liabilities {
                if smallest_v < l.liability.value {
                    smallest_v = l.liability.value;
                    _smallest_k = l.card_idx;
                    index += 1;
                }
            }
            new_selected_cards.issued_liabilities.remove(index);
        }

        // Sell assets and libilities for targeted player
        let extra_asset_cash: u8 = new_selected_cards
            .sold_assets
            .iter()
            .map(|s| s.market_value)
            .sum();
        let extra_liability_cash: u8 = new_selected_cards
            .issued_liabilities
            .iter()
            .map(|l| l.liability.value)
            .sum();
        let mut asset_ids: Vec<usize> = new_selected_cards
            .sold_assets
            .iter()
            .map(|s| s.asset_idx)
            .collect();
        asset_ids.sort();
        for id in asset_ids.iter().rev() {
            self.assets.remove(*id);
        }

        let mut liability_ids: Vec<usize> = new_selected_cards
            .issued_liabilities
            .iter()
            .map(|l| l.card_idx)
            .collect();
        liability_ids.sort();
        for id in liability_ids.iter().rev() {
            self.hand.remove(*id);
            self.liabilities_to_play -= 1;
        }
        let total_available_cash = extra_asset_cash + extra_liability_cash + self.cash;
        if total_available_cash < cash {
            //TODO Pay banker the maximum amount target can affort after selling
            banker.cash += total_available_cash;
            self.cash = 0;

            Ok(PayBankerPlayer {
                paid_amount: total_available_cash,
                new_banker_cash: banker.cash,
                new_target_cash: self.cash,
                target_id: self.id,
                banker_id: banker.id,
                selected_cards: new_selected_cards.clone(),
            })
        } else {
            Err(PayBankerError::NotRightCashAmount {
                expected: total_available_cash,
                got: cash,
            })
        }
    }

    /// Pays the banker in the round the requested amount of gold
    pub fn pay_banker(
        &mut self,
        cash: u8,
        selected_assets: &HashMap<usize, u8>,
        selected_liabilities: &HashMap<usize, u8>,
        banker: &mut BankerTargetPlayer,
    ) -> Result<PayBankerPlayer, PayBankerError> {
        let extra_asset_cash = selected_assets.values().sum::<u8>();
        let extra_liability_cash = selected_liabilities.values().sum::<u8>();

        if self.cash + extra_asset_cash + extra_liability_cash >= cash {
            banker.cash += cash;
            self.cash += extra_asset_cash + extra_liability_cash;
            self.cash -= cash;

            // TODO: reuse in `create_select_assets_liabilities` somehow
            let sold_assets = selected_assets
                .iter()
                .map(|(&asset_idx, &market_value)| SoldAssetToPayBanker {
                    asset_idx,
                    market_value,
                })
                .collect::<Vec<_>>();

            let issued_liabilities = selected_liabilities
                .iter()
                .map(|(&card_idx, _)| {
                    // TODO: verify legitimacy of unwrapping here
                    let liability = self.hand.get(card_idx).unwrap().clone().right().unwrap();
                    IssuedLiabilityToPayBanker {
                        card_idx,
                        liability,
                    }
                })
                .collect::<Vec<_>>();

            let selected_cards = SelectedAssetsAndLiabilities {
                sold_assets,
                issued_liabilities,
            };

            let mut asset_idxs = selected_assets.iter().map(|a| *a.0).collect::<Vec<_>>();

            asset_idxs.sort();

            for asset_idx in asset_idxs.iter().rev() {
                // TODO: figure out if this can have invalid indices
                self.assets.remove(*asset_idx);
            }

            let mut liability_idxs = selected_liabilities
                .iter()
                .map(|l| *l.0)
                .collect::<Vec<_>>();

            liability_idxs.sort();

            for card_idx in liability_idxs.iter().rev() {
                self.hand.remove(*card_idx);
                self.liabilities_to_play -= 1;
            }

            Ok(PayBankerPlayer {
                paid_amount: cash,
                new_banker_cash: banker.cash,
                new_target_cash: self.cash,
                target_id: self.id,
                banker_id: banker.id,
                selected_cards,
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
        selected_assets: &mut HashMap<usize, u8>,
    ) -> Result<&Asset, BankerTargetSelectError> {
        if let Some(asset) = self.assets.get(asset_id) {
            if let Entry::Vacant(entry) = selected_assets.entry(asset_id) {
                let market_value = asset.market_value(market);
                if market_value > 0 {
                    entry.insert(market_value as u8);
                    Ok(asset)
                } else {
                    Err(BankerTargetSelectError::AssetValueToLow)
                }
            } else {
                Err(BankerTargetSelectError::AssetAlreadySelected)
            }
        } else {
            // TODO: use GameError::InvalidAssetIndex or self.asset(asset_idx)
            Err(BankerTargetSelectError::InvalidAssetId)
        }
    }

    /// Unselect an asset to remove it from divest asset list when paying the banker
    pub fn unselect_divest_asset(
        &mut self,
        asset_id: usize,
        selected_assets: &mut HashMap<usize, u8>,
    ) -> Result<&Asset, BankerTargetSelectError> {
        if let Some(asset) = self.assets.get(asset_id) {
            if let Some(_market_value) = selected_assets.remove(&asset_id) {
                Ok(asset)
            } else {
                Err(BankerTargetSelectError::AssetNotSelected)
            }
        } else {
            // TODO: use GameError::InvalidAssetIndex or self.asset(asset_idx)
            Err(BankerTargetSelectError::InvalidAssetId)
        }
    }

    /// Select a liability from this player's hand and adds it to the issue liability list when
    /// paying the banker
    pub fn select_issue_liability(
        &mut self,
        card_idx: usize,
        selected_liabilities: &mut HashMap<usize, u8>,
    ) -> Result<&Liability, BankerTargetSelectError> {
        if self.character == Character::CFO {
            if let Some(Either::Right(liability)) = self.hand.get(card_idx) {
                let playable_liabilities = Character::CFO.playable_liabilities() as usize;
                if selected_liabilities.len() < playable_liabilities {
                    if let Entry::Vacant(entry) = selected_liabilities.entry(card_idx) {
                        entry.insert(liability.value);
                        Ok(liability)
                    } else {
                        Err(BankerTargetSelectError::LiabilityAlreadySelected)
                    }
                } else {
                    Err(BankerTargetSelectError::AlreadySelected3Liabilities)
                }
            } else {
                Err(BankerTargetSelectError::InvalidLiabilityId(card_idx as u8))
            }
        } else {
            Err(BankerTargetSelectError::NotCFO)
        }
    }

    /// Unselect an liability to remove it from the issueliability list when paying the banker
    pub fn unselect_issue_liability(
        &mut self,
        card_idx: usize,
        selected_liabilities: &mut HashMap<usize, u8>,
    ) -> Result<&Liability, BankerTargetSelectError> {
        if let Some(Either::Right(liability)) = self.hand.get(card_idx) {
            if let Some(_market_value) = selected_liabilities.remove(&card_idx) {
                Ok(liability)
            } else {
                Err(BankerTargetSelectError::LiabilityNotSelected)
            }
        } else {
            Err(BankerTargetSelectError::InvalidLiabilityId(card_idx as u8))
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
            was_first_to_six_assets: false,
        }
    }
}
