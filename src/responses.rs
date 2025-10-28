use std::collections::HashSet;

use crate::{game::*, game_errors::GameError, utility::serde_asset_liability};
use either::Either;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum ReceiveData {
    Connect { username: String, channel: String },
    StartGame,
    SelectCharacter { character: Character },
    DrawCard { card_type: CardType },
    PutBackCard { card_idx: usize },
    BuyAsset { card_idx: usize },
    IssueLiability { card_idx: usize },
    EndTurn,
}

#[derive(Debug)]
pub struct Response(pub Option<InternalResponse>, pub DirectResponse);

impl Response {
    pub fn new(internal: InternalResponse, external: DirectResponse) -> Self {
        Self(Some(internal), external)
    }
}

impl From<DirectResponse> for Response {
    fn from(external: DirectResponse) -> Self {
        Self(None, external)
    }
}

impl From<GameError> for Response {
    fn from(error: GameError) -> Self {
        Self(None, DirectResponse::Error(error.into()))
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum DirectResponse {
    Error(ResponseError),
    YouStartedGame,
    YouSelectedCharacter {
        character: Character,
    },
    YouDrewCard {
        #[serde(with = "serde_asset_liability::value")]
        card: Either<Asset, Liability>,
    },
    YouPutBackCard {
        card_idx: usize,
    },
    YouBoughtAsset {
        asset: Asset,
    },
    YouIssuedLiability {
        liability: Liability,
    },
    YouEndedTurn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum UniqueResponse {
    PlayersInLobby {
        changed_player: String,
        usernames: HashSet<String>,
    },
    StartGame {
        cash: u8,
        #[serde(with = "serde_asset_liability::vec")]
        hand: Vec<Either<Asset, Liability>>,
        open_characters: Vec<Character>,
    },
    SelectingCharacters {
        chairman_id: PlayerId,
        pickable_characters: Option<PickableCharacters>,
        player_info: Vec<PlayerInfo>,
        turn_order: Vec<PlayerId>,
    },
    SelectedCharacter {
        player_id: PlayerId,
        pickable_characters: Option<PickableCharacters>,
    },
    TurnStarts {
        /// Id of the player whose turn it is
        player_turn: PlayerId,
        /// Extra cash received by the player whose turn it is
        player_turn_cash: u8,
        player_character: Character,
        draws_n_cards: u8,
        skipped_characters: Vec<Character>,
    },
    DrewCard {
        player_id: PlayerId,
        card_type: CardType,
    },
    PutBackCard {
        player_id: PlayerId,
        card_type: CardType,
    },
    BoughtAsset {
        player_id: PlayerId,
        asset: Asset,
    },
    IssuedLiability {
        player_id: PlayerId,
        liability: Liability,
    },
    TurnEnded {
        player_id: PlayerId,
    },
    /// Always sent with SelectingCharacters?
    RoundEnded,
    GameEnded,
}

#[derive(Clone, Debug)]
pub enum InternalResponse {
    PlayerJoined {
        username: String,
    },
    PlayerLeft {
        username: String,
    },
    GameStarted,
    SelectedCharacter {
        player_id: PlayerId,
    },
    DrawnCard {
        player_id: PlayerId,
        card_type: CardType,
    },
    PutBackCard {
        player_id: PlayerId,
        card_type: CardType,
    },
    BoughtAsset {
        player_id: PlayerId,
        asset: Asset,
    },
    IssuedLiability {
        player_id: PlayerId,
        liability: Liability,
    },
    TurnEnded {
        player_id: PlayerId,
    },
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum ResponseError {
    #[error(transparent)]
    Game(#[from] GameError),
    #[error("Game has not yet started")]
    GameNotYetStarted,
    #[error("Game has already started")]
    GameAlreadyStarted,
    #[error("Username already taken")]
    UsernameAlreadyTaken,
    #[error("Username is invalid")]
    InvalidUsername,
    #[error("Data is not valid for this state")]
    InvalidData,
}

impl<I: Into<ResponseError>> From<I> for DirectResponse {
    fn from(error: I) -> Self {
        DirectResponse::Error(error.into())
    }
}
