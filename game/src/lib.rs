//! This crate contains everything one needs to represent the backend of the game _The Bottom
//! Line_, a card game in which players try to increase the value of their company.

#![warn(missing_docs)]

pub mod cards;
pub mod errors;
pub mod game;
pub mod player;
pub mod utility;

/// The folder containing all shared typescript types.
#[cfg(feature = "ts")]
pub static SHARED_TS_DIR: &str = concat!(std::env!("CARGO_MANIFEST_DIR"), "/../shared-ts/index.ts");
