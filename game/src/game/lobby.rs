//! File containing the lobby state of the game.

use std::path::Path;

use either::Either;

use crate::{cards::GameData, errors::*, game::*, player::*};

/// Cash each player starts with
pub const STARTING_GOLD: u8 = 1;

/// State containing all information related to the lobby stage of the game. In the lobby state,
/// players are allowed to join and leave freely. When between 4 to 7 players are in the lobby,
/// players are allowed to start a game.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Lobby {
    /// The players in the lobby
    players: Players<LobbyPlayer>,
}

impl Lobby {
    /// Instantiates a new lobby. This will be an empty lobby with no players in it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::game::Lobby;
    /// let lobby = Lobby::new();
    /// assert_eq!(lobby, Lobby::default());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of players in the lobby, also referred to as its 'length'.
    ///
    /// Examples
    ///
    /// ```
    /// # use game::{errors::GameError, game::Lobby};
    /// # fn main() -> Result<(), GameError> {
    /// let mut lobby = Lobby::default();
    /// assert_eq!(lobby.len(), 0);
    ///
    /// lobby.join("player 1".to_owned())?;
    /// assert_eq!(lobby.len(), 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn len(&self) -> usize {
        self.players.len()
    }

    /// Returns true if the lobby contains no players.
    ///
    /// Examples
    ///
    /// ```
    /// # use game::{errors::GameError, game::Lobby};
    /// # fn main() -> Result<(), GameError> {
    /// let mut lobby = Lobby::default();
    /// assert!(lobby.is_empty());
    ///
    /// lobby.join("player 1".to_owned())?;
    /// assert!(!lobby.is_empty());
    /// # Ok(())
    /// # }
    /// # use game::game::Players;
    /// ```
    pub fn is_empty(&self) -> bool {
        self.players.is_empty()
    }

    /// Get a reference to a [`LobbyPlayer`] based on a specific `PlayerId`. Note that the players
    /// are in order, so id 0 refers to the player at index 0 and so on.
    /// See [`Players::player`] for further information
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::{errors::GameError, game::Lobby, player::PlayerId};
    /// # fn main() -> Result<(), GameError> {
    /// let mut lobby = Lobby::default();
    /// let id = PlayerId(0);
    /// assert_eq!(lobby.player(id), None);
    ///
    /// lobby.join("player 1".to_owned())?;
    /// assert!(matches!(lobby.player(id), Some(_)));
    /// # Ok(())
    /// # }
    /// ```
    pub fn player(&self, id: PlayerId) -> Option<&LobbyPlayer> {
        self.players.player(id).ok()
    }

    /// Gets a slice of all players in the lobby
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::{errors::GameError, game::Lobby, player::{LobbyPlayer, PlayerId}};
    /// # fn main() -> Result<(), GameError> {
    /// let mut lobby = Lobby::default();
    ///
    /// lobby.join("player 1".to_owned())?;
    ///
    /// let player = LobbyPlayer::new(PlayerId(0), "player 1".to_owned());
    /// assert_eq!(lobby.players(), &[player]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn players(&self) -> &[LobbyPlayer] {
        self.players.players()
    }

    /// Gets a mutable slice of all players in the lobby
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::{errors::GameError, game::Lobby, player::{LobbyPlayer, PlayerId}};
    /// # fn main() -> Result<(), GameError> {
    /// let mut lobby = Lobby::default();
    ///
    /// lobby.join("player 1".to_owned())?;
    ///
    /// let player = LobbyPlayer::new(PlayerId(0), "player 1".to_owned());
    /// assert_eq!(lobby.players_mut(), &mut [player]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn players_mut(&mut self) -> &mut [LobbyPlayer] {
        self.players.players_mut()
    }

    /// Gets a list of usernames in the lobby. Note that this list has to be built every time this
    /// function is called.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::{errors::GameError, game::Lobby, player::{LobbyPlayer, PlayerId}};
    /// # fn main() -> Result<(), GameError> {
    /// let mut lobby = Lobby::default();
    /// lobby.join("player 1".to_owned())?;
    /// lobby.join("player 2".to_owned())?;
    ///
    /// assert_eq!(lobby.usernames(), vec!["player 1", "player 2"]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn usernames(&self) -> Vec<&str> {
        self.players().iter().map(|p| p.name()).collect()
    }

    /// Allows a player to join the lobby based on a username. If the username is not yet taken, the
    /// player is added to the list of players and a reference to it will be returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::{errors::GameError, game::Lobby, player::{LobbyPlayer, PlayerId}};
    /// # fn main() -> Result<(), GameError> {
    /// let mut lobby = Lobby::default();
    /// lobby.join("player 1".to_owned())?;
    /// lobby.join("player 2".to_owned())?;
    ///
    /// assert_eq!(lobby.usernames(), vec!["player 1", "player 2"]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn join(&mut self, username: String) -> Result<&LobbyPlayer, LobbyError> {
        match self.players().iter().find(|p| p.name() == username) {
            Some(_) => Err(LobbyError::UsernameAlreadyTaken(username)),
            None => {
                let id = PlayerId(self.players.len() as u8);
                let name = username.clone();
                let player = LobbyPlayer::new(id, name);

                self.players.0.push(player);
                Ok(&self.players.0[self.players.len() - 1])
            }
        }
    }

    /// Allows a player to leave the lobby based on their username. If that username is in the list,
    /// the player will be removed and `true` will be returned. If the player cannot be removed,
    /// the function will return `false` instead.
    ///
    /// NOTE: this function will reorder player ids if a player that is not at the end of the list
    /// leaves the lobby
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::{errors::GameError, game::Lobby, player::{LobbyPlayer, PlayerId}};
    /// # fn main() -> Result<(), GameError> {
    /// let mut lobby = Lobby::default();
    /// lobby.join("player 1".to_owned())?;
    /// lobby.join("player 2".to_owned())?;
    /// assert!(lobby.leave("player 1"));
    ///
    /// assert_eq!(lobby.usernames(), vec!["player 2"]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn leave(&mut self, username: &str) -> bool {
        match self.players().iter().position(|p| p.name() == username) {
            Some(pos) => {
                // PANIC: we just verified this is a valid position so removing here cannot crash.
                self.players.0.remove(pos);
                self.players_mut()
                    .iter_mut()
                    .zip(0u8..)
                    .for_each(|(p, id)| p.set_id(PlayerId(id)));
                true
            }
            None => false,
        }
    }

    /// Gets the [`PlayerInfo`] for each player, excluding the player
    /// that has the same id as `id`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::{errors::GameError, game::Lobby, player::{LobbyPlayer, PlayerId, PlayerInfo}};
    /// # fn main() -> Result<(), GameError> {
    /// let mut lobby = Lobby::default();
    /// lobby.join("player 1".to_owned())?;
    /// lobby.join("player 2".to_owned())?;
    ///
    /// let id = PlayerId(0);
    /// assert_eq!(lobby.player_info(id).len(), 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn player_info(&self, id: PlayerId) -> Vec<PlayerInfo> {
        self.players()
            .iter()
            .filter(|p| p.id() != id)
            .map(Into::into)
            .collect()
    }

    /// Checks whether or not the game can start. The game can start if the room has between 4 and
    /// 7 players.
    ///
    /// # Examples
    /// ```
    /// # use game::{errors::GameError, game::Lobby, player::{LobbyPlayer, PlayerId}};
    /// # fn main() -> Result<(), GameError> {
    /// let mut lobby = Lobby::default();
    ///
    /// (0..3).for_each(|i| { lobby.join(format!("player {i}")); });
    /// assert!(!lobby.can_start());
    ///
    /// lobby.join("player 3".to_owned())?;
    /// assert!(lobby.can_start());
    /// # Ok(())
    /// # }
    /// ```
    pub fn can_start(&self) -> bool {
        (4..=7).contains(&self.players.len())
    }

    /// Starts the game when between 4 to 7 players are in the lobby and potentially returns the new [`GameState`] if the game is started. Takes in `data_path`, which is meant to be a path
    /// that should point to an instance of [`boardgame.json`](crate::cards), which holds the
    /// information about what cards each deck should be filled with.
    pub(super) fn start_game<P: AsRef<Path>>(
        &mut self,
        data_path: P,
    ) -> Result<GameState, GameError> {
        if self.can_start() {
            let data = match GameData::new(&data_path) {
                Ok(data) => data,
                Err(crate::cards::DataParseError::Io(_)) => {
                    panic!(
                        "Path '{}' for game data is invalid",
                        data_path.as_ref().display()
                    )
                }
                Err(e) => panic!("{e}"),
            };

            #[cfg(feature = "shuffle")]
            let data = {
                let mut data = data;
                data.shuffle_all();
                data
            };

            let mut assets = data.assets;
            let mut liabilities = data.liabilities;
            let mut markets = data.market_deck;

            let players = self.init_players(&mut assets, &mut liabilities);
            let current_market = Lobby::initial_market(&mut markets).unwrap_or_default();

            let chairman = players
                .players()
                .first()
                .ok_or(GameError::InvalidPlayerCount(players.len() as u8))?
                .id();
            debug_assert_eq!(chairman, PlayerId(0));

            let characters = ObtainingCharacters::new(players.len(), chairman)?;

            let selecting = GameState::SelectingCharacters(SelectingCharacters {
                players,
                characters,
                assets,
                liabilities,
                markets,
                chairman,
                current_market,
                current_events: Vec::new(),
            });

            Ok(selecting)
        } else {
            Err(GameError::InvalidPlayerCount(self.players().len() as u8))
        }
    }

    /// Initializes [`SelectingCharactersPlayer`](crate::player::SelectingCharactersPlayer) with
    /// their appropriate starting gold and their initial hand.
    fn init_players(
        &mut self,
        assets: &mut Deck<Asset>,
        liabilities: &mut Deck<Liability>,
    ) -> Players<SelectingCharactersPlayer> {
        self.players.0.sort_by_key(|p| p.id());

        let players = self
            .players()
            .iter()
            .map(|p| {
                let assets = [assets.draw(), assets.draw()];
                let liabilities = [liabilities.draw(), liabilities.draw()];
                SelectingCharactersPlayer::new(
                    p.name().to_owned(),
                    p.id(),
                    assets,
                    liabilities,
                    STARTING_GOLD,
                )
            })
            .collect();

        Players(players)
    }

    /// Grab market card if available. If no market cards are in the deck, `None` is returned.
    fn initial_market(markets: &mut Deck<Either<Market, Event>>) -> Option<Market> {
        match markets.deck.iter().position(|c| c.is_left()) {
            // PANIC: we verified that this is a valid position, so removing it cannot crash.
            Some(pos) => markets.deck.swap_remove(pos).left(),
            _ => None,
        }
    }
}
