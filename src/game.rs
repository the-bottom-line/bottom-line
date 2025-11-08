use std::{collections::HashSet, path::Path, sync::Arc, vec};

use either::Either;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::{cards::GameData, errors::*, player::*, utility::serde_asset_liability};

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

    pub fn shuffle(&mut self) {
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
    pub(crate) characters: Vec<Character>,
    pub(crate) closed_character: Option<Character>,
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
    pub fn new(player_count: usize, chairman_id: PlayerId) -> Self {
        let mut available_characters = Deck::new(Character::CHARACTERS.to_vec());
        available_characters.shuffle();

        let open_characters = match player_count {
            4 => vec![available_characters.draw(), available_characters.draw()],
            5 => vec![available_characters.draw()],
            6 | 7 => vec![],
            _ => unreachable!("Games should always have between 4 and 7 players"),
        };
        let closed_character = available_characters.draw();

        ObtainingCharacters {
            player_count,
            draw_idx: 0,
            chairman_id: chairman_id.into(),
            available_characters,
            open_characters,
            closed_character,
        }
    }

    pub fn peek(&self) -> Result<PickableCharacters, SelectableCharactersError> {
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
            _ => Err(SelectableCharactersError::NotPickingCharacters),
        }
    }

    pub fn pick(&mut self, character: Character) -> Result<(), SelectableCharactersError> {
        match self.peek() {
            Ok(PickableCharacters { mut characters, .. }) => {
                match characters.iter().position(|&c| c == character) {
                    Some(i) => {
                        characters.remove(i);
                        self.draw_idx += 1;
                        self.available_characters.deck = characters;
                        Ok(())
                    }
                    None => Err(SelectableCharactersError::UnavailableCharacter),
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

    // pub fn join(&mut self, username: String) -> Result<PlayerId, GameError> {
    //     match self {
    //         Self::Lobby(lobby) => match lobby.join(username) {
    //             Ok(player) => Ok(player.id),
    //             Err(e) => Err(e.into()),
    //         },
    //         _ => Err(GameError::NotLobbyState),
    //     }
    // }

    // pub fn leave(&mut self, username: &str) -> Result<bool, GameError> {
    //     match self {
    //         Self::Lobby(lobby) => Ok(lobby.leave(username)),
    //         _ => Err(GameError::NotLobbyState),
    //     }
    // }

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

    // pub fn player_get_selectable_characters(
    //     &self,
    //     id: PlayerId,
    // ) -> Result<PickableCharacters, GameError> {
    //     match self {
    //         Self::SelectingCharacters(s) => s.player_get_selectable_characters(id),
    //         _ => Err(GameError::NotSelectingCharactersState),
    //     }
    // }

    // pub fn open_characters(&self) -> Result<&[Character], GameError> {
    //     match self {
    //         Self::SelectingCharacters(s) => Ok(s.open_characters()),
    //         Self::Round(r) => Ok(r.open_characters()),
    //         GameState::Lobby(_) => Err(GameError::NotAvailableInLobbyState),
    //         GameState::Results(_) => Err(GameError::NotAvailableInResultsState),
    //     }
    // }

    // pub fn player(&self, id: PlayerId) -> Result<&Player, GameError> {
    //     match self {
    //         Self::SelectingCharacters(s) => s.player(id),
    //         Self::Round(r) => r.player(id),
    //         Self::Results(r) => r.player(id),
    //         Self::Lobby(_) => Err(GameError::NotAvailableInLobbyState),
    //     }
    // }

    // pub fn current_player(&self) -> Result<&Player, GameError> {
    //     match self {
    //         Self::Round(r) => Ok(r.current_player()),
    //         _ => Err(GameError::NotRoundState),
    //     }
    // }

    // pub fn next_player(&self) -> Option<&Player> {
    //     match self {
    //         Self::Round(r) => r.next_player(),
    //         _ => None,
    //     }
    // }

    // pub fn player_by_name(&self, name: &str) -> Result<&Player, GameError> {
    //     match self {
    //         Self::SelectingCharacters(s) => s.player_by_name(name),
    //         Self::Round(r) => r.player_by_name(name),
    //         Self::Results(r) => r.player_by_name(name),
    //         Self::Lobby(_) => Err(GameError::NotAvailableInLobbyState),
    //     }
    // }

    // pub fn player_from_character(&self, character: Character) -> Option<&Player> {
    //     let players = match self {
    //         Self::SelectingCharacters(s) => &s.players,
    //         Self::Round(r) => &r.players,
    //         Self::Results(r) => &r.players,
    //         Self::Lobby(_) => return None,
    //     };

    //     players.iter().find(|p| p.character == Some(character))
    // }

    // pub fn players(&self) -> Result<&[Player], GameError> {
    //     match self {
    //         GameState::Lobby(_) => Err(GameError::NotAvailableInLobbyState),
    //         GameState::SelectingCharacters(selecting) => Ok(selecting.players()),
    //         GameState::Round(round) => Ok(round.players()),
    //         GameState::Results(results) => Ok(results.players()),
    //     }
    // }

    // pub fn player_play_card(
    //     &mut self,
    //     id: PlayerId,
    //     card_idx: usize,
    // ) -> Result<PlayerPlayedCard, GameError> {
    //     match self {
    //         Self::Round(r) => r.player_play_card(id, card_idx),
    //         _ => Err(GameError::NotRoundState),
    //     }
    // }

    // pub fn player_draw_card(
    //     &mut self,
    //     id: PlayerId,
    //     card_type: CardType,
    // ) -> Result<Either<&Asset, &Liability>, GameError> {
    //     match self {
    //         Self::Round(r) => r.player_draw_card(id, card_type),
    //         _ => Err(GameError::NotRoundState),
    //     }
    // }

    // pub fn player_give_back_card(
    //     &mut self,
    //     id: PlayerId,
    //     card_idx: usize,
    // ) -> Result<CardType, GameError> {
    //     match self {
    //         Self::Round(r) => r.player_give_back_card(id, card_idx),
    //         _ => Err(GameError::NotRoundState),
    //     }
    // }

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

    // pub fn player_info(&self, id: PlayerId) -> Vec<PlayerInfo> {
    //     match self {
    //         Self::Lobby(l) => l.player_info(id),
    //         Self::SelectingCharacters(s) => s.player_info(id),
    //         Self::Round(r) => r.player_info(id),
    //         Self::Results(r) => r.player_info(id),
    //     }
    // }

    // pub fn turn_order(&self) -> Result<Vec<PlayerId>, GameError> {
    //     match self {
    //         Self::SelectingCharacters(s) => Ok(s.turn_order()),
    //         _ => Err(GameError::NotSelectingCharactersState),
    //     }
    // }
}

impl Default for GameState {
    fn default() -> Self {
        Self::Lobby(Lobby::default())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Lobby {
    players: Vec<LobbyPlayer>,
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
        self.players.get(usize::from(id))
    }

    pub fn players(&self) -> &[LobbyPlayer] {
        &self.players
    }

    pub fn usernames(&self) -> Vec<String> {
        self.players.iter().map(|p| &p.name).cloned().collect()
    }

    pub fn join(&mut self, username: String) -> Result<&LobbyPlayer, LobbyError> {
        match self.players.iter().find(|p| p.name == username) {
            Some(_) => Err(LobbyError::UsernameAlreadyTaken(username)),
            None => {
                let player = LobbyPlayer {
                    id: PlayerId(self.players.len() as u8),
                    name: username.clone(),
                };
                self.players.push(player);
                Ok(&self.players[self.players.len() - 1])
            }
        }
    }

    pub fn leave(&mut self, username: &str) -> bool {
        match self.players.iter().position(|p| p.name == username) {
            Some(pos) => {
                self.players.remove(pos);
                self.players
                    .iter_mut()
                    .zip(0u8..)
                    .for_each(|(p, id)| p.id = PlayerId(id));
                true
            }
            None => false,
        }
    }

    pub fn player_info(&self, id: PlayerId) -> Vec<PlayerInfo> {
        self.players
            .iter()
            .filter(|p| p.id != id)
            .map(Into::into)
            .collect()
    }

    pub fn can_start(&self) -> bool {
        (4..=7).contains(&self.players.len())
    }

    fn start_game<P: AsRef<Path>>(&mut self, data_path: P) -> Result<GameState, GameError> {
        if self.can_start() {
            let mut data = GameData::new(data_path).expect("Path for game data is invalid");
            data.shuffle_all();

            let mut assets = data.assets;
            let mut liabilities = data.liabilities;
            let mut markets = data.market_deck;

            let players = self.init_players(&mut assets, &mut liabilities);
            let current_market =
                Lobby::initial_market(&mut markets).expect("No markets in deck for some reason");

            let chairman = players.first().unwrap().id;
            let characters = ObtainingCharacters::new(players.len(), chairman);

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
    ) -> Vec<SelectingCharactersPlayer> {
        self.players.sort_by(|p1, p2| p1.id.cmp(&p2.id));

        self.players
            .iter()
            .map(|p| {
                let assets = [assets.draw(), assets.draw()];
                let liabilities = [liabilities.draw(), liabilities.draw()];
                SelectingCharactersPlayer::new(&p.name, p.id.0, assets, liabilities, 1)
            })
            .collect()
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
    players: Vec<SelectingCharactersPlayer>,
    characters: ObtainingCharacters,
    assets: Deck<Asset>,
    liabilities: Deck<Liability>,
    markets: Deck<Either<Market, Event>>,
    pub chairman: PlayerId,
    current_market: Market,
    current_events: Vec<Event>,
}

impl SelectingCharacters {
    pub fn player(&self, id: PlayerId) -> Result<&SelectingCharactersPlayer, GameError> {
        self.players
            .get(usize::from(id))
            .ok_or(GameError::InvalidPlayerIndex(id.0))
    }

    pub fn player_by_name(&self, name: &str) -> Result<&SelectingCharactersPlayer, GameError> {
        self.players
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| GameError::InvalidPlayerName(name.to_owned()))
    }

    pub fn players(&self) -> &[SelectingCharactersPlayer] {
        &self.players
    }

    pub fn currently_selecting_id(&self) -> PlayerId {
        (self.characters.applies_to_player() as u8).into()
    }

    pub fn player_get_selectable_characters(
        &self,
        id: PlayerId,
    ) -> Result<Vec<Character>, GameError> {
        match self.player(id) {
            Ok(p) if p.id == self.currently_selecting_id() => self
                .characters
                .peek()
                .map(|pc| pc.characters)
                .map_err(Into::into),
            Ok(_) => Err(GameError::NotPlayersTurn),
            Err(e) => Err(e),
        }
    }

    pub fn player_get_closed_character(&self, id: PlayerId) -> Result<Character, GameError> {
        match self.player(id) {
            Ok(p) if p.id == self.currently_selecting_id() => {
                match self.characters.peek()?.closed_character {
                    Some(closed_character) => Ok(closed_character),
                    None => Err(SelectableCharactersError::NotChairman.into()),
                }
            }
            Ok(_) => Err(GameError::NotPlayersTurn),
            Err(e) => Err(e),
        }
    }

    fn player_select_character(
        &mut self,
        id: PlayerId,
        character: Character,
    ) -> Result<Option<GameState>, GameError> {
        let currently_selecting_id = self.currently_selecting_id();

        match self.players.get_mut(usize::from(id)) {
            Some(p) if p.id == currently_selecting_id => {
                self.characters.pick(character)?;

                p.select_character(character);

                // Start round when no more characters can be picked
                if self.characters.peek().is_err() {
                    let current_player = self
                        .players
                        .iter()
                        .min_by(|p1, p2| p1.character.cmp(&p2.character))
                        .map(|p| p.id)
                        .unwrap();

                    let players = std::mem::take(&mut self.players);
                    let assets = std::mem::take(&mut self.assets);
                    let liabilities = std::mem::take(&mut self.liabilities);
                    let markets = std::mem::take(&mut self.markets);
                    let current_market = std::mem::take(&mut self.current_market);
                    let current_events = std::mem::take(&mut self.current_events);
                    let open_characters = self.characters.open_characters().to_vec();

                    let players = players
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<_, _>>()?;

                    let state = GameState::Round(Round {
                        current_player,
                        players,
                        assets,
                        liabilities,
                        markets,
                        chairman: self.chairman,
                        current_market,
                        current_events,
                        open_characters,
                    });

                    Ok(Some(state))
                } else {
                    Ok(None)
                }
            }
            Some(_) => Err(GameError::NotPlayersTurn),
            None => Err(GameError::InvalidPlayerIndex(id.0)),
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
        self.players
            .iter()
            .filter(|p| p.id != id)
            .map(Into::into)
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct Round {
    current_player: PlayerId,
    players: Vec<RoundPlayer>,
    assets: Deck<Asset>,
    liabilities: Deck<Liability>,
    markets: Deck<Either<Market, Event>>,
    pub chairman: PlayerId,
    current_market: Market,
    current_events: Vec<Event>,
    open_characters: Vec<Character>,
}

impl Round {
    pub fn player(&self, id: PlayerId) -> Result<&RoundPlayer, GameError> {
        self.players
            .get(usize::from(id))
            .ok_or(GameError::InvalidPlayerIndex(id.0))
    }

    pub fn player_from_character(&self, character: Character) -> Option<&RoundPlayer> {
        self.players.iter().find(|p| p.character == character)
    }

    pub fn player_by_name(&self, name: &str) -> Result<&RoundPlayer, GameError> {
        self.players
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| GameError::InvalidPlayerName(name.to_owned()))
    }

    pub fn current_player(&self) -> &RoundPlayer {
        self.player(self.current_player)
            .expect("self.current_player went out of bounds")
    }

    pub fn next_player(&self) -> Option<&RoundPlayer> {
        let current_character = self.current_player().character;
        self.players
            .iter()
            .filter(|p| p.character > current_character)
            .min_by(|p1, p2| p1.character.cmp(&p2.character))
    }

    pub fn next_player_mut(&mut self) -> Option<&mut RoundPlayer> {
        let current_character = self.current_player().character;
        self.players
            .iter_mut()
            .filter(|p| p.character > current_character)
            .min_by(|p1, p2| p1.character.cmp(&p2.character))
    }

    pub fn players(&self) -> &[RoundPlayer] {
        &self.players
    }

    pub fn open_characters(&self) -> &[Character] {
        &self.open_characters
    }

    pub fn player_info(&self, id: PlayerId) -> Vec<PlayerInfo> {
        self.players
            .iter()
            .filter(|p| p.id != id)
            .map(Into::into)
            .collect()
    }

    pub fn player_play_card(
        &mut self,
        id: PlayerId,
        card_idx: usize,
    ) -> Result<PlayerPlayedCard, GameError> {
        match self.players.get_mut(usize::from(id)) {
            Some(player) if player.id == self.current_player => {
                let current_assets = player.assets.len();
                match player.play_card(card_idx)? {
                    Either::Left(asset) => {
                        let market = match self.check_new_market(current_assets) {
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
            Some(_) => Err(GameError::NotPlayersTurn),
            _ => Err(GameError::InvalidPlayerIndex(id.0)),
        }
    }

    pub fn player_draw_card(
        &mut self,
        id: PlayerId,
        card_type: CardType,
    ) -> Result<Either<&Asset, &Liability>, GameError> {
        match self.players.get_mut(usize::from(id)) {
            Some(player) if player.id == self.current_player => {
                if player.can_draw_cards() {
                    let card = match card_type {
                        CardType::Asset => Either::Left(self.assets.draw()),
                        CardType::Liability => Either::Right(self.liabilities.draw()),
                    };
                    player.draw_card(card);
                    Ok(player.hand.last().unwrap().as_ref())
                } else {
                    Err(DrawCardError::MaximumCardsDrawn(player.total_cards_drawn).into())
                }
            }
            Some(_) => Err(GameError::NotPlayersTurn),
            _ => Err(GameError::InvalidPlayerIndex(id.0)),
        }
    }

    pub fn player_give_back_card(
        &mut self,
        id: PlayerId,
        card_idx: usize,
    ) -> Result<CardType, GameError> {
        match self.players.get_mut(usize::from(id)) {
            Some(player) if player.id == self.current_player => {
                if player.should_give_back_cards() {
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
                } else {
                    Err(GiveBackCardError::Unnecessary.into())
                }
            }
            Some(_) => Err(GameError::NotPlayersTurn),
            _ => Err(GameError::InvalidPlayerIndex(id.0)),
        }
    }

    pub fn skipped_characters(&self) -> Vec<Character> {
        let mut cs: Vec<Character> = [].to_vec();
        let mut past_current_character = false;
        for c in Character::CHARACTERS.into_iter().rev() {
            if let Some(cp) = self.player_from_character(c) {
                if past_current_character {
                    return cs;
                } else if cp.id == self.current_player {
                    past_current_character = true;
                }
            } else if past_current_character {
                cs.push(c);
            }
        }
        cs
    }

    fn end_player_turn(&mut self, id: PlayerId) -> Result<Either<TurnEnded, GameState>, GameError> {
        match self.player(id) {
            Ok(current)
                if current.id == self.current_player && !current.should_give_back_cards() =>
            {
                if let Some(player) = self.next_player_mut() {
                    player.start_turn();
                    self.current_player = player.id;
                    Ok(Either::Left(TurnEnded::new(Some(self.current_player))))
                } else {
                    let maybe_ceo = self.player_from_character(Character::CEO);
                    let chairman_id = match maybe_ceo.map(|p| p.id) {
                        Some(id) => id,
                        None => self.chairman,
                    };

                    let characters = ObtainingCharacters::new(self.players.len(), chairman_id);
                    let players = std::mem::take(&mut self.players);
                    let assets = std::mem::take(&mut self.assets);
                    let liabilities = std::mem::take(&mut self.liabilities);
                    let markets = std::mem::take(&mut self.markets);
                    let current_market = std::mem::take(&mut self.current_market);
                    let current_events = std::mem::take(&mut self.current_events);

                    let players = players.into_iter().map(Into::into).collect();

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
            }
            Ok(_) => Err(GameError::NotPlayersTurn),
            Err(e) => Err(e),
        }
    }

    fn check_new_market(&self, player_assets: usize) -> bool {
        let max_asset_count = self
            .players
            .iter()
            .map(|player| player.assets.len())
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
    players: Vec<ResultsPlayer>,
    final_market: Market,
    // TODO: implement events
    _final_events: Vec<Event>,
}

impl Results {
    pub fn player(&self, id: PlayerId) -> Result<&ResultsPlayer, GameError> {
        self.players
            .get(usize::from(id))
            .ok_or(GameError::InvalidPlayerIndex(id.0))
    }

    pub fn player_by_name(&self, name: &str) -> Result<&ResultsPlayer, GameError> {
        self.players
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| GameError::InvalidPlayerName(name.to_owned()))
    }

    pub fn players(&self) -> &[ResultsPlayer] {
        &self.players
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

        let score = (fcf / (10.0 * wacc)) + (debt / 3.0) + player.cash as f64;

        Ok(score)
    }

    pub fn player_info(&self, id: PlayerId) -> Vec<PlayerInfo> {
        self.players
            .iter()
            .filter(|p| p.id != id)
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

            assert!(round.players.iter().map(|p| p.id).all_unique());
        }
    }

    #[test]
    fn ids_sorted() {
        for i in 4..=7 {
            let game = pick_with_players(i).expect("couldn't pick characters");
            let round = game.round().unwrap();

            assert!(round.players.iter().map(|p| p.id).is_sorted());
        }
    }

    #[test]
    fn player_from_character() {
        for i in 4..=7 {
            let game = pick_with_players(i).expect("couldn't pick characters");
            let round = game.round().unwrap();

            round
                .players
                .iter()
                .map(|p| (p.character, p.id))
                .for_each(|(c, id)| {
                    let p = round
                        .player_from_character(c)
                        .expect("couldn't find character");

                    assert_eq!(p.id, id);
                });
        }
    }

    #[test]
    fn player_by_name() {
        for i in 4..=7 {
            let game = pick_with_players(i).expect("couldn't pick characters");
            let round = game.round().unwrap();

            round
                .players
                .iter()
                .map(|p| (p.name.as_str(), p.id))
                .for_each(|(name, id)| {
                    let p = round.player_by_name(name).expect("couldn't find name");

                    assert_eq!(p.id, id);
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
                    let current_player = round.current_player().id;

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
            round.player_draw_card(next_player.id, CardType::Asset),
            Err(GameError::NotPlayersTurn)
        )
    }

    #[test]
    fn player_play_card() {
        for i in 4..=7 {
            let mut game = pick_with_players(i).expect("couldn't pick characters");
            let round = game.round_mut().expect("Game not in round state");

            let current_player = round.current_player().id;

            draw_cards(
                round,
                current_player,
                [CardType::Asset, CardType::Asset, CardType::Liability],
            );

            // so player can always afford the asset
            round.players[usize::from(current_player)].cash = 50;

            // test issuing liability
            let player = &round.player(current_player).unwrap();
            let hand_len = player.hand.len();
            let liability_value = player.hand[hand_len - 1]
                .as_ref()
                .right()
                .expect("Couldn't get liability")
                .value;
            let cash_before = player.cash;

            assert_ok!(round.player_play_card(current_player, hand_len - 1));
            assert_eq!(
                cash_before + liability_value,
                round.player(current_player).unwrap().cash
            );

            assert_eq!(
                hand_len - 1,
                round.player(current_player).unwrap().hand.len()
            );

            // test buying asset
            let player = &round.player(current_player).unwrap();
            let hand_len = player.hand.len();
            let liability_value = player.hand[hand_len - 1]
                .as_ref()
                .left()
                .expect("Couldn't get asset")
                .gold_value;
            let cash_before = player.cash;

            assert_ok!(round.player_play_card(current_player, hand_len - 1));
            assert_eq!(
                cash_before - liability_value,
                round.player(current_player).unwrap().cash
            );

            assert_eq!(
                hand_len - 1,
                round.player(current_player).unwrap().hand.len()
            );

            let hand_len = round.player(current_player).unwrap().hand.len();
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
            round.player_play_card(next_player.id, 0),
            Err(GameError::NotPlayersTurn)
        )
    }

    #[test]
    fn end_player_turn_no_actions() {
        let mut game = pick_with_players(4).expect("couldn't pick characters");
        let round = game.round().expect("Game not in round state");

        let current_player = round.current_player().id;

        assert_ok!(game.end_player_turn(current_player));
    }

    #[test]
    fn end_player_turn_used_cards() {
        let mut game = pick_with_players(4).expect("couldn't pick characters");
        let round = game.round_mut().expect("not in round state");

        let current_player = round.current_player().id;

        // so player can always afford the asset
        round.players[usize::from(current_player)].cash = 50;

        let hand_len = round.players[usize::from(current_player)].hand.len();
        assert_ok!(round.player_play_card(current_player, hand_len - 1));
        assert_ok!(round.player_play_card(current_player, 0));

        assert_ok!(game.end_player_turn(current_player));
    }

    #[test]
    fn end_player_turn_drew_three_cards() {
        let mut game = pick_with_players(4).expect("couldn't pick characters");
        let round = game.round().expect("Game not in round state");

        let current_player = round.current_player().id;

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

                    let current_player = round.current_player().id;

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
        let hand_len = round.players[usize::from(player_id)].hand.len();
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
            .for_each(|(i, name)| assert_matches!(lobby.join(name), Ok(p) if p.id == PlayerId(i)));

        game.start_game("assets/cards/boardgame.json")?;

        assert_matches!(game, GameState::SelectingCharacters(_));
        assert_eq!(
            game.selecting_characters().unwrap().players.len(),
            player_count
        );

        finish_selecting_characters(&mut game);

        Ok(game)
    }
}
