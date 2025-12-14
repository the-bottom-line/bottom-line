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