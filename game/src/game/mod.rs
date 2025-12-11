//! This is where the game logic, excluding the player-specific logic, is located.

mod lobby;
mod results;
mod round;
mod selecting_characters;

pub use lobby::*;
pub use results::*;
pub use round::*;
pub use selecting_characters::*;

use either::Either;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use std::{collections::HashSet, path::Path, sync::Arc, vec};

use crate::{errors::*, player::*, utility::serde_asset_liability};

/// Amount of assets required to end the game
pub const ASSETS_FOR_END_OF_GAME: usize = 6;

/// The event card type
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(rename = "EventCard"))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
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
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
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
    #[serde(rename = "zero")]
    Zero,
}

impl MarketCondition {
    /// Makes into a higher market condition:
    /// `Plus` and `Zero` become `Plus`, `Minus` becomes `Zero.
    pub fn make_higher(&mut self) {
        *self = match self {
            Self::Plus | Self::Zero => Self::Plus,
            Self::Minus => Self::Zero,
        };
    }

    /// Makes into a lower market condition:
    /// `Zero` and `Minus` become `Minus`, `Plus` becomes `Zero.
    pub fn make_lower(&mut self) {
        *self = match self {
            Self::Minus | Self::Zero => Self::Minus,
            Self::Plus => Self::Zero,
        };
    }
}

/// The market card type
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(rename = "MarketCard"))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
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
    /// A backup of the deck, which is set when the deck is created.
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
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
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
            // PANIC: this is completely safe because `Character::CHARACTERS always contains all
            // characters, which of course includes the CEO.

            // Get CEO out of the first `open_character_count` positions
            if (0..open_character_count).contains(&ceo_pos) {
                let ceo_insert =
                    rand::random_range(open_character_count..(available_characters.len() - 1));
                // PANIC: We know `ceo_pos` to be a valid position, so removing it cannot crash.
                assert_eq!(available_characters.deck.remove(ceo_pos), Character::CEO);
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
                        // PANIC: we know `i` to be a valid position, so removing it cannot crash.
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
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
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

    /// Returns an iterator over the slice.
    /// The iterator yields all players from start to end.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::game::Players;
    /// let players = Players::new(vec![1, 2, 4]);
    /// let mut iterator = players.iter();
    ///
    /// assert_eq!(iterator.next(), Some(&1));
    /// assert_eq!(iterator.next(), Some(&2));
    /// assert_eq!(iterator.next(), Some(&4));
    /// assert_eq!(iterator.next(), None);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &P> {
        self.0.iter()
    }
}

impl<P> Default for Players<P> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<P> IntoIterator for Players<P> {
    type Item = P;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
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
