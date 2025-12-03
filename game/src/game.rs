//! This is where the game logic, excluding the player-specific logic, is located.

use either::Either;
use serde::{Deserialize, Serialize};

use std::{collections::HashSet, path::Path, sync::Arc, vec};

use crate::{cards::GameData, errors::*, player::*, utility::serde_asset_liability};

/// Cash each player starts with
pub const STARTING_GOLD: u8 = 1;

/// Amount of assets required to end the game
pub const ASSETS_FOR_END_OF_GAME: usize = 6;

/// The event card type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    /// The title of the event
    pub title: String,
    /// A narration of the event which describes what happens
    pub description: String,
    /// A set of colors that gain gold because of this event
    pub plus_gold: HashSet<Color>,
    /// A set of colors that lose gold because of this event
    pub minus_gold: HashSet<Color>,
    /// A character that skips their turn because of this event
    pub skip_turn: Option<Character>,
}

/// A representation of the market condition for a specific color. It can either be
/// 1. Up: (+)
/// 2. Zero: ( )
/// 3. Minus: (-)
///
/// NOTE: The default state is `Zero`, which is also the case when parsing with serde.
#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum MarketCondition {
    /// The market for this color is up
    #[serde(rename = "up")]
    Plus,
    /// The market for this color is down
    #[serde(rename = "down")]
    Minus,
    /// The market for this color is neutral
    #[default]
    #[serde(other)]
    Zero,
}

/// The market card type
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Market {
    /// The title of the market
    pub title: String,
    /// The rfr value of the market
    pub rfr: u8,
    /// The mrp value of the market
    pub mrp: u8,
    /// The market condition for yellow
    #[serde(rename = "Yellow", default)]
    pub yellow: MarketCondition,
    /// The market condition for blue
    #[serde(rename = "Blue", default)]
    pub blue: MarketCondition,
    /// The market condition for green
    #[serde(rename = "Green", default)]
    pub green: MarketCondition,
    /// The market condition for purple
    #[serde(rename = "Purple", default)]
    pub purple: MarketCondition,
    /// The market condition for red
    #[serde(rename = "Red", default)]
    pub red: MarketCondition,
    /// A url which points to the front of this market card in the assets folder
    pub image_front_url: String,
    /// A url which points to the back of a market card in the assets folder
    pub image_back_url: Arc<String>,
}

impl Market {
    /// Gets the market condition for a specific color
    pub fn color_condition(&self, color: Color) -> MarketCondition {
        match color {
            Color::Red => self.red,
            Color::Green => self.green,
            Color::Purple => self.purple,
            Color::Yellow => self.yellow,
            Color::Blue => self.blue,
        }
    }
}

fn default_backup_deck<T>() -> Box<[T]> {
    Box::new([])
}

/// A wrapper struct around `Vec<T>` which allows for easy interaction with it as a deck of cards.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Deck<T> {
    /// The back url of the particular deck
    #[serde(rename = "card_image_back_url")]
    pub image_back_url: Arc<String>,
    /// The list of actual cards
    #[serde(rename = "card_list")]
    pub deck: Vec<T>,
    #[serde(skip, default = "default_backup_deck")]
    backup_deck: Box<[T]>,
}

impl<T: Clone> Deck<T> {
    /// Creates a new `Deck<T>` based on a `Vec<T>`
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::game::Deck;
    /// let deck = Deck::new(vec![1, 2, 3]);
    /// assert_eq!(deck.deck, [1, 2, 3]);
    /// ```
    pub fn new(deck: Vec<T>) -> Self {
        let backup_deck = deck.clone().into_boxed_slice();
        Self {
            deck,
            backup_deck,
            image_back_url: String::new().into(),
        }
    }

    /// Creates a new `Deck<T>` based on a `Vec<T>` and an url which should point to the back of
    /// the deck's cards in the asset folder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::game::Deck;
    /// let url = "assets/cards/card_back.svg";
    ///
    /// let deck = Deck::new_with_url(vec![1, 2, 3], url);
    /// assert_eq!(deck.deck, [1, 2, 3]);
    /// assert_eq!(deck.image_back_url.as_str(), url);
    /// ```
    pub fn new_with_url(deck: Vec<T>, url: &str) -> Self {
        let mut deck = Self::new(deck);
        deck.image_back_url = Arc::new(url.to_owned());
        deck
    }

    /// Draws a new card from the deck. If the deck ran out it is restored from the backup deck,
    /// reshuffled and then a card is drawn from that new deck instead.
    pub fn draw(&mut self) -> T {
        match self.deck.pop() {
            Some(card) => card,
            None => {
                self.deck = self.backup_deck.to_vec();

                #[cfg(feature = "shuffle")]
                self.shuffle();

                // TODO: maybe fix for if the deck was empty when initialized, because in that case
                // it still crashes. This isn't a concern for our game though and I prefer to not
                // return `Option` here.
                self.deck.pop().unwrap()
            }
        }
    }
}

impl<T> Deck<T> {
    /// Returns the number of elements in the deck, also referred to as its 'length'.
    pub fn len(&self) -> usize {
        self.deck.len()
    }

    /// Returns true if the deck contains no elements.
    pub fn is_empty(&self) -> bool {
        self.deck.is_empty()
    }

    /// Sets the card url of the back image of the cards in the deck.
    pub fn set_image_back_url(&mut self, url: &str) {
        self.image_back_url = Arc::new(url.to_owned());
    }

    /// Puts back a card on the bottom of the deck
    pub fn put_back(&mut self, card: T) {
        self.deck.insert(0, card);
    }

    /// Randomly reshuffles the deck
    #[cfg(feature = "shuffle")]
    pub fn shuffle(&mut self) {
        use rand::seq::SliceRandom;

        let mut rng = rand::rng();
        self.deck.shuffle(&mut rng);
    }
}

impl<T> Default for Deck<T> {
    fn default() -> Self {
        Self {
            deck: Default::default(),
            backup_deck: Default::default(),
            image_back_url: Default::default(),
        }
    }
}

/// Contains information when picking cards. One gets a list of pickable characters as
/// well as a possible closed character if the player requesting it is the chairman.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickableCharacters {
    /// List of pickable characters
    characters: Vec<Character>,
    /// Possible closed character only shown to the chairman
    closed_character: Option<Character>,
}

/// Used for keeping track of selectable characters in the selecting characters phase.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObtainingCharacters {
    /// The amount of players in the game
    player_count: usize,
    /// The index of the next player who should draw
    draw_idx: usize,
    /// The id of the chairman represented as `usize`
    chairman_id: usize,
    /// A deck containing all available characters
    available_characters: Deck<Character>,
    /// A list of open characters, the length of which depends on how many players are in the game
    open_characters: Vec<Character>,
    /// The closed character
    closed_character: Character,
}

impl ObtainingCharacters {
    /// Creates a new instance based on the player count and the chairman id.
    pub fn new(player_count: usize, chairman_id: PlayerId) -> Result<Self, GameError> {
        let open_character_count = match player_count {
            4 => 2,
            5 => 1,
            6 | 7 => 0,
            c => return Err(GameError::InvalidPlayerCount(c as u8)),
        };

        let mut available_characters = Deck::new(Character::CHARACTERS.to_vec());
        #[cfg(feature = "shuffle")]
        {
            available_characters.shuffle();

            let ceo_pos = available_characters
                .deck
                .iter()
                .position(|c| *c == Character::CEO)
                .unwrap();

            // Get CEO out of the first `open_character_count` positions
            if (0..open_character_count).contains(&ceo_pos) {
                let ceo_insert =
                    rand::random_range(open_character_count..(available_characters.len() - 1));
                debug_assert_eq!(available_characters.deck.remove(ceo_pos), Character::CEO);
                available_characters.deck.insert(ceo_insert, Character::CEO);
            }
            // CEO is now out of bottom positions of the deck (start of list) but we want it out
            // of the top of the deck (end of list)
            available_characters.deck.reverse();
        }

        let open_characters = (0..open_character_count)
            .map(|_| available_characters.draw())
            .collect();
        let closed_character = available_characters.draw();

        Ok(ObtainingCharacters {
            player_count,
            draw_idx: 0,
            chairman_id: chairman_id.into(),
            available_characters,
            open_characters,
            closed_character,
        })
    }

    /// Looks one step ahead and gets the next instance of `PickableCharacters`. This may error if
    /// every player has selected a character
    pub fn peek(&self) -> Result<PickableCharacters, SelectingCharactersError> {
        match self.draw_idx {
            0 => Ok(PickableCharacters {
                characters: self.available_characters.deck.to_vec(),
                closed_character: Some(self.closed_character),
            }),
            n if n < self.player_count - 1 => Ok(PickableCharacters {
                characters: self.available_characters.deck.to_vec(),
                closed_character: None,
            }),
            n if n == self.player_count - 1 => Ok(PickableCharacters {
                characters: self
                    .available_characters
                    .deck
                    .iter()
                    .cloned()
                    .chain([self.closed_character])
                    .collect(),
                closed_character: None,
            }),
            _ => Err(SelectingCharactersError::NotPickingCharacters),
        }
    }

    /// Attempts to select a character for a particular player.
    pub fn pick(&mut self, character: Character) -> Result<(), SelectingCharactersError> {
        match self.peek() {
            Ok(PickableCharacters { mut characters, .. }) => {
                match characters.iter().position(|&c| c == character) {
                    Some(i) => {
                        characters.remove(i);
                        self.draw_idx += 1;
                        self.available_characters.deck = characters;
                        Ok(())
                    }
                    None => Err(SelectingCharactersError::UnavailableCharacter),
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Gets the index of the currently selecting player
    pub fn applies_to_player(&self) -> usize {
        (self.draw_idx + self.chairman_id) % self.player_count
    }

    /// Gets a list of open characters
    pub fn open_characters(&self) -> &[Character] {
        &self.open_characters
    }
}

/// Data used when someone buys a new asset and a market change is triggered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketChange {
    /// A list of events encountered in search for a market card
    pub events: Vec<Event>,
    /// The new market card
    pub new_market: Market,
}

/// Data used when someone plays a card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerPlayedCard {
    /// The market change data if the market actually changes
    pub market: Option<MarketChange>,
    /// The card that was played
    #[serde(with = "serde_asset_liability::value")]
    pub used_card: Either<Asset, Liability>,
}

/// Data used when a turn ends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnEnded {
    /// The next player, if they exist
    pub next_player: Option<PlayerId>,
    /// Whether or not the game has ended
    pub game_ended: bool,
}

/// Wrapper struct around `Vec<P>` to make interacting with them as players internally much easier.
#[derive(Debug, Clone, PartialEq)]
pub struct Players<P>(Vec<P>);

impl<P> Players<P> {
    /// Create a new `Players<P>` based on a `Vec<P>`
    ///
    /// Examples
    ///
    /// ```
    /// # use game::game::Players;
    /// let p: Players<u8> = Players::new(vec![1, 2, 3]);
    pub fn new(players: Vec<P>) -> Self {
        Self(players)
    }

    /// Returns the number of players in the list, also referred to as its 'length'.
    ///
    /// Examples
    ///
    /// ```
    /// # use game::game::Players;
    /// let p = Players::new(vec![1, 2, 3]);
    /// assert_eq!(p.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if the list contains no elements.
    ///
    /// Examples
    ///
    /// ```
    /// # use game::game::Players;
    /// let p: Players<u8> = Players::default();
    /// assert!(p.is_empty());
    ///
    /// let p2 = Players::new(vec![1]);
    /// assert!(!p2.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get a reference to a player based on a specific `PlayerId`. Note that the players are in
    /// order, so id 0 refers to the player at index 0 and so on.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::{game::Players, player::PlayerId};
    /// let players = Players::new(vec![1, 2, 3]);
    /// let id = PlayerId(2);
    ///
    /// let player = players.player(id);
    /// assert_eq!(player, Ok(&3));
    /// ```
    pub fn player(&self, id: PlayerId) -> Result<&P, GameError> {
        self.0
            .get(usize::from(id))
            .ok_or(GameError::InvalidPlayerIndex(id.0))
    }

    /// Get a mutable reference to a player based on a specific `PlayerId`. Note that the players
    /// are in order, so id 0 refers to the player at index 0 and so on.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::{game::Players, player::PlayerId};
    /// let mut players = Players::new(vec![1, 2, 3]);
    /// let id = PlayerId(2);
    ///
    /// if let Ok(mut player) = players.player_mut(id) {
    ///     *player = 10;
    /// }
    /// assert_eq!(players, Players::new(vec![1, 2, 10]));
    /// ```
    pub fn player_mut(&mut self, id: PlayerId) -> Result<&mut P, GameError> {
        self.0
            .get_mut(usize::from(id))
            .ok_or(GameError::InvalidPlayerIndex(id.0))
    }

    /// Gets a slice of all players in the list
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::game::Players;
    /// let players = Players::new(vec![1, 2, 3]);
    /// assert_eq!(players.players(), &[1, 2, 3]);
    pub fn players(&self) -> &[P] {
        &self.0
    }

    /// Gets a mutable slice of all players in the list
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::game::Players;
    /// let mut players = Players::new(vec![1, 2, 3]);
    /// let refs = players.players_mut();
    /// refs[2] = 10;
    /// assert_eq!(players.players_mut(), &[1, 2, 10]);
    pub fn players_mut(&mut self) -> &mut [P] {
        &mut self.0
    }

    /// Wrapper around `slice::get_disjoint_mut()` which returns mutable references to many indices
    /// at once.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::game::Players;
    /// let mut players = Players::new(vec![1, 2, 3]);
    /// if let Ok([a, b]) = players.get_disjoint_mut([1, 2]) {
    ///     *a = 10;
    ///     *b = 20;
    /// }
    /// assert_eq!(players, Players::new(vec![1, 10, 20]));
    /// ```
    pub fn get_disjoint_mut<const N: usize>(
        &mut self,
        indices: [usize; N],
    ) -> Result<[&mut P; N], std::slice::GetDisjointMutError> {
        self.0.get_disjoint_mut(indices)
    }
}

impl<P> Default for Players<P> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// The core state representation of The Bottom Line.
/// It has four internal states:
/// 1. Lobby  ([`Lobby`])
/// 2. Selecting Characters ([`SelectingCharacters`])
/// 3. Round ([`Round`])
/// 4. Results ([`Results`])
#[derive(Debug, Clone, PartialEq)]
pub enum GameState {
    /// Lobby state of the game. In this state players can freely join and leave the game
    Lobby(Lobby),
    /// Selecting characters state of the game. In this state each player can select a character,
    /// after which the game starts
    SelectingCharacters(SelectingCharacters),
    /// Round state of the game. In this state each player plays their turn, which includes drawing
    /// cards, playing assets and liabilities and using their character ability
    Round(Round),
    /// Results state of the game. In this state players can see how they did compared to everyone
    /// else
    Results(Results),
}

impl GameState {
    /// Creates a new instance of the game. The game starts in lobby state.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::game::{GameState, Lobby};
    /// let game = GameState::new();
    /// assert_eq!(game, GameState::Lobby(Lobby::default()));
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Tries to get a `&`[`Lobby`] state. Returns an error if the game is not in a lobby state.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::game::{GameState, Lobby};
    /// let game = GameState::Lobby(Lobby::default());
    /// assert_eq!(game.lobby(), Ok(&Lobby::default()));
    /// ```
    pub fn lobby(&self) -> Result<&Lobby, GameError> {
        match self {
            Self::Lobby(l) => Ok(l),
            _ => Err(GameError::NotLobbyState),
        }
    }

    /// Tries to get a `&mut`[`Lobby`] state. Returns an error if the game is not in a lobby state.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::game::{GameState, Lobby};
    /// let mut game = GameState::Lobby(Lobby::default());
    /// assert_eq!(game.lobby_mut(), Ok(&mut Lobby::default()));
    /// ```
    pub fn lobby_mut(&mut self) -> Result<&mut Lobby, GameError> {
        match self {
            Self::Lobby(l) => Ok(l),
            _ => Err(GameError::NotLobbyState),
        }
    }

    /// Tries to get a `&`[`SelectingCharacters`] state. Returns an error if the game is not in a
    /// selecting characters state.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::{errors::GameError, game::{GameState, Lobby}};
    /// let game = GameState::Lobby(Lobby::default());
    /// assert_eq!(game.selecting_characters(), Err(GameError::NotSelectingCharactersState));
    /// ```
    pub fn selecting_characters(&self) -> Result<&SelectingCharacters, GameError> {
        match self {
            Self::SelectingCharacters(s) => Ok(s),
            _ => Err(GameError::NotSelectingCharactersState),
        }
    }

    /// Tries to get a `&mut`[`SelectingCharacters`] state. Returns an error if the game is not in a
    /// selecting characters state.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::{errors::GameError, game::{GameState, Lobby}};
    /// let mut game = GameState::Lobby(Lobby::default());
    /// assert_eq!(game.selecting_characters_mut(), Err(GameError::NotSelectingCharactersState));
    /// ```
    pub fn selecting_characters_mut(&mut self) -> Result<&mut SelectingCharacters, GameError> {
        match self {
            Self::SelectingCharacters(s) => Ok(s),
            _ => Err(GameError::NotSelectingCharactersState),
        }
    }

    /// Tries to get a `&`[`Round`] state. Returns an error if the game is not in a round state.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::{errors::GameError, game::{GameState, Lobby}};
    /// let game = GameState::Lobby(Lobby::default());
    /// assert_eq!(game.round(), Err(GameError::NotRoundState));
    /// ```
    pub fn round(&self) -> Result<&Round, GameError> {
        match self {
            Self::Round(r) => Ok(r),
            _ => Err(GameError::NotRoundState),
        }
    }

    /// Tries to get a `&mut`[`Round`] state. Returns an error if the game is not in a round state.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::{errors::GameError, game::{GameState, Lobby}};
    /// let mut game = GameState::Lobby(Lobby::default());
    /// assert_eq!(game.round_mut(), Err(GameError::NotRoundState));
    /// ```
    pub fn round_mut(&mut self) -> Result<&mut Round, GameError> {
        match self {
            Self::Round(r) => Ok(r),
            _ => Err(GameError::NotRoundState),
        }
    }

    /// Tries to get a `&`[`Results`] state. Returns an error if the game is not in a results state.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::{errors::GameError, game::{GameState, Lobby}};
    /// let game = GameState::Lobby(Lobby::default());
    /// assert_eq!(game.results(), Err(GameError::NotResultsState));
    pub fn results(&self) -> Result<&Results, GameError> {
        match self {
            Self::Results(r) => Ok(r),
            _ => Err(GameError::NotResultsState),
        }
    }

    /// Tries to get a `&mut`[`Results`] state. Returns an error if the game is not in a results
    /// state.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::{errors::GameError, game::{GameState, Lobby}};
    /// let mut game = GameState::Lobby(Lobby::default());
    /// assert_eq!(game.results_mut(), Err(GameError::NotResultsState));
    pub fn results_mut(&mut self) -> Result<&mut Results, GameError> {
        match self {
            Self::Results(r) => Ok(r),
            _ => Err(GameError::NotResultsState),
        }
    }

    /// Starts the game if enough players are in the lobby. If the lobby has between 4 and 7 players
    /// inclusive, turns the state from a [`Lobby`] into a [`SelectingCharacters`]. Takes in a path
    /// that should point to an instance of [`boardgame.json`](crate::cards), which holds the
    /// information about what cards each deck should be filled with.
    pub fn start_game<P: AsRef<Path>>(&mut self, data_path: P) -> Result<(), GameError> {
        match self {
            Self::Lobby(lobby) => {
                *self = lobby.start_game(data_path)?;
                Ok(())
            }
            _ => Err(GameError::NotLobbyState),
        }
    }

    /// Allows a player with `id` to select `character` if that character is available. If this was
    /// the last player to select a character, the state will be transformed from
    /// [`SelectingCharacters`] to [`Round`]
    pub fn player_select_character(
        &mut self,
        id: PlayerId,
        character: Character,
    ) -> Result<(), GameError> {
        let selecting = self.selecting_characters_mut()?;

        if let Some(state) = selecting.player_select_character(id, character)? {
            *self = state;
        };

        Ok(())
    }

    /// Allows player with `id` to end their turn.
    /// If it was the last player in a round, transforms the internal state from [`Round`] back to
    /// [`SelectingCharacters`].
    /// If it was the last turn of the game, transforms the internal state from [`Round`] into
    /// [`Results`]
    pub fn end_player_turn(&mut self, id: PlayerId) -> Result<TurnEnded, GameError> {
        let round = self.round_mut()?;

        match round.end_player_turn(id)? {
            Either::Left(te) => Ok(te),
            Either::Right(state) => {
                *self = state;
                Ok(TurnEnded {
                    next_player: None,
                    game_ended: true,
                })
            }
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::Lobby(Lobby::default())
    }
}

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
    fn start_game<P: AsRef<Path>>(&mut self, data_path: P) -> Result<GameState, GameError> {
        if self.can_start() {
            let data = GameData::new(data_path).expect("Path for game data is invalid");

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
            let current_market =
                Lobby::initial_market(&mut markets).expect("No markets in deck for some reason");

            let chairman = players.players().first().unwrap().id();
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
            Some(pos) => markets.deck.swap_remove(pos).left(),
            _ => None,
        }
    }
}

/// State containing all information related to the selecting characters state of the game. In the
/// selecting characters stage, players select a character one by one until everyone has selected
/// a character, after which a round starts.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectingCharacters {
    players: Players<SelectingCharactersPlayer>,
    characters: ObtainingCharacters,
    assets: Deck<Asset>,
    liabilities: Deck<Liability>,
    markets: Deck<Either<Market, Event>>,
    chairman: PlayerId,
    current_market: Market,
    current_events: Vec<Event>,
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
    fn player_select_character(
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

                    let players = std::mem::take(&mut self.players);
                    let assets = std::mem::take(&mut self.assets);
                    let liabilities = std::mem::take(&mut self.liabilities);
                    let markets = std::mem::take(&mut self.markets);
                    let current_market = std::mem::take(&mut self.current_market);
                    let current_events = std::mem::take(&mut self.current_events);
                    let open_characters = self.characters.open_characters().to_vec();
                    let fired_characters: Vec<Character> = vec![];

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

/// State containing all information related to the round state of the game. In the round stage,
/// players each play a turn where they can draw cards, play cards and use their character ability.
/// After every player has played a turn, players will be able to select characters again. If one
/// player reached six or more assets during a round, the game will move to [`Results`] instead.
#[derive(Debug, Clone, PartialEq)]
pub struct Round {
    current_player: PlayerId,
    players: Players<RoundPlayer>,
    assets: Deck<Asset>,
    liabilities: Deck<Liability>,
    markets: Deck<Either<Market, Event>>,
    chairman: PlayerId,
    current_market: Market,
    current_events: Vec<Event>,
    open_characters: Vec<Character>,
    fired_characters: Vec<Character>,
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
                let market = match self.should_refresh_market(old_max_bought_assets) {
                    true => Some(self.refresh_market()),
                    false => None,
                };
                let used_card = Either::Left(asset.clone());
                Ok(PlayerPlayedCard { market, used_card })
            }
            Either::Right(liability) => {
                let market = None;
                let used_card = Either::Right(liability);
                Ok(PlayerPlayedCard { market, used_card })
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
    fn end_player_turn(&mut self, id: PlayerId) -> Result<Either<TurnEnded, GameState>, GameError> {
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
            } else if !self.is_last_round() {
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

                let players = Players(players.0.into_iter().map(Into::into).collect());

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
                let final_market = std::mem::take(&mut self.current_market);
                let final_events = std::mem::take(&mut self.current_events);
                let players = std::mem::take(&mut self.players);

                let players = Players(players.0.into_iter().map(Into::into).collect());

                let state = GameState::Results(Results {
                    players,
                    final_market,
                    final_events,
                });

                Ok(Either::Right(state))
            }
        } else {
            Err(GameError::PlayerShouldGiveBackCard)
        }
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

    /// Checks whether someone has bought equal to or more assets than [`ASSETS_FOR_END_OF_GAME`].
    /// If so, this should be the final round.
    fn is_last_round(&self) -> bool {
        self.max_bought_assets() >= ASSETS_FOR_END_OF_GAME
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

/// State containing all information related to the results state of the game. In the resuts stage,
/// players can see their scores.
#[derive(Debug, Clone, PartialEq)]
pub struct Results {
    players: Players<ResultsPlayer>,
    final_market: Market,
    // TODO: implement events
    final_events: Vec<Event>,
}

impl Results {
    /// Get a reference to a [`ResultsPlayer`] based on a specific `PlayerId`. Note that the players
    /// are in order, so id 0 refers to the player at index 0 and so on.
    /// See [`Players::player`] for further information
    pub fn player(&self, id: PlayerId) -> Result<&ResultsPlayer, GameError> {
        self.players.player(id)
    }

    /// Get a reference to a [`ResultsPlayer`] based on a specific `name`.
    pub fn player_by_name(&self, name: &str) -> Result<&ResultsPlayer, GameError> {
        self.players()
            .iter()
            .find(|p| p.name() == name)
            .ok_or_else(|| GameError::InvalidPlayerName(name.to_owned()))
    }

    /// Gets a slice of all players in the lobby.
    /// See [`Players::players`] for further information
    pub fn players(&self) -> &[ResultsPlayer] {
        self.players.players()
    }

    /// Gets the [`PlayerInfo`] for each player, excluding the player that has the same id as `id`.
    pub fn player_info(&self, id: PlayerId) -> Vec<PlayerInfo> {
        self.players()
            .iter()
            .filter(|p| p.id() != id)
            .map(Into::into)
            .collect()
    }

    /// Gets the final market of the game
    pub fn final_market(&self) -> &Market {
        &self.final_market
    }

    /// Gets the list of events that happened over the course of the game
    pub fn final_events(&self) -> &[Event] {
        &self.final_events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claim::*;
    use itertools::Itertools;

    #[test]
    fn all_unique_ids() {
        for i in 4..=7 {
            let game = pick_with_players(i).expect("couldn't pick characters");
            let round = game.round().unwrap();

            assert!(round.players().iter().map(|p| p.id()).all_unique());
        }
    }

    #[test]
    fn ids_sorted() {
        for i in 4..=7 {
            let game = pick_with_players(i).expect("couldn't pick characters");
            let round = game.round().unwrap();

            assert!(round.players().iter().map(|p| p.id()).is_sorted());
        }
    }

    #[test]
    fn player_from_character() {
        for i in 4..=7 {
            let game = pick_with_players(i).expect("couldn't pick characters");
            let round = game.round().unwrap();

            round
                .players()
                .iter()
                .map(|p| (p.character(), p.id()))
                .for_each(|(c, id)| {
                    let p = round
                        .player_from_character(c)
                        .expect("couldn't find character");

                    assert_eq!(p.id(), id);
                });
        }
    }

    #[test]
    fn player_by_name() {
        for i in 4..=7 {
            let game = pick_with_players(i).expect("couldn't pick characters");
            let round = game.round().unwrap();

            round
                .players()
                .iter()
                .map(|p| (p.name(), p.id()))
                .for_each(|(name, id)| {
                    let p = round.player_by_name(name).expect("couldn't find name");

                    assert_eq!(p.id(), id);
                });
        }
    }

    #[test]
    fn player_draw_card() {
        for i in 4..=7 {
            // All permutations of a list of 3 card types
            std::iter::repeat_n([CardType::Asset, CardType::Liability].into_iter(), 4)
                .multi_cartesian_product()
                .map(|v| ([v[0], v[1], v[2]], v[3]))
                .for_each(|(card_types, too_many)| {
                    let mut game = pick_with_players(i).expect("couldn't pick characters");
                    let round = game.round_mut().expect("Game not in round state");
                    let current_player = round.current_player().id();

                    // For some reason never picks head of rnd
                    assert_ne!(round.current_player().character(), Character::HeadRnD);

                    card_types.into_iter().for_each(|card_type| {
                        assert_ok!(round.player_draw_card(current_player, card_type));
                    });

                    assert_matches!(
                        round.player_draw_card(current_player, too_many),
                        Err(GameError::DrawCard(DrawCardError::MaximumCardsDrawn(_)))
                    );
                });
        }
    }

    #[test]
    fn player_draw_card_invalid_id() {
        let mut game = pick_with_players(4).expect("couldn't pick characters");
        let round = game.round_mut().expect("not in round state");

        assert_matches!(
            round.player_draw_card(u8::MAX.into(), CardType::Asset),
            Err(GameError::InvalidPlayerIndex(_))
        );
    }

    #[test]
    fn player_draw_card_not_turn() {
        let mut game = pick_with_players(4).expect("couldn't pick characters");
        let round = game.round_mut().expect("not in round state");
        // This is not the current player
        let next_player = round.next_player().expect("couldn't get next player");

        assert_matches!(
            round.player_draw_card(next_player.id(), CardType::Asset),
            Err(GameError::NotPlayersTurn)
        )
    }

    #[test]
    fn ceo_not_in_open_characters() {
        // Since we're testing with random values, get large enough sample to where CEO has a
        // (1 - 1.1554035912766488e-128) chance of showing up among the open cards
        for i in 0..1024 {
            for player_count in 4..=7 {
                let characters = ObtainingCharacters::new(player_count, PlayerId(0))
                    .expect("couldn't init ObtainingCharacters");

                assert!(
                    !characters.open_characters().contains(&Character::CEO),
                    "{i}"
                );
            }
        }
    }

    #[test]
    fn player_play_card() {
        for i in 4..=7 {
            let mut game = pick_with_players(i).expect("couldn't pick characters");
            let round = game.round_mut().expect("Game not in round state");

            let current_player = round.current_player().id();

            draw_cards(
                round,
                current_player,
                [CardType::Asset, CardType::Asset, CardType::Liability],
            );

            // so player can always afford the asset
            round.player_mut(current_player).unwrap()._set_cash(50);

            // test issuing liability
            let player = &round.player(current_player).unwrap();
            let hand_len = player.hand().len();
            let liability_value = player.hand()[hand_len - 1]
                .as_ref()
                .right()
                .expect("Couldn't get liability")
                .value;
            let cash_before = player.cash();

            assert_ok!(round.player_play_card(current_player, hand_len - 1));
            assert_eq!(
                cash_before + liability_value,
                round.player(current_player).unwrap().cash()
            );

            assert_eq!(
                hand_len - 1,
                round.player(current_player).unwrap().hand().len()
            );

            // test buying asset
            let player = &round.player(current_player).unwrap();
            let hand_len = player.hand().len();
            let liability_value = player.hand()[hand_len - 1]
                .as_ref()
                .left()
                .expect("Couldn't get asset")
                .gold_value;
            let cash_before = player.cash();

            assert_ok!(round.player_play_card(current_player, hand_len - 1));
            assert_eq!(
                cash_before - liability_value,
                round.player(current_player).unwrap().cash()
            );

            assert_eq!(
                hand_len - 1,
                round.player(current_player).unwrap().hand().len()
            );

            let player = round.player(current_player).unwrap();

            if player.character() == Character::CSO
                && [Color::Red, Color::Green].contains(&player.assets()[0].color)
            {
                panic!("Not testing for this yet");
            }

            // Set assets to play to 0 to not fail the test when CEO is picked
            let player = round.player_mut(current_player).unwrap();
            if player.character() == Character::CEO {
                return;
                // player.assets_to_play() = 0;
            }

            let hand_len = player.hand().len();
            assert_matches!(
                round.player_play_card(current_player, hand_len - 1),
                Err(GameError::PlayCard(PlayCardError::ExceedsMaximumAssets))
            );
            assert_matches!(
                round.player_play_card(current_player, hand_len - 2),
                // Assumes a starter hand has 2 assets and then 2 liabilities
                Err(GameError::PlayCard(
                    PlayCardError::ExceedsMaximumLiabilities
                ))
            );
        }
    }

    #[test]
    fn player_play_card_invalid_id() {
        let mut game = pick_with_players(4).expect("couldn't pick characters");
        let round = game.round_mut().expect("not in round state");

        assert_matches!(
            round.player_play_card(u8::MAX.into(), 0),
            Err(GameError::InvalidPlayerIndex(_))
        )
    }

    #[test]
    fn player_play_card_not_turn() {
        let mut game = pick_with_players(4).expect("couldn't pick characters");
        let round = game.round_mut().expect("Game not in round state");

        // This is not the current player
        let next_player = round.next_player().expect("couldn't get next player");

        assert_matches!(
            round.player_play_card(next_player.id(), 0),
            Err(GameError::NotPlayersTurn)
        )
    }

    #[test]
    fn end_player_turn_no_actions() {
        let mut game = pick_with_players(4).expect("couldn't pick characters");
        let round = game.round().expect("Game not in round state");

        let current_player = round.current_player().id();

        assert_ok!(game.end_player_turn(current_player));
    }

    #[test]
    fn end_player_turn_used_cards() {
        let mut game = pick_with_players(4).expect("couldn't pick characters");
        let round = game.round_mut().expect("not in round state");

        let current_player = round.current_player().id();

        // so player can always afford the asset
        round.player_mut(current_player).unwrap()._set_cash(50);

        let hand_len = round.player(current_player).unwrap().hand().len();
        assert_ok!(round.player_play_card(current_player, hand_len - 1));
        assert_ok!(round.player_play_card(current_player, 0));

        assert_ok!(game.end_player_turn(current_player));
    }

    #[test]
    fn end_player_turn_drew_three_cards() {
        let mut game = pick_with_players(4).expect("couldn't pick characters");
        let round = game.round().expect("Game not in round state");

        let current_player = round.current_player().id();

        play_turn(&mut game, current_player)
    }

    #[test]
    fn play_rounds() {
        for player_count in 4..=7 {
            let mut game = pick_with_players(player_count).expect("couldn't pick characters");

            // nr of rounds
            // with current strategy runs out of liabilities after 5 rounds
            for _ in 0..5 {
                for _ in 0..player_count {
                    let round = game.round().expect("Game not in round state");

                    let current_player = round.current_player().id();

                    play_turn(&mut game, current_player);
                }

                assert_matches!(game, GameState::SelectingCharacters(_));

                finish_selecting_characters(&mut game);

                assert_matches!(game, GameState::Round(_));
            }
        }
    }

    #[test]
    fn pick_characters() {
        for i in 0..=3 {
            assert_matches!(
                pick_with_players(i),
                Err(GameError::InvalidPlayerCount(n)) if n == i as u8
            );
        }
        assert_ok!(pick_with_players(4));
        assert_ok!(pick_with_players(5));
        assert_ok!(pick_with_players(6));
        assert_ok!(pick_with_players(7));
        for i in 8..=25 {
            assert_matches!(
                pick_with_players(i),
                Err(GameError::InvalidPlayerCount(n)) if n == i as u8
            );
        }
    }

    fn play_turn(game: &mut GameState, player_id: PlayerId) {
        let round = game.round_mut().expect("not in round state");
        draw_cards(
            round,
            player_id,
            [CardType::Asset, CardType::Liability, CardType::Asset],
        );

        assert_err!(game.end_player_turn(player_id));

        let round = game.round_mut().expect("not in round state");
        let hand_len = round.player(player_id).unwrap().hand().len();
        assert_ok!(round.player_give_back_card(player_id, hand_len - 1));

        assert_ok!(game.end_player_turn(player_id));
    }

    fn draw_cards<const N: usize>(round: &mut Round, id: PlayerId, cards: [CardType; N]) {
        for card_type in cards {
            let _ = round.player_draw_card(id, card_type);
        }
    }

    fn finish_selecting_characters(game: &mut GameState) {
        let player_count = game.selecting_characters().unwrap().players.len();

        let add = match player_count {
            4..=6 => 1,
            7 => 0,
            _ => unreachable!(),
        };

        #[allow(unused)]
        let mut closed = None::<Character>;

        let chairman = game.selecting_characters().unwrap().chairman;
        let turn_order = game.selecting_characters().unwrap().turn_order();

        assert_eq!(chairman, turn_order[0]);

        let selecting = game
            .selecting_characters()
            .expect("game not in selecting phase");
        match selecting.player_get_selectable_characters(chairman) {
            Ok(characters) => {
                let closed_character = selecting.player_get_closed_character(chairman);
                assert_eq!(characters.len(), player_count + add);
                assert_ok!(closed_character);
                assert_ok!(game.player_select_character(chairman, characters[0]));

                closed = closed_character.ok();
            }
            _ => panic!(),
        }

        #[allow(clippy::needless_range_loop)]
        for i in 1..(player_count - 1) {
            let player = turn_order[i];
            let selecting = game
                .selecting_characters()
                .expect("game not in selecting phase");

            match selecting.player_get_selectable_characters(player) {
                Ok(characters) => {
                    assert_eq!(characters.len(), player_count + add - i);
                    assert_err!(selecting.player_get_closed_character(player));
                    assert_ok!(game.player_select_character(player, characters[0]));
                }
                _ => panic!(),
            }
        }

        let selecting = game
            .selecting_characters()
            .expect("game not in selecting phase");
        match selecting.player_get_selectable_characters(turn_order[player_count - 1]) {
            Ok(characters) => {
                assert_eq!(characters.len(), 2 + add);
                assert_err!(selecting.player_get_closed_character(turn_order[player_count - 1]));
                assert!(characters.contains(&closed.unwrap()));
                assert_ok!(
                    game.player_select_character(turn_order[player_count - 1], closed.unwrap())
                );

                assert_matches!(game, GameState::Round(_));
                assert_ok!(game.round());
            }
            _ => panic!(),
        }
    }

    fn pick_with_players(player_count: usize) -> Result<GameState, GameError> {
        let mut game = GameState::new();
        let lobby = game.lobby_mut().expect("game not in lobby state");

        (0..(player_count as u8))
            .map(|i| (i, format!("Player {i}")))
            .for_each(
                |(i, name)| assert_matches!(lobby.join(name), Ok(p) if p.id() == PlayerId(i)),
            );

        game.start_game("../assets/cards/boardgame.json")?;

        assert_matches!(game, GameState::SelectingCharacters(_));
        assert_eq!(
            game.selecting_characters().unwrap().players.len(),
            player_count
        );

        finish_selecting_characters(&mut game);

        Ok(game)
    }
}
