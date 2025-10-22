use std::{collections::HashSet, sync::Arc, vec};

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

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
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
        }
    }

    pub fn total_gold(&self) -> u8 {
        self.assets.iter().map(|a| a.gold_value).sum()
    }

    pub fn total_silver(&self) -> u8 {
        self.assets.iter().map(|a| a.silver_value).sum()
    }

    pub fn info(&self) -> PlayerInfo {
        self.into()
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
                Either::Left(a) if self.assets_to_play > 0 && self.cash >= a.gold_value => {
                    let asset = self.hand.remove(card_idx).left().unwrap();
                    self.cash -= asset.gold_value;
                    self.assets_to_play -= 1;
                    self.assets.push(asset.clone());
                    Ok(Either::Left(asset))
                }
                Either::Left(_) if self.assets_to_play == 0 => Err(ExceedsMaximumAssets),
                Either::Left(a) if self.cash < a.gold_value => Err(CannotAffordAsset {
                    cash: self.cash,
                    cost: a.gold_value,
                }),
                Either::Right(_) if self.liabilities_to_play > 0 => {
                    let liability = self.hand.remove(card_idx).right().unwrap();
                    self.liabilities_to_play -= 1;
                    self.liabilities.push(liability.clone());
                    Ok(Either::Right(liability))
                }
                Either::Right(_) if self.liabilities_to_play == 0 => Err(ExceedsMaximumLiabilities),
                _ => unreachable!(),
            }
        } else {
            Err(InvalidCardIndex(card_idx as u8))
        }
    }

    pub fn draw_card(&mut self, card: Either<Asset, Liability>) {
        self.cards_drawn.push(self.hand.len());
        self.hand.push(card);
    }

    pub fn give_back_card(
        &mut self,
        card_idx: usize,
    ) -> Result<Either<Asset, Liability>, GiveBackCardError> {
        if let Some(_) = self.hand.get(card_idx) {
            Ok(self.hand.remove(card_idx))
        } else {
            Err(GiveBackCardError::InvalidCardIndex(card_idx as u8))
        }
    }

    pub fn should_give_back_card(&self) -> bool {
        let limit = match self.character {
            Some(Character::HeadRnD) => 4,
            Some(_) => 2,
            _ => 0,
        };

        self.cards_drawn.len() > limit
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    pub fn shuffle(&mut self) {
        let mut rng = rand::rng();
        self.deck.shuffle(&mut rng);
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
                characters: self.available_characters.deck.iter().cloned().collect(),
                closed_character: Some(self.closed_character),
            }),
            n if n < self.player_count - 1 => Ok(PickableCharacters {
                characters: self.available_characters.deck.iter().cloned().collect(),
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

    pub fn next(&mut self) -> Result<PickableCharacters, SelectableCharactersError> {
        self.draw_idx += 1;

        self.peek()
    }
    pub fn applies_to_player(&self) -> usize {
        (self.draw_idx + self.chairman_id) % self.player_count
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
    /// Checks if the game is in a selecting characters phase, which happens before each round
    /// starts.
    fn is_selecting_characters(&self) -> bool;

    /// Gets the character of the current turn.
    fn current_player(&self) -> Option<&Player>;

    /// Gets the character of the next turn
    fn next_player(&self) -> Option<&Player>;

    /// Gets a player object based on a given username
    fn player_by_name(&self, name: &str) -> Option<&Player>;

    /// Gets player if one exists with specified character
    fn player_from_character(&self, character: Character) -> Option<&Player>;

    fn chairman(&self) -> &Player;

    /// Gets list of selectable caracters if its the players turn
    fn player_get_selectable_characters(
        &self,
        player_idx: usize,
    ) -> Result<PickableCharacters, GameError>;

    /// Assigns a character role to a specific player. Returns a set of pickable characters for the
    /// next player to choose from
    fn player_select_character(
        &mut self,
        player_idx: usize,
        character: Character,
    ) -> Result<(), GameError>;

    /// Attempts to play a card (either an asset or liability) for player with `player_idx`. If
    /// playing this card triggers a market change, returns an object with a list of events and
    /// a new market.
    fn player_play_card(
        &mut self,
        player_idx: usize,
        card_idx: usize,
    ) -> Result<PlayerPlayedCard, GameError>;

    fn player_draw_card(
        &mut self,
        player_idx: usize,
        card_type: CardType,
    ) -> Result<Either<&Asset, &Liability>, GameError>;

    /// When the player grabs 3 cards, the player should give back one.
    fn player_give_back_card(
        &mut self,
        player_idx: usize,
        card_idx: usize,
    ) -> Result<CardType, GameError>;

    /// Ends player's turn
    fn end_player_turn(&mut self, player_idx: usize) -> Result<TurnEnded, GameError>;

    /// Gets a list of players with publicly available information, besides the main player
    fn player_info(&self, player_idx: usize) -> Vec<PlayerInfo>;

    /// Gets a list of `PlayerId`s in the order of their respective turns.
    fn turn_order(&self) -> Vec<PlayerId>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    assets: Deck<Asset>,
    liabilities: Deck<Liability>,
    market_deck: Deck<Either<Market, Event>>,
    characters: ObtainingCharacters,
    players: Vec<Player>,
    current_player: Option<PlayerId>,
    chairman: PlayerId,
    current_market: Market,
    current_events: Vec<Event>,
    highest_amount_of_assets: u8,
}

impl GameState {
    pub fn new(player_names: &[String], mut game_data: GameData) -> Self {
        game_data.shuffle_all();

        let current_market = Self::get_first_market(&mut game_data.market_deck)
            .expect("The default deck should have a market");

        let players = Self::get_players(
            player_names,
            &mut game_data.assets,
            &mut game_data.liabilities,
        );

        let characters =
            ObtainingCharacters::new(player_names.len(), players.first().unwrap().id.into());

        GameState {
            players,
            characters,
            assets: game_data.assets,
            liabilities: game_data.liabilities,
            market_deck: game_data.market_deck,
            current_market,
            current_player: None,
            current_events: vec![],
            chairman: PlayerId(0),
            highest_amount_of_assets: 0,
        }
    }

    /// Grab market card if available and reshuffles the rest of the deck.
    fn get_first_market(deck: &mut Deck<Either<Market, Event>>) -> Option<Market> {
        let mut rng = rand::rng();

        if let Some(pos) = deck.deck.iter().position(|c| c.is_left()) {
            let market = deck.deck.swap_remove(pos).left();
            deck.deck.shuffle(&mut rng);
            market
        } else {
            None
        }
    }

    fn get_players(
        player_names: &[String],
        assets: &mut Deck<Asset>,
        liabilites: &mut Deck<Liability>,
    ) -> Vec<Player> {
        let player_count = player_names.len();
        assert!(
            player_count >= 4 && player_count <= 7,
            "This game supports playing with 4 to 7 players"
        );

        player_names
            .into_iter()
            .zip(0u8..)
            .map(|(name, i)| {
                let assets = [assets.draw(), assets.draw()];
                let liabilities = [liabilites.draw(), liabilites.draw()];
                Player::new(&name, i, assets, liabilities, 1)
            })
            .collect()
    }

    fn check_new_market(&self) -> bool {
        let max_asset_count = self
            .players
            .iter()
            .map(|player| player.assets.len() as u8)
            .max()
            .unwrap_or_default();

        max_asset_count > self.highest_amount_of_assets
    }

    /// Starts a new market. Automatically triggers if any player gets the first, second, third, fourth, fifth, seventh or eight asset. Loops through the deck and fetches events as they come.
    fn new_market(&mut self) -> MarketChange {
        let mut events = vec![];

        loop {
            match self.market_deck.draw() {
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

impl TheBottomLine for GameState {
    fn is_selecting_characters(&self) -> bool {
        self.characters.draw_idx < self.players.len() - 1
    }

    fn player_select_character(
        &mut self,
        player_idx: usize,
        character: Character,
    ) -> Result<(), GameError> {
        use SelectableCharactersError::*;

        if let Some(_) = self.players.get(player_idx) {
            match (
                self.is_selecting_characters(),
                player_idx == self.characters.applies_to_player(),
            ) {
                (true, true) => {
                    self.players
                        .get_mut(player_idx)
                        .unwrap()
                        .select_character(character);
                    self.characters.next()?;
                    Ok(())
                }
                (true, false) => Err(GameError::NotPlayersTurn.into()),
                _ => Err(NotPickingCharacters.into()),
            }
        } else {
            Err(GameError::InvalidPlayerIndex(player_idx as u8).into())
        }
    }

    fn player_get_selectable_characters(
        &self,
        player_idx: usize,
    ) -> Result<PickableCharacters, GameError> {
        match (
            self.is_selecting_characters(),
            player_idx == self.characters.applies_to_player(),
        ) {
            (true, true) => {
                if let Some(_) = self.players.get(player_idx) {
                    self.characters.peek().map_err(Into::into)
                } else {
                    Err(GameError::InvalidPlayerIndex(player_idx as u8))
                }
            }
            (true, false) => Err(GameError::NotPlayersTurn),
            _ => Err(SelectableCharactersError::NotPickingCharacters.into()),
        }
    }

    fn current_player(&self) -> Option<&Player> {
        if let Some(id) = self.current_player {
            self.players.get(usize::from(id))
        } else {
            None
        }
    }

    fn next_player(&self) -> Option<&Player> {
        if let Some(current) = self.current_player() {
            self.players
                .iter()
                .filter(|p| p.character > current.character)
                .min_by(|p1, p2| p1.character.cmp(&p2.character))
        } else {
            None
        }
    }

    fn player_by_name(&self, name: &str) -> Option<&Player> {
        self.players.iter().find(|p| p.name == name)
    }

    fn player_from_character(&self, character: Character) -> Option<&Player> {
        self.players.iter().find(|p| p.character == Some(character))
    }

    fn chairman(&self) -> &Player {
        &self.players[usize::from(self.chairman)]
    }

    fn player_play_card(
        &mut self,
        idx: usize,
        card_idx: usize,
    ) -> Result<PlayerPlayedCard, GameError> {
        let current_character = self.current_player().unwrap().character;

        if let Some(player) = self.players.get_mut(idx) {
            if player.character == current_character {
                match player.play_card(card_idx)? {
                    Either::Left(asset) => {
                        let market = match self.check_new_market() {
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
            } else {
                Err(GameError::NotPlayersTurn.into())
            }
        } else {
            Err(GameError::InvalidPlayerIndex(idx as u8).into())
        }
    }

    fn player_draw_card(
        &mut self,
        player_idx: usize,
        card_type: CardType,
    ) -> Result<Either<&Asset, &Liability>, GameError> {
        if let Some(player) = self.players.get_mut(player_idx) {
            if self.current_player == Some(player.id) {
                if player.cards_drawn.len() < 3 {
                    let card = match card_type {
                        CardType::Asset => Either::Left(self.assets.draw()),
                        CardType::Liability => Either::Right(self.liabilities.draw()),
                    };
                    player.draw_card(card);
                    Ok(player.hand.last().unwrap().as_ref())
                } else {
                    Err(DrawCardError::MaximumCardsDrawn(player.cards_drawn.len() as u8).into())
                }
            } else {
                Err(GameError::NotPlayersTurn)
            }
        } else {
            Err(GameError::InvalidPlayerIndex(player_idx as u8))
        }
    }

    fn player_give_back_card(
        &mut self,
        player_idx: usize,
        card_idx: usize,
    ) -> Result<CardType, GameError> {
        if let Some(player) = self.players.get_mut(player_idx) {
            if self.current_player == Some(player.id) {
                if player.should_give_back_card() {
                    match player.give_back_card(card_idx)? {
                        Either::Left(asset) => {
                            self.assets.deck.insert(0, asset);
                            Ok(CardType::Asset)
                        }
                        Either::Right(liability) => {
                            self.liabilities.deck.insert(0, liability);
                            Ok(CardType::Liability)
                        }
                    }
                } else {
                    Err(GiveBackCardError::Unnecessary.into())
                }
            } else {
                Err(GameError::NotPlayersTurn)
            }
        } else {
            Err(GameError::InvalidPlayerIndex(player_idx as u8))
        }
    }

    fn end_player_turn(&mut self, player_idx: usize) -> Result<TurnEnded, GameError> {
        if let Some(player) = self.players.get(player_idx) {
            if self.current_player == Some(player.id) && !player.should_give_back_card() {
                if let Some(player) = self.next_player() {
                    self.current_player = Some(player.id);
                    Ok(TurnEnded::new(self.current_player))
                } else {
                    let maybe_ceo = self.player_from_character(Character::CEO);
                    let chairman_id = match maybe_ceo.map(|p| p.id) {
                        Some(id) => id,
                        None => self.chairman,
                    };
                    self.current_player = None;
                    self.players.iter_mut().for_each(|p| {
                        p.character = None;
                    });
                    self.characters = ObtainingCharacters::new(self.players.len(), chairman_id);
                    Ok(TurnEnded::new(None))
                }
            } else {
                Err(GameError::PlayerShouldGiveBackCard)
            }
        } else {
            Err(GameError::InvalidPlayerIndex(player_idx as u8))
        }
    }

    fn player_info(&self, player_idx: usize) -> Vec<PlayerInfo> {
        self.players
            .iter()
            .flat_map(|p| p.id.0.eq(&(player_idx as u8)).then_some(p.info()))
            .collect()
    }

    fn turn_order(&self) -> Vec<PlayerId> {
        let start = usize::from(self.chairman) as u8;
        let limit = self.players.len() as u8;
        (start..limit)
            .into_iter()
            .chain(0..start)
            .map(Into::into)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draw_cards() {
        let data = GameData::new("assets/cards/boardgame.json").expect("this should exist");
        let game = GameState::new(
            &[
                "your".to_owned(),
                "mama".to_owned(),
                "joe".to_owned(),
                "biden".to_owned(),
            ],
            data,
        );

        let json = serde_json::to_string(&game).unwrap();
        println!("{json}");
    }
}
