use std::{collections::HashSet, path::Path, sync::Arc, vec};

use either::Either;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::{cards::GameData, game_errors::*, utility::serde_asset_liability};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerId(u8);

impl<I: Into<u8>> From<I> for PlayerId {
    fn from(value: I) -> Self {
        Self(value.into())
    }
}

impl From<PlayerId> for usize {
    fn from(value: PlayerId) -> Self {
        value.0 as usize
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Color {
    Red,
    Green,
    Purple,
    Yellow,
    Blue,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Character {
    Shareholder,
    Banker,
    Regulator,
    CEO,
    CFO,
    CSO,
    HeadRnD,
    Stakeholder,
}

impl Character {
    pub const CHARACTERS: [Character; 8] = [
        Self::Shareholder,
        Self::Banker,
        Self::Regulator,
        Self::CEO,
        Self::CFO,
        Self::CSO,
        Self::HeadRnD,
        Self::Stakeholder,
    ];

    pub fn color(&self) -> Option<Color> {
        use Color::*;

        match self {
            Self::Shareholder => None,
            Self::Banker => None,
            Self::Regulator => None,
            Self::CEO => Some(Yellow),
            Self::CFO => Some(Blue),
            Self::CSO => Some(Green),
            Self::HeadRnD => Some(Purple),
            Self::Stakeholder => Some(Red),
        }
    }

    pub fn next(&self) -> Option<Self> {
        use Character::*;

        match self {
            Shareholder => Some(Banker),
            Banker => Some(Regulator),
            Regulator => Some(CEO),
            CEO => Some(CFO),
            CFO => Some(CSO),
            CSO => Some(HeadRnD),
            HeadRnD => Some(Stakeholder),
            Stakeholder => None,
        }
    }

    pub fn first(characters: &[Self]) -> Option<Self> {
        characters.iter().max().copied()
    }

    pub fn playable_assets(&self) -> usize {
        match self {
            Self::CEO => 3,
            _ => 1,
        }
    }

    pub fn playable_liabilities(&self) -> usize {
        match self {
            Self::CFO => 3,
            _ => 1,
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum AssetPowerup {
    #[serde(rename = "At the end of the game, for one color, turn - into 0 or 0 into +")]
    MinusIntoPlus,
    #[serde(rename = "At the end of the game, turn silver into gold on one asset card")]
    SilverIntoGold,
    #[serde(rename = "At the end of the game, count one of your assets as any color")]
    CountAsAnyColor,
}

impl std::fmt::Display for AssetPowerup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MinusIntoPlus => write!(
                f,
                "At the end of the game, for one color, turn - into 0 or 0 into +"
            ),
            Self::SilverIntoGold => write!(
                f,
                "At the end of the game, turn silver into gold on one asset card"
            ),
            Self::CountAsAnyColor => write!(
                f,
                "At the end of the game, count one of your assets as any color"
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub title: String,
    pub gold_value: u8,
    pub silver_value: u8,
    pub color: Color,
    pub ability: Option<AssetPowerup>,
    pub image_front_url: String,
    pub image_back_url: Arc<String>,
}

impl std::fmt::Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\ngold: {}\nsilver: {}\ncolor: {:?}",
            self.title, self.gold_value, self.silver_value, self.color
        )
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LiabilityType {
    #[serde(rename = "Trade Credit")]
    TradeCredit,
    #[serde(rename = "Bank Loan")]
    BankLoan,
    Bonds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Liability {
    pub value: u8,
    pub rfr_type: LiabilityType,
    pub image_front_url: String,
    pub image_back_url: Arc<String>,
}

impl Liability {
    pub fn rfr_percentage(&self) -> u8 {
        match self.rfr_type {
            LiabilityType::TradeCredit => 1,
            LiabilityType::BankLoan => 2,
            LiabilityType::Bonds => 3,
        }
    }
}

impl std::fmt::Display for Liability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let title = serde_json::to_string(&self.rfr_type).unwrap();
        write!(
            f,
            "{title} - {}%\nvalue: {}\n",
            self.rfr_percentage(),
            self.value
        )
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CardType {
    Asset,
    Liability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub name: String,
    pub id: PlayerId,
    pub hand: Vec<CardType>,
    pub assets: Vec<Asset>,
    pub liabilities: Vec<Liability>,
    pub cash: u8,
    pub character: Option<Character>,
}

impl From<&Player> for PlayerInfo {
    fn from(player: &Player) -> Self {
        let hand = player
            .hand
            .iter()
            .map(|e| match e {
                Either::Left(_) => CardType::Asset,
                Either::Right(_) => CardType::Liability,
            })
            .collect();

        Self {
            hand,
            name: player.name.clone(),
            assets: player.assets.clone(),
            liabilities: player.liabilities.clone(),
            id: player.id,
            cash: player.cash,
            character: player.character,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub cash: u8,
    pub assets: Vec<Asset>,
    pub liabilities: Vec<Liability>,
    pub character: Option<Character>,
    #[serde(with = "serde_asset_liability::vec")]
    pub hand: Vec<Either<Asset, Liability>>,
    pub cards_drawn: Vec<usize>,
    pub assets_to_play: u8,
    pub liabilities_to_play: u8,
    pub total_cards_drawn: u8,
    pub total_cards_given_back: u8,
}

impl Player {
    pub fn new(
        name: &str,
        id: u8,
        assets: [Asset; 2],
        liabilities: [Liability; 2],
        cash: u8,
    ) -> Player {
        let hand = assets
            .into_iter()
            .map(Either::Left)
            .chain(liabilities.into_iter().map(Either::Right))
            .collect();

        Player {
            id: PlayerId(id),
            name: name.to_string(),
            cash,
            assets: vec![],
            liabilities: vec![],
            character: None,
            hand,
            cards_drawn: vec![],
            assets_to_play: 1,
            liabilities_to_play: 1,
            total_cards_drawn: 0,
            total_cards_given_back: 0,
        }
    }

    pub fn info(&self) -> PlayerInfo {
        self.into()
    }

    fn update_cards_drawn(&mut self, card_idx: usize) {
        self.cards_drawn = self
            .cards_drawn
            .iter()
            .copied()
            .filter(|&i| i != card_idx)
            .collect();
    }

    fn can_play_asset(&self) -> bool {
        match self.character {
            Some(_) => self.assets_to_play > 0,
            None => false,
        }
    }

    fn can_play_liability(&self) -> bool {
        match self.character {
            Some(_) => self.liabilities_to_play > 0,
            None => false,
        }
    }

    /// Plays card in players hand with index `card_idx`. If that index is valid, the card is played
    /// if
    pub fn play_card(
        &mut self,
        card_idx: usize,
    ) -> Result<Either<Asset, Liability>, PlayCardError> {
        use PlayCardError::*;

        if let Some(card) = self.hand.get(card_idx) {
            match card {
                Either::Left(a) if self.can_play_asset() && self.cash >= a.gold_value => {
                    let asset = self.hand.remove(card_idx).left().unwrap();
                    self.cash -= asset.gold_value;
                    self.assets_to_play -= 1;
                    self.assets.push(asset.clone());
                    self.update_cards_drawn(card_idx);
                    Ok(Either::Left(asset))
                }
                Either::Left(_) if !self.can_play_asset() => Err(ExceedsMaximumAssets),
                Either::Left(a) if self.cash < a.gold_value => Err(CannotAffordAsset {
                    cash: self.cash,
                    cost: a.gold_value,
                }),
                Either::Right(_) if self.can_play_liability() => {
                    let liability = self.hand.remove(card_idx).right().unwrap();
                    self.cash += liability.value;
                    self.liabilities_to_play -= 1;
                    self.liabilities.push(liability.clone());
                    self.update_cards_drawn(card_idx);
                    Ok(Either::Right(liability))
                }
                Either::Right(_) if !self.can_play_liability() => Err(ExceedsMaximumLiabilities),
                _ => unreachable!(),
            }
        } else {
            Err(InvalidCardIndex(card_idx as u8))
        }
    }

    pub fn draw_card(&mut self, card: Either<Asset, Liability>) {
        self.total_cards_drawn += 1;
        self.cards_drawn.push(self.hand.len());
        self.hand.push(card);
    }

    pub fn give_back_card(
        &mut self,
        card_idx: usize,
    ) -> Result<Either<Asset, Liability>, GiveBackCardError> {
        self.total_cards_given_back += 1;

        match self.hand.get(card_idx) {
            Some(_) => {
                self.update_cards_drawn(card_idx);
                Ok(self.hand.remove(card_idx))
            }
            None => Err(GiveBackCardError::InvalidCardIndex(card_idx as u8)),
        }
    }

    pub fn should_give_back_cards(&self) -> bool {
        // TODO: add head rnd ability
        match self.character {
            Some(_) => self.total_cards_drawn - self.total_cards_given_back >= 3,
            None => false,
        }
    }

    pub fn can_draw_cards(&self) -> bool {
        // TODO: add head rnd ability
        self.total_cards_drawn < 3
    }

    pub fn select_character(&mut self, character: Character) {
        use Character::*;

        self.character = Some(character);

        match character {
            Shareholder => {}
            Banker => {}
            Regulator => {}
            CEO => {}
            CFO => {}
            CSO => {}
            HeadRnD => {}
            Stakeholder => {}
        }
    }

    pub fn total_gold(&self) -> u8 {
        self.assets.iter().map(|a| a.gold_value).sum()
    }

    pub fn total_silver(&self) -> u8 {
        self.assets.iter().map(|a| a.silver_value).sum()
    }

    fn calc_loan(&self, rfr_type: LiabilityType) -> u8 {
        self.liabilities
            .iter()
            .filter_map(|l| (l.rfr_type == rfr_type).then_some(l.value))
            .sum()
    }

    pub fn trade_credit(&self) -> u8 {
        self.calc_loan(LiabilityType::TradeCredit)
    }

    pub fn bank_loan(&self) -> u8 {
        self.calc_loan(LiabilityType::BankLoan)
    }

    pub fn bonds(&self) -> u8 {
        self.calc_loan(LiabilityType::Bonds)
    }

    pub fn color_value(&self, color: Color, market: &Market) -> f64 {
        let market_condition = match color {
            Color::Red => market.red,
            Color::Green => market.green,
            Color::Purple => market.purple,
            Color::Yellow => market.yellow,
            Color::Blue => market.blue,
        };

        let mul = match market_condition {
            MarketCondition::Plus => 1.0,
            MarketCondition::Minus => 0.0,
            MarketCondition::Zero => -1.0,
        };

        self.assets
            .iter()
            .filter_map(|a| {
                color
                    .eq(&a.color)
                    .then_some(a.gold_value as f64 + (a.silver_value as f64) * mul)
            })
            .sum()
    }
}

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

    /// Panics if no more cards are in the deck, for now. Decks don't run out in regular games.
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

pub trait TheBottomLine {
    /// Join the lobby if game is in lobby state
    fn join(&mut self, username: String) -> Result<bool, GameError>;

    /// Leave the lobby if game is in lobby state
    fn leave(&mut self, username: &str) -> Result<bool, GameError>;

    /// Starts the game when the game is in lobby state and between 4 and 7 players are present
    fn start_game<P: AsRef<Path>>(&mut self, data_path: P) -> Result<(), GameError>;

    /// Returns the ID of the player that's currently picking
    fn currently_selecting_id(&self) -> Option<PlayerId>;

    /// Checks if the game is in a selecting characters phase, which happens before each round
    /// starts.
    fn is_selecting_characters(&self) -> bool;

    /// Gets the character of the current turn.
    fn current_player(&self) -> Result<&Player, GameError>;

    /// Gets the character of the next turn
    fn next_player(&self) -> Option<&Player>;

    /// Get player based on player ID
    fn player(&self, id: PlayerId) -> Result<&Player, GameError>;

    /// Gets a player object based on a given username
    fn player_by_name(&self, name: &str) -> Result<&Player, GameError>;

    /// Gets player if one exists with specified character
    fn player_from_character(&self, character: Character) -> Option<&Player>;

    /// Gets list of selectable caracters if its the players turn
    fn player_get_selectable_characters(
        &self,
        id: PlayerId,
    ) -> Result<PickableCharacters, GameError>;

    /// Assigns a character role to a specific player. Returns a set of pickable characters for the
    /// next player to choose from
    fn player_select_character(
        &mut self,
        id: PlayerId,
        character: Character,
    ) -> Result<(), GameError>;

    /// Gets the list of open characters visible to everyone if there are any.
    fn open_characters(&self) -> Result<&[Character], GameError>;

    /// Attempts to play a card (either an asset or liability) for player with `player_idx`. If
    /// playing this card triggers a market change, returns an object with a list of events and
    /// a new market.
    fn player_play_card(
        &mut self,
        id: PlayerId,
        card_idx: usize,
    ) -> Result<PlayerPlayedCard, GameError>;

    fn player_draw_card(
        &mut self,
        id: PlayerId,
        card_type: CardType,
    ) -> Result<Either<&Asset, &Liability>, GameError>;

    /// When the player grabs 3 cards, the player should give back one.
    fn player_give_back_card(
        &mut self,
        id: PlayerId,
        card_idx: usize,
    ) -> Result<CardType, GameError>;

    /// Ends player's turn
    fn end_player_turn(&mut self, id: PlayerId) -> Result<TurnEnded, GameError>;

    /// Gets a list of players with publicly available information, besides the main player
    fn player_info(&self, id: PlayerId) -> Result<Vec<PlayerInfo>, GameError>;

    /// Gets a list of `PlayerId`s in the order of their respective turns.
    fn turn_order(&self) -> Result<Vec<PlayerId>, GameError>;
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

    pub fn selecting_characters(&self) -> Result<&SelectingCharacters, GameError> {
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

    pub fn results(&self) -> Result<&Results, GameError> {
        match self {
            Self::Results(r) => Ok(r),
            _ => Err(GameError::NotResultsState),
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::Lobby(Lobby::default())
    }
}

impl TheBottomLine for GameState {
    fn join(&mut self, username: String) -> Result<bool, GameError> {
        match self {
            Self::Lobby(lobby) => Ok(lobby.join(username)),
            _ => Err(GameError::NotLobbyState),
        }
    }

    fn leave(&mut self, username: &str) -> Result<bool, GameError> {
        match self {
            Self::Lobby(lobby) => Ok(lobby.leave(username)),
            _ => Err(GameError::NotLobbyState),
        }
    }

    fn start_game<P: AsRef<Path>>(&mut self, data_path: P) -> Result<(), GameError> {
        match self {
            Self::Lobby(lobby) => {
                *self = lobby.start_game(data_path)?;
                Ok(())
            }
            _ => Err(GameError::NotLobbyState),
        }
    }

    fn currently_selecting_id(&self) -> Option<PlayerId> {
        match self {
            Self::SelectingCharacters(s) => Some(s.currently_selecting_id()),
            _ => Err(GameError::NotSelectingCharactersState).ok(),
        }
    }

    fn is_selecting_characters(&self) -> bool {
        matches!(self, Self::SelectingCharacters(_))
    }

    fn player_select_character(
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

    fn player_get_selectable_characters(
        &self,
        id: PlayerId,
    ) -> Result<PickableCharacters, GameError> {
        let selecting = match self {
            Self::SelectingCharacters(s) => s,
            _ => return Err(GameError::NotSelectingCharactersState),
        };

        match selecting.currently_selecting_id() == id {
            true => match selecting.player(id) {
                Ok(_) => selecting.characters.peek().map_err(Into::into),
                Err(e) => Err(e),
            },
            false => Err(GameError::NotPlayersTurn),
        }
    }

    fn open_characters(&self) -> Result<&[Character], GameError> {
        match self {
            Self::SelectingCharacters(s) => Ok(s.open_characters()),
            Self::Round(r) => Ok(r.open_characters()),
            GameState::Lobby(_) => Err(GameError::NotAvailableInLobbyState),
            GameState::Results(_) => Err(GameError::NotAvailableInResultsState),
        }
    }

    fn player(&self, id: PlayerId) -> Result<&Player, GameError> {
        match self {
            Self::SelectingCharacters(s) => s.player(id),
            Self::Round(r) => r.player(id),
            Self::Results(r) => r.player(id),
            Self::Lobby(_) => Err(GameError::NotAvailableInLobbyState),
        }
    }

    fn current_player(&self) -> Result<&Player, GameError> {
        match self {
            Self::Round(r) => Ok(r.current_player()),
            _ => Err(GameError::NotRoundState),
        }
    }

    fn next_player(&self) -> Option<&Player> {
        match self {
            Self::Round(r) => r.next_player(),
            _ => None,
        }
    }

    fn player_by_name(&self, name: &str) -> Result<&Player, GameError> {
        match self {
            Self::SelectingCharacters(s) => s.player_by_name(name),
            Self::Round(r) => r.player_by_name(name),
            Self::Results(r) => r.player_by_name(name),
            Self::Lobby(_) => Err(GameError::NotAvailableInLobbyState),
        }
    }

    fn player_from_character(&self, character: Character) -> Option<&Player> {
        let players = match self {
            Self::SelectingCharacters(s) => &s.players,
            Self::Round(r) => &r.players,
            Self::Results(r) => &r.players,
            Self::Lobby(_) => return None,
        };

        players.iter().find(|p| p.character == Some(character))
    }

    fn player_play_card(
        &mut self,
        id: PlayerId,
        card_idx: usize,
    ) -> Result<PlayerPlayedCard, GameError> {
        match self {
            Self::Round(r) => r.player_play_card(id, card_idx),
            _ => Err(GameError::NotRoundState),
        }
    }

    fn player_draw_card(
        &mut self,
        id: PlayerId,
        card_type: CardType,
    ) -> Result<Either<&Asset, &Liability>, GameError> {
        let round = match self {
            Self::Round(r) => r,
            _ => return Err(GameError::NotRoundState),
        };

        match round.players.get_mut(usize::from(id)) {
            Some(player) if player.id == round.current_player => {
                if player.can_draw_cards() {
                    let card = match card_type {
                        CardType::Asset => Either::Left(round.assets.draw()),
                        CardType::Liability => Either::Right(round.liabilities.draw()),
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

    fn player_give_back_card(
        &mut self,
        id: PlayerId,
        card_idx: usize,
    ) -> Result<CardType, GameError> {
        let round = match self {
            Self::Round(r) => r,
            _ => return Err(GameError::NotRoundState),
        };

        match round.players.get_mut(usize::from(id)) {
            Some(player) if player.id == round.current_player => {
                if player.should_give_back_cards() {
                    match player.give_back_card(card_idx)? {
                        Either::Left(asset) => {
                            round.assets.put_back(asset);
                            Ok(CardType::Asset)
                        }
                        Either::Right(liability) => {
                            round.liabilities.put_back(liability);
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

    fn end_player_turn(&mut self, id: PlayerId) -> Result<TurnEnded, GameError> {
        let round = match self {
            Self::Round(r) => r,
            _ => return Err(GameError::NotRoundState),
        };

        match round.player(id) {
            Ok(current)
                if current.id == round.current_player && !current.should_give_back_cards() =>
            {
                if let Some(player) = round.next_player() {
                    round.current_player = player.id;
                    Ok(TurnEnded::new(Some(round.current_player)))
                } else {
                    let maybe_ceo = round.player_from_character(Character::CEO);
                    let chairman_id = match maybe_ceo.map(|p| p.id) {
                        Some(id) => id,
                        None => round.chairman,
                    };
                    round.players.iter_mut().for_each(|p| {
                        p.character = None;
                    });

                    let characters = ObtainingCharacters::new(round.players.len(), chairman_id);
                    let players = std::mem::take(&mut round.players);
                    let assets = std::mem::take(&mut round.assets);
                    let liabilities = std::mem::take(&mut round.liabilities);
                    let markets = std::mem::take(&mut round.markets);
                    let current_market = std::mem::take(&mut round.current_market);
                    let current_events = std::mem::take(&mut round.current_events);

                    *self = Self::SelectingCharacters(SelectingCharacters {
                        players,
                        characters,
                        assets,
                        liabilities,
                        markets,
                        chairman: chairman_id,
                        current_market,
                        current_events,
                    });

                    Ok(TurnEnded::new(None))
                }
            }
            Ok(_) => Err(GameError::NotPlayersTurn),
            Err(e) => Err(e),
        }
    }

    fn player_info(&self, id: PlayerId) -> Result<Vec<PlayerInfo>, GameError> {
        match self {
            Self::SelectingCharacters(s) => Ok(s.player_info(id)),
            Self::Round(r) => Ok(r.player_info(id)),
            Self::Results(r) => Ok(r.player_info(id)),
            Self::Lobby(_) => Err(GameError::NotAvailableInLobbyState),
        }
    }

    fn turn_order(&self) -> Result<Vec<PlayerId>, GameError> {
        match self {
            Self::SelectingCharacters(s) => Ok(s.turn_order()),
            _ => Err(GameError::NotSelectingCharactersState),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Lobby {
    players: HashSet<String>,
}

impl Lobby {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn players(&self) -> &HashSet<String> {
        &self.players
    }

    pub fn join(&mut self, username: String) -> bool {
        self.players.insert(username)
    }

    pub fn leave(&mut self, username: &str) -> bool {
        self.players.remove(username)
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

    pub fn init_players(
        &self,
        assets: &mut Deck<Asset>,
        liabilities: &mut Deck<Liability>,
    ) -> Vec<Player> {
        self.players
            .iter()
            .zip(0u8..)
            .map(|(name, i)| {
                let assets = [assets.draw(), assets.draw()];
                let liabilities = [liabilities.draw(), liabilities.draw()];
                Player::new(name, i, assets, liabilities, 1)
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
    players: Vec<Player>,
    characters: ObtainingCharacters,
    assets: Deck<Asset>,
    liabilities: Deck<Liability>,
    markets: Deck<Either<Market, Event>>,
    pub chairman: PlayerId,
    current_market: Market,
    current_events: Vec<Event>,
}

impl SelectingCharacters {
    fn player(&self, id: PlayerId) -> Result<&Player, GameError> {
        self.players
            .get(usize::from(id))
            .ok_or(GameError::InvalidPlayerIndex(id.0))
    }

    pub fn player_by_name(&self, name: &str) -> Result<&Player, GameError> {
        self.players
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| GameError::InvalidPlayerName(name.to_owned()))
    }

    fn currently_selecting_id(&self) -> PlayerId {
        (self.characters.applies_to_player() as u8).into()
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
            .flat_map(|p| p.id.ne(&id).then_some(p.info()))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct Round {
    current_player: PlayerId,
    players: Vec<Player>,
    assets: Deck<Asset>,
    liabilities: Deck<Liability>,
    markets: Deck<Either<Market, Event>>,
    pub chairman: PlayerId,
    current_market: Market,
    current_events: Vec<Event>,
    open_characters: Vec<Character>,
}

impl Round {
    fn player(&self, id: PlayerId) -> Result<&Player, GameError> {
        self.players
            .get(usize::from(id))
            .ok_or(GameError::InvalidPlayerIndex(id.0))
    }

    fn player_from_character(&self, character: Character) -> Option<&Player> {
        self.players.iter().find(|p| p.character == Some(character))
    }

    pub fn player_by_name(&self, name: &str) -> Result<&Player, GameError> {
        self.players
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| GameError::InvalidPlayerName(name.to_owned()))
    }

    pub fn current_player(&self) -> &Player {
        self.player(self.current_player)
            .expect("self.current_player went out of bounds")
    }

    fn next_player(&self) -> Option<&Player> {
        let current_character = self.current_player().character;
        self.players
            .iter()
            .filter(|p| p.character > current_character)
            .min_by(|p1, p2| p1.character.cmp(&p2.character))
    }

    pub fn open_characters(&self) -> &[Character] {
        &self.open_characters
    }

    pub fn player_info(&self, id: PlayerId) -> Vec<PlayerInfo> {
        self.players
            .iter()
            .flat_map(|p| p.id.ne(&id).then_some(p.info()))
            .collect()
    }

    fn player_play_card(
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
    players: Vec<Player>,
    final_market: Market,
    // TODO: implement events
    _final_events: Vec<Event>,
}

impl Results {
    pub fn player(&self, id: PlayerId) -> Result<&Player, GameError> {
        self.players
            .get(usize::from(id))
            .ok_or(GameError::InvalidPlayerIndex(id.0))
    }

    pub fn player_by_name(&self, name: &str) -> Result<&Player, GameError> {
        self.players
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| GameError::InvalidPlayerName(name.to_owned()))
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
            .flat_map(|p| p.id.ne(&id).then_some(p.info()))
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
                .map(|p| {
                    (
                        p.character.expect("There is a player without a character"),
                        p.id,
                    )
                })
                .for_each(|(c, id)| {
                    let p = game
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
                    let p = game.player_by_name(name).expect("couldn't find name");

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
                    let current_player = game
                        .current_player()
                        .expect("couldn't get current player")
                        .id;

                    card_types.into_iter().for_each(|card_type| {
                        assert_ok!(game.player_draw_card(current_player, card_type));
                    });

                    assert_matches!(
                        game.player_draw_card(current_player, too_many),
                        Err(GameError::DrawCard(DrawCardError::MaximumCardsDrawn(_)))
                    );
                });
        }
    }

    #[test]
    fn player_draw_card_invalid_id() {
        let mut game = pick_with_players(4).expect("couldn't pick characters");

        assert_matches!(
            game.player_draw_card(u8::MAX.into(), CardType::Asset),
            Err(GameError::InvalidPlayerIndex(_))
        );
    }

    #[test]
    fn player_draw_card_not_turn() {
        let mut game = pick_with_players(4).expect("couldn't pick characters");
        // This is not the current player
        let next_player = game.next_player().expect("couldn't get next player");

        assert_matches!(
            game.player_draw_card(next_player.id, CardType::Asset),
            Err(GameError::NotPlayersTurn)
        )
    }

    #[test]
    fn player_play_card() {
        for i in 4..=7 {
            let mut game = pick_with_players(i).expect("couldn't pick characters");
            // let round = game.round().unwrap();
            let current_player = game
                .current_player()
                .expect("couldn't get current player")
                .id;

            draw_cards(
                &mut game,
                current_player,
                [CardType::Asset, CardType::Asset, CardType::Liability],
            );

            // so player can always afford the asset
            match &mut game {
                GameState::Round(r) => {
                    r.players[usize::from(current_player)].cash = 50;
                }
                _ => panic!("Not round even though that's expected"),
            };

            // test issuing liability
            let player = &game.round().unwrap().player(current_player).unwrap();
            let hand_len = player.hand.len();
            let liability_value = player.hand[hand_len - 1]
                .as_ref()
                .right()
                .expect("Couldn't get liability")
                .value;
            let cash_before = player.cash;

            assert_ok!(game.player_play_card(current_player, hand_len - 1));
            assert_eq!(
                cash_before + liability_value,
                game.round().unwrap().player(current_player).unwrap().cash
            );

            assert_eq!(
                hand_len - 1,
                game.round()
                    .unwrap()
                    .player(current_player)
                    .unwrap()
                    .hand
                    .len()
            );

            // test buying asset
            let player = &game.round().unwrap().player(current_player).unwrap();
            let hand_len = player.hand.len();
            let liability_value = player.hand[hand_len - 1]
                .as_ref()
                .left()
                .expect("Couldn't get asset")
                .gold_value;
            let cash_before = player.cash;

            assert_ok!(game.player_play_card(current_player, hand_len - 1));
            assert_eq!(
                cash_before - liability_value,
                game.round().unwrap().player(current_player).unwrap().cash
            );

            assert_eq!(
                hand_len - 1,
                game.round()
                    .unwrap()
                    .player(current_player)
                    .unwrap()
                    .hand
                    .len()
            );

            let hand_len = game
                .round()
                .unwrap()
                .player(current_player)
                .unwrap()
                .hand
                .len();
            assert_matches!(
                game.player_play_card(current_player, hand_len - 1),
                Err(GameError::PlayCard(PlayCardError::ExceedsMaximumAssets))
            );
            assert_matches!(
                game.player_play_card(current_player, hand_len - 2),
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

        assert_matches!(
            game.player_play_card(u8::MAX.into(), 0),
            Err(GameError::InvalidPlayerIndex(_))
        )
    }

    #[test]
    fn player_play_card_not_turn() {
        let mut game = pick_with_players(4).expect("couldn't pick characters");
        // This is not the current player
        let next_player = game.next_player().expect("couldn't get next player");

        assert_matches!(
            game.player_play_card(next_player.id, 0),
            Err(GameError::NotPlayersTurn)
        )
    }

    #[test]
    fn end_player_turn_no_actions() {
        let mut game = pick_with_players(4).expect("couldn't pick characters");
        let current_player = game
            .current_player()
            .expect("couldn't get current player")
            .id;

        assert_ok!(game.end_player_turn(current_player));
    }

    #[test]
    fn end_player_turn_used_cards() {
        let mut game = pick_with_players(4).expect("couldn't pick characters");
        let current_player = game
            .current_player()
            .expect("couldn't get current player")
            .id;

        // so player can always afford the asset
        match &mut game {
            GameState::Round(r) => {
                r.players[usize::from(current_player)].cash = 50;
            }
            _ => panic!("Not round even though that's expected"),
        };

        let hand_len = game.round().unwrap().players[usize::from(current_player)]
            .hand
            .len();
        assert_ok!(game.player_play_card(current_player, hand_len - 1));
        assert_ok!(game.player_play_card(current_player, 0));

        assert_ok!(game.end_player_turn(current_player));
    }

    #[test]
    fn end_player_turn_drew_three_cards() {
        let mut game = pick_with_players(4).expect("couldn't pick characters");
        let current_player = game
            .current_player()
            .expect("couldn't get current player")
            .id;

        draw_cards(
            &mut game,
            current_player,
            [CardType::Asset, CardType::Asset, CardType::Liability],
        );

        assert_err!(game.end_player_turn(current_player));

        let hand_len = game.round().unwrap().players[usize::from(current_player)]
            .hand
            .len();
        assert_ok!(game.player_give_back_card(current_player, hand_len - 1));

        assert_ok!(game.end_player_turn(current_player));
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

    fn draw_cards<const N: usize>(game: &mut GameState, id: PlayerId, cards: [CardType; N]) {
        for card_type in cards {
            let _ = game.player_draw_card(id, card_type);
        }
    }

    fn pick_with_players(player_count: usize) -> Result<GameState, GameError> {
        let mut game = GameState::new();

        (0..player_count)
            .map(|i| format!("Player {i}"))
            .for_each(|name| assert_matches!(game.join(name), Ok(true)));

        game.start_game("assets/cards/boardgame.json")?;

        let add = match player_count {
            4..=6 => 1,
            7 => 0,
            _ => unreachable!(),
        };

        #[allow(unused)]
        let mut closed = None::<Character>;

        match game.player_get_selectable_characters(0.into()) {
            Ok(PickableCharacters {
                characters,
                closed_character,
            }) => {
                assert_eq!(characters.len(), player_count + add);
                assert_some!(closed_character);
                assert_ok!(game.player_select_character(0.into(), characters[0]));

                closed = closed_character;
            }
            _ => panic!(),
        }

        for i in 1..(player_count - 1) {
            match game.player_get_selectable_characters(PlayerId(i as u8)) {
                Ok(PickableCharacters {
                    characters,
                    closed_character,
                }) => {
                    assert_eq!(characters.len(), player_count + add - i);
                    assert_none!(closed_character);
                    assert_ok!(game.player_select_character(PlayerId(i as u8), characters[0]));
                }
                _ => panic!(),
            }
        }

        match game.player_get_selectable_characters(PlayerId((player_count - 1) as u8)) {
            Ok(PickableCharacters {
                characters,
                closed_character,
            }) => {
                assert_eq!(characters.len(), 2 + add);
                assert_none!(closed_character);
                assert!(characters.contains(&closed.unwrap()));
                assert_ok!(
                    game.player_select_character(
                        PlayerId((player_count - 1) as u8),
                        closed.unwrap()
                    )
                );

                assert!(!game.is_selecting_characters());
                assert_ok!(game.current_player());
                assert_matches!(game, GameState::Round(_));
                assert_ok!(game.round());

                Ok(game)
            }
            _ => panic!(),
        }
    }
}
