mod api;
mod cli;
mod conf;
mod events;
mod frontend;
mod object_store;
mod scheduler;
mod secret_store;
mod storage;

#[tokio::main]
async fn main() {
    cli::init().await;
}
