mod api;
mod cli;
mod conf;
mod events;
mod frontend;
mod scheduler;
mod storage;
mod object_store;

#[tokio::main]
async fn main() {
    cli::init().await;
}
