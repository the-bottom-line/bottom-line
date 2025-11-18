use either::Either;
use serde::{Deserialize, Serialize};

use std::{collections::HashSet, path::Path, sync::Arc, vec};

use crate::{cards::GameData, errors::*, player::*, utility::serde_asset_liability};

pub const STARTING_GOLD: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub title: String,
    pub description: String,
    pub plus_gold: HashSet<Color>,
    pub minus_gold: HashSet<Color>,
    pub skip_turn: Option<Character>,
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize)]
pub enum MarketCondition {
    #[serde(rename = "up")]
    Plus,
    #[serde(rename = "down")]
    Minus,
    #[default]
    #[serde(other)]
    Zero,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Market {
    pub title: String,
    pub rfr: u8,
    pub mrp: u8,
    #[serde(rename = "Yellow", default)]
    pub yellow: MarketCondition,
    #[serde(rename = "Blue", default)]
    pub blue: MarketCondition,
    #[serde(rename = "Green", default)]
    pub green: MarketCondition,
    #[serde(rename = "Purple", default)]
    pub purple: MarketCondition,
    #[serde(rename = "Red", default)]
    pub red: MarketCondition,
    pub image_front_url: String,
    pub image_back_url: Arc<String>,
}

impl Market {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck<T> {
    #[serde(rename = "card_image_back_url")]
    pub image_back_url: Arc<String>,
    #[serde(rename = "card_list")]
    pub deck: Vec<T>,
}

impl<T> Deck<T> {
    pub fn new(deck: Vec<T>) -> Self {
        Self {
            deck,
            image_back_url: String::new().into(),
        }
    }

    pub fn len(&self) -> usize {
        self.deck.len()
    }

    pub fn is_empty(&self) -> bool {
        self.deck.is_empty()
    }

    // TODO: think of way to make this not unwrap. Maybe keep a copy of the deck as backup to
    // reshuffle?
    /// Panics if no more cards are in the deck, for now. Decks don't run out in regular games.
    /// NOTE: Playing 5 rounds with 7 players where each player draws one liability per turn comes
    /// out to 49 out of 50 cards
    pub fn draw(&mut self) -> T {
        self.deck.pop().unwrap()
    }

    pub fn put_back(&mut self, card: T) {
        self.deck.insert(0, card);
    }

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
            image_back_url: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickableCharacters {
    characters: Vec<Character>,
    closed_character: Option<Character>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObtainingCharacters {
    player_count: usize,
    draw_idx: usize,
    chairman_id: usize,
    available_characters: Deck<Character>,
    open_characters: Vec<Character>,
    closed_character: Character,
}

impl ObtainingCharacters {
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

    pub fn applies_to_player(&self) -> usize {
        (self.draw_idx + self.chairman_id) % self.player_count
    }

    pub fn open_characters(&self) -> &[Character] {
        &self.open_characters
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketChange {
    pub events: Vec<Event>,
    pub new_market: Market,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerPlayedCard {
    pub market: Option<MarketChange>,
    #[serde(with = "serde_asset_liability::value")]
    pub used_card: Either<Asset, Liability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnEnded {
    pub next_player: Option<PlayerId>,
}

impl TurnEnded {
    pub fn new(next_player: Option<PlayerId>) -> Self {
        Self { next_player }
    }
}

#[derive(Debug, Clone)]
pub struct Players<P>(Vec<P>);

impl<P> Players<P> {
    pub fn new(players: Vec<P>) -> Self {
        Self(players)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn player(&self, id: PlayerId) -> Result<&P, GameError> {
        self.0
            .get(usize::from(id))
            .ok_or(GameError::InvalidPlayerIndex(id.0))
    }

    pub fn player_mut(&mut self, id: PlayerId) -> Result<&mut P, GameError> {
        self.0
            .get_mut(usize::from(id))
            .ok_or(GameError::InvalidPlayerIndex(id.0))
    }

    pub fn players(&self) -> &[P] {
        &self.0
    }

    pub fn players_mut(&mut self) -> &mut [P] {
        &mut self.0
    }
}

impl<P> Default for Players<P> {
    fn default() -> Self {
        Self(Default::default())
    }
}

#[derive(Debug, Clone)]
pub enum GameState {
    Lobby(Lobby),
    SelectingCharacters(SelectingCharacters),
    Round(Round),
    Results(Results),
}

impl GameState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lobby(&self) -> Result<&Lobby, GameError> {
        match self {
            Self::Lobby(l) => Ok(l),
            _ => Err(GameError::NotLobbyState),
        }
    }

    pub fn lobby_mut(&mut self) -> Result<&mut Lobby, GameError> {
        match self {
            Self::Lobby(l) => Ok(l),
            _ => Err(GameError::NotLobbyState),
        }
    }

    pub fn selecting_characters(&self) -> Result<&SelectingCharacters, GameError> {
        match self {
            Self::SelectingCharacters(s) => Ok(s),
            _ => Err(GameError::NotSelectingCharactersState),
        }
    }

    pub fn selecting_characters_mut(&mut self) -> Result<&mut SelectingCharacters, GameError> {
        match self {
            Self::SelectingCharacters(s) => Ok(s),
            _ => Err(GameError::NotSelectingCharactersState),
        }
    }

    pub fn round(&self) -> Result<&Round, GameError> {
        match self {
            Self::Round(r) => Ok(r),
            _ => Err(GameError::NotRoundState),
        }
    }

    pub fn round_mut(&mut self) -> Result<&mut Round, GameError> {
        match self {
            Self::Round(r) => Ok(r),
            _ => Err(GameError::NotRoundState),
        }
    }

    pub fn results(&self) -> Result<&Results, GameError> {
        match self {
            Self::Results(r) => Ok(r),
            _ => Err(GameError::NotResultsState),
        }
    }

    pub fn results_mut(&mut self) -> Result<&mut Results, GameError> {
        match self {
            Self::Results(r) => Ok(r),
            _ => Err(GameError::NotResultsState),
        }
    }

    pub fn start_game<P: AsRef<Path>>(&mut self, data_path: P) -> Result<(), GameError> {
        match self {
            Self::Lobby(lobby) => {
                *self = lobby.start_game(data_path)?;
                Ok(())
            }
            _ => Err(GameError::NotLobbyState),
        }
    }

    pub fn player_select_character(
        &mut self,
        id: PlayerId,
        character: Character,
    ) -> Result<(), GameError> {
        let selecting = match self {
            Self::SelectingCharacters(s) => s,
            _ => return Err(GameError::NotSelectingCharactersState),
        };

        if let Some(state) = selecting.player_select_character(id, character)? {
            *self = state;
        };

        Ok(())
    }

    pub fn end_player_turn(&mut self, id: PlayerId) -> Result<TurnEnded, GameError> {
        let round = match self {
            Self::Round(r) => r,
            _ => return Err(GameError::NotRoundState),
        };

        match round.end_player_turn(id)? {
            Either::Left(te) => Ok(te),
            Either::Right(state) => {
                *self = state;
                Ok(TurnEnded::new(None))
            }
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::Lobby(Lobby::default())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Lobby {
    players: Players<LobbyPlayer>,
}

impl Lobby {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.players.len()
    }

    pub fn is_empty(&self) -> bool {
        self.players.is_empty()
    }

    pub fn player(&self, id: PlayerId) -> Option<&LobbyPlayer> {
        self.players.player(id).ok()
    }

    pub fn players(&self) -> &[LobbyPlayer] {
        self.players.players()
    }

    pub fn players_mut(&mut self) -> &mut [LobbyPlayer] {
        self.players.players_mut()
    }

    pub fn usernames(&self) -> Vec<String> {
        self.players().iter().map(|p| p.name().to_owned()).collect()
    }

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

    pub fn player_info(&self, id: PlayerId) -> Vec<PlayerInfo> {
        self.players()
            .iter()
            .filter(|p| p.id() != id)
            .map(Into::into)
            .collect()
    }

    pub fn can_start(&self) -> bool {
        (4..=7).contains(&self.players.len())
    }

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

    /// Grab market card if available and reshuffles the rest of the deck.
    fn initial_market(markets: &mut Deck<Either<Market, Event>>) -> Option<Market> {
        match markets.deck.iter().position(|c| c.is_left()) {
            Some(pos) => markets.deck.swap_remove(pos).left(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
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
    pub fn player(&self, id: PlayerId) -> Result<&SelectingCharactersPlayer, GameError> {
        self.players.player(id)
    }

    pub fn player_by_name(&self, name: &str) -> Result<&SelectingCharactersPlayer, GameError> {
        self.players()
            .iter()
            .find(|p| p.name() == name)
            .ok_or_else(|| GameError::InvalidPlayerName(name.to_owned()))
    }

    pub fn players(&self) -> &[SelectingCharactersPlayer] {
        self.players.players()
    }

    pub fn chairman_id(&self) -> PlayerId {
        self.chairman
    }

    pub fn currently_selecting_id(&self) -> PlayerId {
        (self.characters.applies_to_player() as u8).into()
    }

    /// Internally used function that checks whether a player with such an `id` exists, and whether
    /// that player is actually the current player.
    fn player_as_current(&self, id: PlayerId) -> Result<&SelectingCharactersPlayer, GameError> {
        let currently_selecting_id = self.currently_selecting_id();
        match self.players.player(id) {
            Ok(player) if player.id() == currently_selecting_id => Ok(player),
            Ok(_) => Err(GameError::NotPlayersTurn),
            Err(e) => Err(e),
        }
    }

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

    pub fn player_get_closed_character(&self, id: PlayerId) -> Result<Character, GameError> {
        let _ = self.player_as_current(id)?;

        match self.characters.peek()?.closed_character {
            Some(closed_character) => Ok(closed_character),
            None => Err(SelectingCharactersError::NotChairman.into()),
        }
    }

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

    pub fn open_characters(&self) -> &[Character] {
        self.characters.open_characters()
    }

    pub fn turn_order(&self) -> Vec<PlayerId> {
        let start = usize::from(self.chairman) as u8;
        let limit = self.players.len() as u8;
        (start..limit).chain(0..start).map(Into::into).collect()
    }

    pub fn player_info(&self, id: PlayerId) -> Vec<PlayerInfo> {
        self.players()
            .iter()
            .filter(|p| p.id() != id)
            .map(Into::into)
            .collect()
    }
}

#[derive(Debug, Clone)]
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
    pub fn player(&self, id: PlayerId) -> Result<&RoundPlayer, GameError> {
        self.players.player(id)
    }

    pub fn player_mut(&mut self, id: PlayerId) -> Result<&mut RoundPlayer, GameError> {
        self.players.player_mut(id)
    }

    pub fn player_from_character(&self, character: Character) -> Option<&RoundPlayer> {
        self.players().iter().find(|p| p.character() == character)
    }

    pub fn player_by_name(&self, name: &str) -> Result<&RoundPlayer, GameError> {
        self.players()
            .iter()
            .find(|p| p.name() == name)
            .ok_or_else(|| GameError::InvalidPlayerName(name.to_owned()))
    }

    pub fn current_player(&self) -> &RoundPlayer {
        self.player(self.current_player)
            .expect("self.current_player went out of bounds")
    }

    pub fn next_player(&self) -> Option<&RoundPlayer> {
        let current_character = self.current_player().character();
        self.players()
            .iter()
            .filter(|p| {
                p.character() > current_character && !self.fired_characters.contains(&p.character())
            })
            .min_by(|p1, p2| p1.character().cmp(&p2.character()))
    }

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

    pub fn players(&self) -> &[RoundPlayer] {
        self.players.players()
    }

    pub fn open_characters(&self) -> &[Character] {
        &self.open_characters
    }

    pub fn player_info(&self, id: PlayerId) -> Vec<PlayerInfo> {
        self.players()
            .iter()
            .filter(|p| p.id() != id)
            .map(Into::into)
            .collect()
    }

    pub fn current_market(&self) -> &Market {
        &self.current_market
    }

    /// Internally used function that checks whether a player with such an `id` exists, and whether
    /// that player is actually the current player.
    fn player_as_current_mut(&mut self, id: PlayerId) -> Result<&mut RoundPlayer, GameError> {
        match self.players.player_mut(id) {
            Ok(player) if player.id() == self.current_player => Ok(player),
            Ok(_) => Err(GameError::NotPlayersTurn),
            Err(e) => Err(e),
        }
    }

    pub fn player_get_fireble_characters(
        &mut self,
    ) -> Vec<Character> {
        Character::CHARACTERS
            .into_iter()
            .filter(|c| {
                *c as u8 > 2
                    && !self.fired_characters.contains(c)
                    && !self.open_characters.contains(c)
            })
            .clone().collect()
    }

    pub fn player_play_card(
        &mut self,
        id: PlayerId,
        card_idx: usize,
    ) -> Result<PlayerPlayedCard, GameError> {
        let player = self.player_as_current_mut(id)?;
        let assets_len = player.assets().len();

        match player.play_card(card_idx)? {
            Either::Left(asset) => {
                let market = match self.check_new_market(assets_len) {
                    true => Some(self.new_market()),
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

    fn end_player_turn(&mut self, id: PlayerId) -> Result<Either<TurnEnded, GameState>, GameError> {
        let player = self.player_as_current_mut(id)?;
        if !player.should_give_back_cards() {
            if let Some(id) = self.next_player().map(|p| p.id()) {
                let player = self.players.player_mut(id)?;
                player.start_turn(&self.current_market);
                self.current_player = player.id();
                Ok(Either::Left(TurnEnded::new(Some(self.current_player))))
            } else {
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
            }
        } else {
            Err(GameError::PlayerShouldGiveBackCard)
        }
    }

    fn check_new_market(&self, player_assets: usize) -> bool {
        let max_asset_count = self
            .players()
            .iter()
            .map(|player| player.assets().len())
            .max()
            .unwrap_or_default();

        max_asset_count > player_assets
    }

    /// Starts a new market. Automatically triggers if any player gets the first, second, third, fourth, fifth, seventh or eight asset. Loops through the deck and fetches events as they come.
    fn new_market(&mut self) -> MarketChange {
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

#[derive(Debug, Clone)]
pub struct Results {
    players: Players<ResultsPlayer>,
    final_market: Market,
    // TODO: implement events
    _final_events: Vec<Event>,
}

impl Results {
    pub fn player(&self, id: PlayerId) -> Result<&ResultsPlayer, GameError> {
        self.players.player(id)
    }

    pub fn player_by_name(&self, name: &str) -> Result<&ResultsPlayer, GameError> {
        self.players()
            .iter()
            .find(|p| p.name() == name)
            .ok_or_else(|| GameError::InvalidPlayerName(name.to_owned()))
    }

    pub fn players(&self) -> &[ResultsPlayer] {
        self.players.players()
    }

    pub fn score(&self, id: PlayerId) -> Result<f64, GameError> {
        let player = self.player(id)?;

        let gold = player.total_gold() as f64;
        let silver = player.total_silver() as f64;

        let trade_credit = player.trade_credit() as f64;
        let bank_loan = player.bank_loan() as f64;
        let bonds = player.bonds() as f64;
        let debt = trade_credit + bank_loan + bonds;

        let beta = silver / gold;

        // TODO: end of game bonuses
        let drp = (trade_credit + bank_loan * 2.0 + bonds * 3.0) / gold;

        let wacc = self.final_market.rfr as f64 + drp + beta * self.final_market.mrp as f64;

        let red = player.color_value(Color::Red, &self.final_market);
        let green = player.color_value(Color::Green, &self.final_market);
        let yellow = player.color_value(Color::Yellow, &self.final_market);
        let purple = player.color_value(Color::Purple, &self.final_market);
        let blue = player.color_value(Color::Blue, &self.final_market);

        let fcf = red + green + yellow + purple + blue;

        let score = (fcf / (10.0 * wacc)) + (debt / 3.0) + player.cash() as f64;

        Ok(score)
    }

    pub fn player_info(&self, id: PlayerId) -> Vec<PlayerInfo> {
        self.players()
            .iter()
            .filter(|p| p.id() != id)
            .map(Into::into)
            .collect()
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
