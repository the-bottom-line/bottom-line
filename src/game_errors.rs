use std::ops::Deref;

use thiserror::Error;

/// The main error struct of the game logic. Internally it uses a Box containing
/// [`GameErrorInner`](crate::GameErrorInner) to save space.
#[derive(Debug, Clone, PartialEq, Error)]
#[error("{0}")]
pub struct GameError(#[source] Box<GameErrorInner>);

impl From<GameErrorInner> for GameError {
    fn from(value: GameErrorInner) -> Self {
        Self(Box::new(value))
    }
}

impl Deref for GameError {
    type Target = GameErrorInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum GameErrorInner {
    
}

