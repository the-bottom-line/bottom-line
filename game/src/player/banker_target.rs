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
    pub (super) id: PlayerId,
    pub (super) name: String,
    pub (super) cash: u8,
    pub (super) assets: Vec<Asset>,
    pub (super) liabilities: Vec<Liability>,
    pub (super) character: Character,
    pub (super) hand: Vec<Either<Asset, Liability>>,
    pub (super) liabilities_to_play: u8,
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
        banker: &mut BankerTargetPlayer,
    ) -> Result<PayBankerPlayer, PayBankerError> {
        if self.cash >= cash {
            banker.cash += cash;
            self.cash -= cash;
            Ok(PayBankerPlayer {
                cash: cash,
                target_id: self.id,
                banker_id: banker.id,
            })
        } else {
            Err(PayBankerError::NotEnoughCash)
        }
    }

    pub fn select_divest_asset(
        &mut self,
        asset_id: usize,
        market: &Market,
        mut selected: SelectedAssetsAndLiabilities,
    ) -> Result<SelectedAssetsAndLiabilities, BankerTargetSelectError> {
        if let Some(asset) = self.assets.get(asset_id){
            if !selected.assets.contains_key(&asset_id){
                if asset.market_value(market) <= 0 {
                    let v = asset.market_value(market);
                    selected.assets.insert(asset_id,v as u8);
                    Ok(selected.clone())
                }else{
                    Err(BankerTargetSelectError::AssetValueToLow)
                }
            }else{
                Err(BankerTargetSelectError::AssetAlreadySelected)
            }
        }else{
            Err(BankerTargetSelectError::InvalidAssetId)
        }
    }

    pub fn unselect_divest_asset(
        &mut self,
        asset_id: usize,
        mut selected: SelectedAssetsAndLiabilities,
    ) -> Result<SelectedAssetsAndLiabilities, BankerTargetSelectError> {
        if self.assets.get(asset_id).is_some(){
            if selected.assets.contains_key(&asset_id){
                selected.assets.remove(&asset_id);
                Ok(selected.clone())
            }else{
                Err(BankerTargetSelectError::AssetNotSelected)
            }
        }else{
            Err(BankerTargetSelectError::InvalidAssetId)
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
