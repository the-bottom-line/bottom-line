mod cards;
mod game;
mod server;
mod request_handler;

use server::*;
use game::*;

use crate::cards::GameData;

#[tokio::main]
async fn main() {
    setupsocket().await;
}
