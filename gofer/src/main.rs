mod api;
mod cli;
mod conf;
mod object_store;
mod scheduler;
mod secret_store;
mod storage;

use human_panic::setup_panic;
use polyfmt::error;

#[tokio::main]
async fn main() {
    setup_panic!();

    let mut cli = match cli::Cli::new() {
        Ok(cli) => cli,
        Err(e) => {
            error!("{:?}", e);
            std::process::exit(1)
        }
    };

    match cli.run().await {
        Ok(_) => {}
        Err(e) => {
            error!("{:?}", e);
            std::process::exit(1)
        }
    }
}
