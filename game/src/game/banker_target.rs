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
}
