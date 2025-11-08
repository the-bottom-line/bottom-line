use server::server::setupsocket;

#[tokio::main]
async fn main() {
    setupsocket().await;
}
