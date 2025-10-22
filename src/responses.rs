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
pub struct Response(pub Option<InternalResponse>, pub PersonalResponse);

impl Response {
    pub fn new(internal: InternalResponse, external: PersonalResponse) -> Self {
        Self(Some(internal), external)
    }
}

impl From<PersonalResponse> for Response {
    fn from(external: PersonalResponse) -> Self {
        Self(None, external)
    }
}

impl From<GameError> for Response {
    fn from(error: GameError) -> Self {
        Self(None, PersonalResponse::Error(error.into()))
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "action", content = "data")]
pub enum PersonalResponse {
    Error(ResponseError),
    GameStarted,
    SelectedCharacter {
        character: Character,
    },
    DrawnCard {
        #[serde(with = "serde_asset_liability::value")]
        card: Either<Asset, Liability>,
    },
    PutBackCard {
        card_idx: usize,
    },
    BoughtAsset {
        asset: Asset,
    },
    IssuedLiability {
        liability: Liability,
    },
    EndedTurn,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "action", content = "data")]
pub enum TargetedResponse {
    PlayersInLobby {
        changed_player: String,
        usernames: HashSet<String>,
    },
    StartGame {
        cash: u8,
        #[serde(with = "serde_asset_liability::vec")]
        hand: Vec<Either<Asset, Liability>>,
    },
    SelectingCharacters {
        chairman_id: PlayerId,
        pickable_characters: Option<PickableCharacters>,
        player_info: Vec<PlayerInfo>,
        turn_order: Vec<PlayerId>,
    },
    SelectedCharacter {
        player_id: PlayerId,
        character: Character,
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
        character: Character,
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

#[derive(Debug, Error, Serialize)]
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

impl<I: Into<ResponseError>> From<I> for PersonalResponse {
    fn from(error: I) -> Self {
        PersonalResponse::Error(error.into())
    }
}
