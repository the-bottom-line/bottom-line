//! File containing the round state of the game.

use either::Either;

use crate::{errors::*, game::*, player::*};

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
                    let pbp = player.pay_banker(cash, banker)?;
                    return Ok(pbp)
                }else{
                    Err(PayBankerError::NotRightCashAmount.into())
                }
                
            }
            Err(_) => Err(PayBankerError::NoBankerPlayer.into()),
        }
    }
}
