use crate::{errors::GameError, game::*, player::*, utility::serde_asset_liability};
use either::Either;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum Connect {
    Connect { username: String, channel: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum ReceiveData {
    StartGame,
    SelectCharacter { character: Character },
    DrawCard { card_type: CardType },
    PutBackCard { card_idx: usize },
    BuyAsset { card_idx: usize },
    IssueLiability { card_idx: usize },
    EndTurn,
}

#[derive(Debug)]
pub struct Response(pub InternalResponse, pub DirectResponse);

impl From<GameError> for DirectResponse {
    fn from(error: GameError) -> Self {
        DirectResponse::Error(error.into())
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
        can_draw_cards: bool,
        can_give_back_cards: bool,
    },
    YouPutBackCard {
        card_idx: usize,
        can_draw_cards: bool,
        can_give_back_cards: bool,
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
        usernames: Vec<String>,
    },
    StartGame {
        id: PlayerId,
        cash: u8,
        #[serde(with = "serde_asset_liability::vec")]
        hand: Vec<Either<Asset, Liability>>,
        player_info: Vec<PlayerInfo>,
        open_characters: Vec<Character>,
    },
    SelectingCharacters {
        chairman_id: PlayerId,
        pickable_characters: Option<PickableCharacters>,
        turn_order: Vec<PlayerId>,
    },
    SelectedCharacter {
        currently_picking_id: Option<PlayerId>,
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
pub struct InternalResponse(pub [Option<Vec<UniqueResponse>>; 7]);

impl InternalResponse {
    pub fn get_responses(&self, idx: usize) -> Option<&[UniqueResponse]> {
        match self.0.get(idx) {
            Some(Some(v)) => Some(&v),
            _ => None,
        }
    }

    pub fn into_inner(self) -> [Option<Vec<UniqueResponse>>; 7] {
        self.0
    }
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum ResponseError {
    #[error(transparent)]
    Game(#[from] GameError),
    #[error("Game has not yet started")]
    GameNotYetStarted,
    #[error("Game has already started")]
    GameAlreadyStarted,
    #[error("Data is not valid for this state")]
    InvalidData,
}
