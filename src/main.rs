mod cards;
mod game;
mod request_handler;
mod server;

use game::*;
use server::*;

#[tokio::main]
async fn main() {
    setupsocket().await;
}
