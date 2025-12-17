//! This file contains the implementation of [`LobbyPlayer`].

use crate::player::*;

/// The player type corresponding to the [`Lobby`](crate::game::Lobby) state of the game.
#[derive(Debug, Clone, PartialEq)]
pub struct LobbyPlayer {
    id: PlayerId,
    name: String,
    is_human : bool,
}

impl LobbyPlayer {
    /// Instantiates a new lobby player based on an id and a name.
    pub fn new(id: PlayerId, name: String, is_human: bool) -> Self {
        Self { id, name, is_human }
    }

    /// Gets the id of the player
    pub fn id(&self) -> PlayerId {
        self.id
    }

    /// Sets the id of the player
    pub fn set_id(&mut self, id: PlayerId) {
        self.id = id;
    }

    /// Gets the name of the player
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets the humanity state of the player
    pub fn is_human(&self) -> bool {
        self.is_human
    }
}

impl From<&LobbyPlayer> for PlayerInfo {
    fn from(player: &LobbyPlayer) -> Self {
        Self {
            name: player.name().to_owned(),
            id: player.id(),
            ..Default::default()
        }
    }
}
