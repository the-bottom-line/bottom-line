mod cards;
mod game;
mod game_errors;
mod request_handler;
mod server;
mod utility;

use game::*;
use server::*;

#[tokio::main]
async fn main() {
    setupsocket().await;
}
