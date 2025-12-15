//! File containing the selecting characters state of the game.

use either::Either;

use crate::{errors::*, game::*, player::*};

/// State containing all information related to the selecting characters state of the game. In the
/// selecting characters stage, players select a character one by one until everyone has selected
/// a character, after which a round starts.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectingCharacters {
    pub(super) players: Players<SelectingCharactersPlayer>,
    pub(super) characters: ObtainingCharacters,
    pub(super) assets: Deck<Asset>,
    pub(super) liabilities: Deck<Liability>,
    pub(super) markets: Deck<Either<Market, Event>>,
    pub(super) chairman: PlayerId,
    pub(super) current_market: Market,
    pub(super) current_events: Vec<Event>,
}

impl SelectingCharacters {
    /// Get a reference to a [`SelectingCharactersPlayer`] based on a specific `PlayerId`. Note
    /// that the players are in order, so id 0 refers to the player at index 0 and so on.
    /// See [`Players::player`] for further information
    pub fn player(&self, id: PlayerId) -> Result<&SelectingCharactersPlayer, GameError> {
        self.players.player(id)
    }

    /// Get a reference to a [`SelectingCharactersPlayer`] based on a specific `name`. Note
    /// that the players are in order, so id 0 refers to the player at index 0 and so on.
    pub fn player_by_name(&self, name: &str) -> Result<&SelectingCharactersPlayer, GameError> {
        self.players()
            .iter()
            .find(|p| p.name() == name)
            .ok_or_else(|| GameError::InvalidPlayerName(name.to_owned()))
    }

    /// Gets a slice of all players in the lobby.
    /// See [`Players::players`] for further information
    pub fn players(&self) -> &[SelectingCharactersPlayer] {
        self.players.players()
    }

    /// Gets the id of the current chairman
    pub fn chairman_id(&self) -> PlayerId {
        self.chairman
    }

    /// Gets the id of the player that's currently selecting a character
    pub fn currently_selecting_id(&self) -> PlayerId {
        (self.characters.applies_to_player() as u8).into()
    }

    /// Internally used function that checks whether a player with such an `id` exists, and whether
    /// that player is the current player. If this is the case, a reference to the player is
    /// returned.
    fn player_as_current(&self, id: PlayerId) -> Result<&SelectingCharactersPlayer, GameError> {
        let currently_selecting_id = self.currently_selecting_id();
        match self.players.player(id) {
            Ok(player) if player.id() == currently_selecting_id => Ok(player),
            Ok(_) => Err(GameError::NotPlayersTurn),
            Err(e) => Err(e),
        }
    }

    /// Gets a list of selectable characters for the player with `id`, if it's their turn to select
    /// a character next.
    pub fn player_get_selectable_characters(
        &self,
        id: PlayerId,
    ) -> Result<Vec<Character>, GameError> {
        let _ = self.player_as_current(id)?;

        self.characters
            .peek()
            .map(|pc| pc.characters)
            .map_err(Into::into)
    }

    /// Gets the closed character for the player with `id` if they're chairman.
    pub fn player_get_closed_character(&self, id: PlayerId) -> Result<Character, GameError> {
        let _ = self.player_as_current(id)?;

        match self.characters.peek()?.closed_character {
            Some(closed_character) => Ok(closed_character),
            None => Err(SelectingCharactersError::NotChairman.into()),
        }
    }

    /// Allows player with `id` to select `character`, if it is their turn and if that character is
    /// available to select. If they are the last player to select a character, a new [`GameState`]
    /// is returned of type [`Round`].
    pub(super) fn player_select_character(
        &mut self,
        id: PlayerId,
        character: Character,
    ) -> Result<Option<GameState>, GameError> {
        let currently_selecting_id = self.currently_selecting_id();

        match self.players.player_mut(id) {
            Ok(p) if p.id() == currently_selecting_id => {
                self.characters.pick(character)?;

                p.select_character(character)?;

                // Start round when no more characters can be picked
                if self.characters.peek().is_err() {
                    let current_player = self
                        .players()
                        .iter()
                        .min_by(|p1, p2| p1.character().cmp(&p2.character()))
                        .map(|p| p.id())
                        .unwrap();
                    // PANIC: This is safe because a game has to have at least four players to
                    // start, and they cannot be removed

                    let players = std::mem::take(&mut self.players);
                    let assets = std::mem::take(&mut self.assets);
                    let liabilities = std::mem::take(&mut self.liabilities);
                    let markets = std::mem::take(&mut self.markets);
                    let current_market = std::mem::take(&mut self.current_market);
                    let current_events = std::mem::take(&mut self.current_events);
                    let open_characters = self.characters.open_characters().to_vec();
                    let fired_characters: Vec<Character> = vec![];
                    let banker_target = None;
                    let players = players
                        .0
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<_, _>>()?;

                    let players = Players(players);

                    let mut round = Round {
                        current_player,
                        players,
                        assets,
                        liabilities,
                        markets,
                        chairman: self.chairman,
                        current_market,
                        current_events,
                        open_characters,
                        fired_characters,
                        banker_target,
                    };

                    round
                        .players
                        .player_mut(current_player)?
                        .start_turn(&round.current_market);

                    Ok(Some(GameState::Round(round)))
                } else {
                    Ok(None)
                }
            }
            Ok(_) => Err(GameError::NotPlayersTurn),
            Err(e) => Err(e),
        }
    }

    /// Gets the list of open characters, which are the characters nobody can select this round.
    pub fn open_characters(&self) -> &[Character] {
        self.characters.open_characters()
    }

    /// Gets a list of player ids that represent the order each player's turn is in. The chairman
    /// id will always be the first id in this list, and ids will then count upward and loop back
    /// around if necessary.
    pub fn turn_order(&self) -> Vec<PlayerId> {
        let start = usize::from(self.chairman) as u8;
        let limit = self.players.len() as u8;
        (start..limit).chain(0..start).map(Into::into).collect()
    }

    /// Get the current market
    pub fn current_market(&self) -> &Market {
        &self.current_market
    }

    /// Gets the [`PlayerInfo`] for each player, excluding the player that has the same id as `id`.
    pub fn player_info(&self, id: PlayerId) -> Vec<PlayerInfo> {
        self.players()
            .iter()
            .filter(|p| p.id() != id)
            .map(Into::into)
            .collect()
    }
}
