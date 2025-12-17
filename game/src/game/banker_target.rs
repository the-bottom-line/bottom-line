//! File containing the round state of the game.

use either::Either;

use crate::{
    errors::*,
    game::*,
    player::{self, *},
};

#[derive(Debug, Clone, PartialEq)]
pub struct BankerTargetRound {
    pub(super) current_player: PlayerId,
    pub(super) players: Players<BankerTargetPlayer>,
    pub(super) assets: Deck<Asset>,
    pub(super) liabilities: Deck<Liability>,
    pub(super) markets: Deck<Either<Market, Event>>,
    pub(super) chairman: PlayerId,
    pub(super) current_market: Market,
    pub(super) current_events: Vec<Event>,
    pub(super) open_characters: Vec<Character>,
    pub(super) fired_characters: Vec<Character>,
    pub(super) gold_to_be_paid: u8,
    pub(super) can_pay_banker: bool,
    pub(super) selected_cards: SelectedAssetsAndLiabilities,
    pub(super) is_final_round: bool,
}

impl BankerTargetRound {
    /// Get a reference to the [`BankerTargetPlayer`] whose turn it is.
    pub fn current_player(&self) -> &BankerTargetPlayer {
        // PANIC: This is an invariant that holds because `self.current_player` is only assigned by
        // in Round::end_player_turn() and relies on Round::next_player() which is safe. Therefore,
        // `self.current_player` is never invalid.
        self.player(self.current_player)
            .expect("self.current_player went out of bounds")
    }

    pub fn player(&self, id: PlayerId) -> Result<&BankerTargetPlayer, GameError> {
        self.players.player(id)
    }

    pub fn gold_to_be_paid(&self) -> u8 {
        self.gold_to_be_paid
    }

    pub fn can_pay_banker(&self) -> bool {
        self.can_pay_banker
    }

    /// Gets a slice of all players in the lobby.
    /// See [`Players::players`] for further information
    pub fn players(&self) -> &[BankerTargetPlayer] {
        self.players.players()
    }
    /// Get a reference to a [`BankerTargetPlayer`] based on a specific `name`.
    pub fn player_by_name(&self, name: &str) -> Result<&BankerTargetPlayer, GameError> {
        self.players()
            .iter()
            .find(|p| p.name() == name)
            .ok_or_else(|| GameError::InvalidPlayerName(name.to_owned()))
    }

    /// Internally used function that checks whether a player with such an `id` exists, and whether
    /// that player is actually the current player. If this is the case, a mutable reference to the
    /// player is returned.
    fn player_as_current_mut(
        &mut self,
        id: PlayerId,
    ) -> Result<&mut BankerTargetPlayer, GameError> {
        match self.players.player_mut(id) {
            Ok(player) if player.id() == self.current_player => Ok(player),
            Ok(_) => Err(GameError::NotPlayersTurn),
            Err(e) => Err(e),
        }
    }

    /// function to pay the banker and switch game back to a normal round state
    pub fn player_pay_banker(
        &mut self,
        player_id: PlayerId,
        cash: u8,
    ) -> Result<PayBankerPlayer, GameError> {
        let banker_id = self
            .players()
            .iter()
            .find(|p| p.character() == Character::Banker)
            .ok_or(PayBankerError::NoBankerPlayer)?
            .id();
        match self
            .players
            .get_disjoint_mut([usize::from(player_id), usize::from(banker_id)])
        {
            Ok([player, banker]) => {
                if cash == self.gold_to_be_paid {
                    let pbp = player.pay_banker(cash, &self.selected_cards, banker)?;
                    return Ok(pbp);
                } else {
                    Err(PayBankerError::NotRightCashAmount.into())
                }
            }
            Err(_) => Err(PayBankerError::NoBankerPlayer.into()),
        }
    }

    ///function to select an asset for divesting when targeted by the banker
    pub fn player_select_divest_asset(
        &mut self,
        player_id: PlayerId,
        asset_id: usize,
    ) -> Result<SelectedAssetsAndLiabilities, GameError> {
        let selected = self.selected_cards.clone();
        let market = self.current_market.clone();
        let player = self.player_as_current_mut(player_id)?;
        self.selected_cards = player.select_divest_asset(asset_id, &market, selected)?;
        Ok(self.selected_cards.clone())
    }

    ///function to unselect an asset for divesting when targeted by the banker
    pub fn player_unselect_divest_asset(
        &mut self,
        player_id: PlayerId,
        asset_id: usize,
    ) -> Result<SelectedAssetsAndLiabilities, GameError> {
        let selected = self.selected_cards.clone();
        let player = self.player_as_current_mut(player_id)?;
        self.selected_cards = player.unselect_divest_asset(asset_id, selected)?;
        Ok(self.selected_cards.clone())
    }

    ///function to select an liability to issue when targeted by the banker
    pub fn player_select_issue_liability(
        &mut self,
        player_id: PlayerId,
        liability_id: usize,
    ) -> Result<SelectedAssetsAndLiabilities, GameError> {
        let selected = self.selected_cards.clone();
        let player = self.player_as_current_mut(player_id)?;
        self.selected_cards = player.select_issue_iability(liability_id, selected)?;
        Ok(self.selected_cards.clone())
    }

    ///function to unselect an asset for divesting when targeted by the banker
    pub fn player_unselect_issue_liability(
        &mut self,
        player_id: PlayerId,
        liability_id: usize,
    ) -> Result<SelectedAssetsAndLiabilities, GameError> {
        let selected = self.selected_cards.clone();
        let player = self.player_as_current_mut(player_id)?;
        self.selected_cards = player.unselect_issue_iability(liability_id, selected)?;
        Ok(self.selected_cards.clone())
    }
}

// TODO: use separate function that uses std::mem::take rather than clones
impl From<&mut round::Round> for BankerTargetRound {
    fn from(round: &mut Round) -> Self {
        let color_array: Vec<Color> = round
            .current_player()
            .assets()
            .iter()
            .map(|a| a.color)
            .collect();

        let gtbp = color_array.iter().collect::<HashSet<_>>().len() as u8 + 1;
        let asset_values: Vec<u8> = round
            .current_player()
            .assets()
            .iter()
            .map(|a| {
                if a.market_value(&round.current_market) > 0 {
                    a.market_value(&round.current_market) as u8
                } else {
                    0 as u8
                }
            })
            .collect();
        let total_asset_value: u8 = asset_values.iter().sum();
        let mut total_libility_value: u8 = 0;
        if round.current_player().character() == Character::CFO {
            let liability_values: Vec<u8> = round
                .current_player()
                .hand()
                .iter()
                .filter(|c| c.is_right())
                .map(|l| l.clone().right().unwrap().value)
                .collect();
            if liability_values.len() <= 3 {
                total_libility_value = liability_values.iter().sum();
            } else {
                let mut lvs = liability_values.clone();
                lvs.sort();
                total_libility_value = lvs[0] + lvs[1] + lvs[2];
            }
        }

        Self {
            current_player: round.current_player,
            players: Players(round.players.iter().map(Into::into).collect()),
            assets: round.assets.clone(),
            liabilities: round.liabilities.clone(),
            markets: round.markets.clone(),
            chairman: round.chairman,
            current_market: round.current_market.clone(),
            current_events: round.current_events.clone(),
            open_characters: round.open_characters.clone(),
            fired_characters: round.fired_characters.clone(),
            is_final_round: round.is_final_round,
            gold_to_be_paid: gtbp,
            can_pay_banker: gtbp
                <= total_libility_value + total_asset_value + round.current_player().cash(),
            selected_cards: SelectedAssetsAndLiabilities {
                assets: HashMap::new(),
                liabilities: HashMap::new(),
            },
        }
    }
}
