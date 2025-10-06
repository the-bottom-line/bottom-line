mod cards;
mod game;
mod server;

use server::*;
use game::*;

use crate::cards::GameData;

#[tokio::main]
async fn main() {
    setupsocket().await;
}
