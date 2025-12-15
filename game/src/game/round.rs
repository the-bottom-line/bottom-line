//! File containing the round state of the game.

use either::Either;

use crate::{errors::*, game::*, player::*};

/// State containing all information related to the round state of the game. In the round stage,
/// players each play a turn where they can draw cards, play cards and use their character ability.
/// After every player has played a turn, players will be able to select characters again. If one
/// player reached six or more assets during a round, the game will move to [`Results`] instead.
#[derive(Debug, Clone, PartialEq)]
pub struct Round {
    pub(super) current_player: PlayerId,
    pub(super) players: Players<RoundPlayer>,
    pub(super) assets: Deck<Asset>,
    pub(super) liabilities: Deck<Liability>,
    pub(super) markets: Deck<Either<Market, Event>>,
    pub(super) chairman: PlayerId,
    pub(super) current_market: Market,
    pub(super) current_events: Vec<Event>,
    pub(super) open_characters: Vec<Character>,
    pub(super) fired_characters: Vec<Character>,
    pub(super) is_final_round: bool,
}

impl Round {
    /// Get a reference to a [`RoundPlayer`] based on a specific `PlayerId`. Note that the players
    /// are in order, so id 0 refers to the player at index 0 and so on.
    /// See [`Players::player`] for further information
    pub fn player(&self, id: PlayerId) -> Result<&RoundPlayer, GameError> {
        self.players.player(id)
    }

    /// Get a mutable reference to a [`RoundPlayer`] based on a specific `PlayerId`. Note that the
    /// players are in order, so id 0 refers to the player at index 0 and so on.
    /// See [`Players::player_mut`] for further information
    pub fn player_mut(&mut self, id: PlayerId) -> Result<&mut RoundPlayer, GameError> {
        self.players.player_mut(id)
    }

    /// Get a reference to a [`RoundPlayer`] based on a specific `character`. Note that the players
    /// are in order, so id 0 refers to the player at index 0 and so on.
    pub fn player_from_character(&self, character: Character) -> Option<&RoundPlayer> {
        self.players().iter().find(|p| p.character() == character)
    }

    /// Get a reference to a [`RoundPlayer`] based on a specific `name`.
    pub fn player_by_name(&self, name: &str) -> Result<&RoundPlayer, GameError> {
        self.players()
            .iter()
            .find(|p| p.name() == name)
            .ok_or_else(|| GameError::InvalidPlayerName(name.to_owned()))
    }

    /// Get a reference to the [`RoundPlayer`] whose turn it is.
    pub fn current_player(&self) -> &RoundPlayer {
        // PANIC: This is an invariant that holds because `self.current_player` is only assigned by
        // in Round::end_player_turn() and relies on Round::next_player() which is safe. Therefore,
        // `self.current_player` is never invalid.
        self.player(self.current_player)
            .expect("self.current_player went out of bounds")
    }

    /// Get a reference to the [`RoundPlayer`] whose turn is up next. If the current player is the
    /// last player, returns `None` instead.
    ///
    /// NOTE: this will exclude players who will be skipped this round for one reason or another.
    pub fn next_player(&self) -> Option<&RoundPlayer> {
        let current_character = self.current_player().character();
        self.players()
            .iter()
            .filter(|p| {
                p.character() > current_character && !self.fired_characters.contains(&p.character())
            })
            .min_by(|p1, p2| p1.character().cmp(&p2.character()))
    }

    /// Get a mutable reference to the [`RoundPlayer`] whose turn is up next. If the current player
    /// is the last player, returns `None` instead.
    ///
    /// NOTE: this will exclude players who will be skipped this round for one reason or another.
    pub fn next_player_mut(&mut self) -> Option<&mut RoundPlayer> {
        let current_character = self.current_player().character();
        self.players
            .players_mut()
            .iter_mut()
            .filter(|p| {
                p.character() > current_character && !self.fired_characters.contains(&p.character())
            })
            .min_by(|p1, p2| p1.character().cmp(&p2.character()))
    }

    /// Gets a slice of all players in the lobby.
    /// See [`Players::players`] for further information
    pub fn players(&self) -> &[RoundPlayer] {
        self.players.players()
    }

    /// Gets a slice containing all characters that cannot be picked by anyone this round.
    pub fn open_characters(&self) -> &[Character] {
        &self.open_characters
    }

    /// Gets the [`PlayerInfo`] for each player, excluding the player that has the same id as `id`.
    pub fn player_info(&self, id: PlayerId) -> Vec<PlayerInfo> {
        self.players()
            .iter()
            .filter(|p| p.id() != id)
            .map(Into::into)
            .collect()
    }

    /// Gets the current market
    pub fn current_market(&self) -> &Market {
        &self.current_market
    }

    /// Gets whether or not this is the final round
    pub fn is_final_round(&self) -> bool {
        self.is_final_round
    }

    /// Internally used function that checks whether a player with such an `id` exists, and whether
    /// that player is actually the current player. If this is the case, a mutable reference to the
    /// player is returned.
    fn player_as_current_mut(&mut self, id: PlayerId) -> Result<&mut RoundPlayer, GameError> {
        match self.players.player_mut(id) {
            Ok(player) if player.id() == self.current_player => Ok(player),
            Ok(_) => Err(GameError::NotPlayersTurn),
            Err(e) => Err(e),
        }
    }

    /// Gets a list of characters that are available to be fired this round. This will exclude the
    /// list of [`Round::open_characters`] as well as characters that have already been skipped or
    /// fired this round.
    pub fn player_get_fireble_characters(&mut self) -> Vec<Character> {
        Character::CHARACTERS
            .into_iter()
            .filter(|c| {
                c.can_be_fired()
                    && !self.fired_characters.contains(c)
                    && !self.open_characters.contains(c)
            })
            .clone()
            .collect()
    }

    /// Gets the number of assets and liabilities for each player the regulator can choose to swap
    /// with. This excludes their own cards.
    pub fn player_get_regulator_swap_players(&mut self) -> Vec<RegulatorSwapPlayer> {
        self.players()
            .iter()
            .filter(|p| p.character() != Character::Regulator)
            .map(|p| RegulatorSwapPlayer {
                player_id: p.id(),
                asset_count: p.hand().iter().filter(|c| c.is_left()).count(),
                liability_count: p.hand().iter().filter(|c| c.is_right()).count(),
            })
            .collect()
    }

    /// Allows player with id `id` to play a card from their hand at index `card_idx`. If this
    /// player was the first to buy their first, second, third, fourth, fifth, seventh, eight or
    /// ninth asset, a new market and corresponding triggered events will be returned. The card that
    /// was played will also be returned.
    pub fn player_play_card(
        &mut self,
        id: PlayerId,
        card_idx: usize,
    ) -> Result<PlayerPlayedCard, GameError> {
        let old_max_bought_assets = self.max_bought_assets();
        let player = self.player_as_current_mut(id)?;

        match player.play_card(card_idx)? {
            Either::Left(asset) => {
                if !self.is_final_round() && self.check_is_final_round() {
                    // Keep the borrow checker happy
                    let player = self.player_as_current_mut(id)?;
                    player.enable_first_to_six_assets_bonus();
                }

                self.is_final_round = self.check_is_final_round();

                let market = match self.should_refresh_market(old_max_bought_assets) {
                    true => Some(self.refresh_market()),
                    false => None,
                };
                let used_card = Either::Left(asset);
                let is_final_round = self.is_final_round;

                Ok(PlayerPlayedCard {
                    market,
                    used_card,
                    is_final_round,
                })
            }
            Either::Right(liability) => {
                let market = None;
                let used_card = Either::Right(liability);
                let is_final_round = self.is_final_round;

                Ok(PlayerPlayedCard {
                    market,
                    used_card,
                    is_final_round,
                })
            }
        }
    }

    /// This allows player with id `id` to redeem a liability at index `liability_idx` if they are
    /// the [`CFO`](Character::CFO) and if they can afford to pay off the debt. If they can redeem
    /// the liability, it will be added back into the deck.
    pub fn player_redeem_liability(
        &mut self,
        id: PlayerId,
        liability_idx: usize,
    ) -> Result<(), GameError> {
        let player = self.player_as_current_mut(id)?;

        let liability = player.redeem_liability(liability_idx)?;
        self.liabilities.put_back(liability);

        Ok(())
    }

    /// This allows player with id `id` to draw a card of card type `card_type`. If they were
    /// allowed to draw that card, a reference to the card will be returned.
    pub fn player_draw_card(
        &mut self,
        id: PlayerId,
        card_type: CardType,
    ) -> Result<Either<&Asset, &Liability>, GameError> {
        // TODO: think of way to use `player_as_current_mut()` without taking `&mut self` to be
        // able to do `&mut self.assets` later in the function
        match self.players.player_mut(id) {
            Ok(player) if player.id() == self.current_player => match card_type {
                CardType::Asset => {
                    let asset = player.draw_asset(&mut self.assets)?;
                    Ok(Either::Left(asset))
                }
                CardType::Liability => {
                    let liability = player.draw_liability(&mut self.liabilities)?;
                    Ok(Either::Right(liability))
                }
            },
            Ok(_) => Err(GameError::NotPlayersTurn),
            Err(e) => Err(e),
        }
    }

    /// This allows player with id `id` to give back a card from their hand at index `card_idx`. If
    /// they were able to give back the card, the card type of this card will be returned.
    pub fn player_give_back_card(
        &mut self,
        id: PlayerId,
        card_idx: usize,
    ) -> Result<CardType, GameError> {
        let player = self.player_as_current_mut(id)?;

        match player.give_back_card(card_idx)? {
            Either::Left(asset) => {
                self.assets.put_back(asset);
                Ok(CardType::Asset)
            }
            Either::Right(liability) => {
                self.liabilities.put_back(liability);
                Ok(CardType::Liability)
            }
        }
    }

    /// This allows player with id `id` to fire a player who has character `character` if they are
    /// the shareholder. If this is successful, the player who got fired will not play their turn
    /// this round.
    pub fn player_fire_character(
        &mut self,
        id: PlayerId,
        character: Character,
    ) -> Result<Character, GameError> {
        let player = self.player_as_current_mut(id)?;
        let character = player.fire_character(character)?;
        self.fired_characters.push(character);
        Ok(character)
    }

    /// This allows player with id `id` to swap a list of cards from their hand at indexes
    /// `card_idxs` with the deck. If succesful, this function returns the number of cards that were
    /// swapped with the deck in total.
    pub fn player_swap_with_deck(
        &mut self,
        id: PlayerId,
        card_idxs: Vec<usize>,
    ) -> Result<Vec<usize>, GameError> {
        // cant use player_as_current_mut here because of multiple mutable borrows of self. hmm.
        let player = match self.players.player_mut(id) {
            Ok(player) if player.id() == self.current_player => player,
            Ok(_) => return Err(GameError::NotPlayersTurn),
            Err(e) => return Err(e),
        };

        let drawcount =
            player.swap_with_deck(card_idxs, &mut self.assets, &mut self.liabilities)?;
        Ok(drawcount)
    }

    /// This allows a player with id `id` to swap their hand of cards with a player with id
    /// `target_id`. If succesful, a copy of each player's new hand is returned.
    pub fn player_swap_with_player(
        &mut self,
        id: PlayerId,
        target_id: PlayerId,
    ) -> Result<HandsAfterSwap, GameError> {
        // Same debug assertions as below
        #[cfg(debug_assertions)]
        {
            let ps_index = self.players().iter().position(|p| p.id() == id);
            let pt_index = self.players().iter().position(|p| p.id() == target_id);
            if let Some(psi) = ps_index
                && let Some(pti) = pt_index
            {
                debug_assert_eq!(psi as u8, id.0);
                debug_assert_eq!(pti as u8, target_id.0);
            }
        }

        if id != target_id {
            match self
                .players
                .get_disjoint_mut([usize::from(id), usize::from(target_id)])
            {
                Ok([regulator, target]) => {
                    regulator.regulator_swap_with_player(target)?;
                    let hands = HandsAfterSwap {
                        regulator_new_hand: regulator.hand().to_vec(),
                        target_new_hand: target.hand().to_vec(),
                    };
                    Ok(hands)
                }
                Err(_) => Err(SwapError::InvalidTargetPlayer.into()),
            }
        } else {
            Err(SwapError::InvalidTargetPlayer.into())
        }
    }

    /// This allows a player with id `id` to force player with id `target_id` to divest an asset at
    /// index `asset_idx` for market value minus 1. If succesful, returns the amount of gold it cost
    /// to divest the asset for.
    pub fn player_divest_asset(
        &mut self,
        id: PlayerId,
        target_id: PlayerId,
        asset_idx: usize,
    ) -> Result<u8, GameError> {
        // I've done a lot of work to ensure player id == player index. This should be
        // unnecessary, but I'll leave the check enabled for debug builds.
        #[cfg(debug_assertions)]
        {
            let ps_index = self.players().iter().position(|p| p.id() == id);
            let pt_index = self.players().iter().position(|p| p.id() == target_id);
            if let Some(psi) = ps_index
                && let Some(pti) = pt_index
            {
                debug_assert_eq!(psi as u8, id.0);
                debug_assert_eq!(pti as u8, target_id.0);
            }
        }

        if id != target_id {
            match self
                .players
                .get_disjoint_mut([usize::from(id), usize::from(target_id)])
            {
                Ok([stakeholder, target]) => {
                    let cost = stakeholder.divest_asset(target, asset_idx, &self.current_market)?;
                    target.remove_asset(asset_idx)?;
                    Ok(cost)
                }
                Err(_) => Err(DivestAssetError::InvalidCharacter.into()),
            }
        } else {
            Err(DivestAssetError::InvalidCharacter.into())
        }
    }

    /// Gets a list of [`DivestPlayer`], which contains their player id as well as each asset that
    /// can be divested as well as the current cost to do so. This list excludes their own cards.
    pub fn get_divest_assets(&mut self, id: PlayerId) -> Result<Vec<DivestPlayer>, GameError> {
        let player = self.player_as_current_mut(id)?;
        if player.character() == Character::Stakeholder {
            Ok(self
                .players()
                .iter()
                .filter(|p| p.id() != id) // Not yourself
                .filter(|p| p.character() != Character::CSO) // Not CSO
                .map(|p| DivestPlayer {
                    player_id: p.id(),
                    assets: p
                        .assets()
                        .iter()
                        .map(|a| DivestAsset {
                            asset: a.clone(),
                            divest_cost: a.divest_cost(&self.current_market),
                            is_divestable: a.color != Color::Red && a.color != Color::Green,
                        })
                        .collect(),
                })
                .collect())
        } else {
            Err(DivestAssetError::InvalidPlayerCharacter.into())
        }
    }

    /// Gets a list of characters that are skipped between the turns of two players. Characters are
    /// called in order, so if any character is called but unavailable for any reason (not selected,
    /// fired or otherwise skipped), they will be added to this list.
    pub fn skipped_characters(&self) -> Vec<Character> {
        let current_character = self.current_player().character();
        let mut skipped = Character::CHARACTERS
            .into_iter()
            .rev()
            .skip_while(|c| *c >= current_character)
            .take_while(|c| {
                self.player_from_character(*c).is_none() || self.fired_characters.contains(c)
            })
            .collect::<Vec<_>>();

        skipped.sort();

        skipped
    }

    /// Ends the turn of the player with id `id`. If succesful and this player is not the last
    /// player to play this round, this function, returns [`TurnEnded`], which contains the next
    /// player as well as whether or not the game has ended. If succesful and the player is the last
    /// turn of the round, returs a new [`GameState`] of [`SelectingCharacters`].
    pub(super) fn end_player_turn(
        &mut self,
        id: PlayerId,
    ) -> Result<Either<TurnEnded, GameState>, GameError> {
        let player = self.player_as_current_mut(id)?;
        if !player.should_give_back_cards() {
            if let Some(id) = self.next_player().map(|p| p.id()) {
                let player = self.players.player_mut(id)?;

                player.start_turn(&self.current_market);

                self.current_player = player.id();

                let turn_ended = TurnEnded {
                    next_player: Some(self.current_player),
                    game_ended: false,
                };

                Ok(Either::Left(turn_ended))
            } else if !self.is_final_round() {
                let maybe_ceo = self.player_from_character(Character::CEO);
                let chairman_id = match maybe_ceo.map(|p| p.id()) {
                    Some(id) => id,
                    None => self.chairman,
                };

                let characters = ObtainingCharacters::new(self.players.len(), chairman_id)?;
                let players = std::mem::take(&mut self.players);
                let assets = std::mem::take(&mut self.assets);
                let liabilities = std::mem::take(&mut self.liabilities);
                let markets = std::mem::take(&mut self.markets);
                let current_market = std::mem::take(&mut self.current_market);
                let current_events = std::mem::take(&mut self.current_events);

                let players = Players(players.into_iter().map(Into::into).collect());

                let state = GameState::SelectingCharacters(SelectingCharacters {
                    players,
                    characters,
                    assets,
                    liabilities,
                    markets,
                    chairman: chairman_id,
                    current_market,
                    current_events,
                });

                Ok(Either::Right(state))
            } else {
                let final_events = std::mem::take(&mut self.current_events);
                let players = std::mem::take(&mut self.players);

                let players = Players(
                    players
                        .into_iter()
                        .map(|round_player| ResultsPlayer::new(round_player, self.current_market()))
                        .collect(),
                );

                let state = GameState::Results(Results {
                    players,
                    final_events,
                });

                Ok(Either::Right(state))
            }
        } else {
            Err(GameError::PlayerShouldGiveBackCard)
        }
    }

    /// Checks whether someone has bought equal to or more assets than [`ASSETS_FOR_END_OF_GAME`].
    /// If so, this should be the final round.
    fn check_is_final_round(&self) -> bool {
        self.max_bought_assets() >= ASSETS_FOR_END_OF_GAME
    }

    /// Returns the highest amount of assets of any player.
    fn max_bought_assets(&self) -> usize {
        self.players()
            .iter()
            .map(|player| player.assets().len())
            .max()
            .unwrap_or_default()
    }

    /// Checks whether or not a market should be refreshed based on whether or not someone was the
    /// first to buy their first, second, third, fourth, fifth, seventh, eight or ninth asset.
    fn should_refresh_market(&self, old_max_bought_assets: usize) -> bool {
        let max_bought_assets = self.max_bought_assets();

        max_bought_assets > old_max_bought_assets && max_bought_assets != ASSETS_FOR_END_OF_GAME
    }

    /// Generates a new market change. Cards will be taken from the market/event deck one by one
    /// until a new market is encountered, returning a [`MarketChange`].
    fn refresh_market(&mut self) -> MarketChange {
        let mut events = vec![];

        loop {
            match self.markets.draw() {
                Either::Left(new_market) => {
                    self.current_market = new_market.clone();
                    break MarketChange { events, new_market };
                }
                Either::Right(event) => {
                    self.current_events.push(event.clone());
                    events.push(event);
                }
            }
        }
    }
}

/// Used to return the new hands for the regulator and its player target.
#[derive(Debug, Clone)]
pub struct HandsAfterSwap {
    /// The new hand of the regulator
    pub regulator_new_hand: Vec<Either<Asset, Liability>>,
    /// The new hand for the regulator's target
    pub target_new_hand: Vec<Either<Asset, Liability>>,
}
