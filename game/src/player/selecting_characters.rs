//! This file contains the implementation of [`SelectingCharactersPlayer`].

use crate::player::*;

/// The player type that corresponds to the
/// [`SelectingCharacters`](crate::game::SelectingCharacters) stage of the game. In this stage,
/// players may not have a selected character yet.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectingCharactersPlayer {
    pub(super) id: PlayerId,
    pub(super) name: String,
    pub(super) cash: u8,
    pub(super) assets: Vec<Asset>,
    pub(super) liabilities: Vec<Liability>,
    pub(super) character: Option<Character>,
    pub(super) hand: Vec<Either<Asset, Liability>>,
    pub(super) is_human: bool,
}

impl SelectingCharactersPlayer {
    /// Gets the id of the player
    pub fn id(&self) -> PlayerId {
        self.id
    }

    /// Gets the name of the player
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets the amount of cash of the player
    pub fn cash(&self) -> u8 {
        self.cash
    }

    /// Gets a list of bought assets of the player
    pub fn assets(&self) -> &[Asset] {
        &self.assets
    }

    /// Gets a list of issued liabilities of the player
    pub fn liabilities(&self) -> &[Liability] {
        &self.liabilities
    }

    /// Gets the character for this player, if they've selected a chararacter yet
    pub fn character(&self) -> Option<Character> {
        self.character
    }

    /// Gets the hand of this player.
    pub fn hand(&self) -> &[Either<Asset, Liability>] {
        &self.hand
    }

    /// Gets the human state of this player
    pub fn is_human(&self) -> bool {
        self.is_human
    }

    /// Sets the human state of this player
    pub fn set_is_human(&mut self, human: bool) {
        self.is_human = human;
    }

    /// Constructs a new player id and a name, a certain hand and starting cash.
    pub(crate) fn new(
        name: String,
        id: PlayerId,
        assets: [Asset; 2],
        liabilities: [Liability; 2],
        cash: u8,
        is_human: bool,
    ) -> Self {
        let hand = assets
            .into_iter()
            .map(Either::Left)
            .chain(liabilities.into_iter().map(Either::Right))
            .collect();

        SelectingCharactersPlayer {
            id,
            name,
            cash,
            assets: vec![],
            liabilities: vec![],
            character: None,
            hand,
            is_human,
        }
    }

    /// Tries to select a character for the player. If the player has not selected a character yet,
    /// changes their character to `character`.
    pub fn select_character(
        &mut self,
        character: Character,
    ) -> Result<(), SelectingCharactersError> {
        match self.character {
            Some(c) => Err(SelectingCharactersError::AlreadySelectedCharacter(c)),
            None => {
                self.character = Some(character);
                Ok(())
            }
        }
    }
}

impl From<RoundPlayer> for SelectingCharactersPlayer {
    fn from(player: RoundPlayer) -> Self {
        Self {
            id: player.id,
            name: player.name,
            cash: player.cash,
            assets: player.assets,
            liabilities: player.liabilities,
            character: None,
            hand: player.hand,
            is_human: player.is_human,
        }
    }
}

impl From<&SelectingCharactersPlayer> for PlayerInfo {
    fn from(player: &SelectingCharactersPlayer) -> Self {
        Self {
            name: player.name.clone(),
            id: player.id,
            hand: Self::hand(&player.hand),
            assets: player.assets.clone(),
            liabilities: player.liabilities.clone(),
            cash: player.cash,
            character: player.character,
            is_human: player.is_human,
        }
    }
}
