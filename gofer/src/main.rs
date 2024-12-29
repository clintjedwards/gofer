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

// This main function is set-up to help debug tokio tasks. Helpful when there is a blocking task, but the logs
// don't quite pinpoint where in the code it is.
// Steps to set up the debugger:
// 1) install tokio console: `cargo install --locked tokio-console`
// 2) comment out our `tracing-subscibe::fmt::init` in api/mod.rs
// 3) run `export RUSTFLAGS="--cfg tokio_unstable"` before running `make run`
// 4) wait till Gofer starts before running `tokio-console`

// fn main() {
//     console_subscriber::init();

//     let rt = tokio::runtime::Builder::new_current_thread()
//         .enable_all() // Enable IO, time, and other necessary components
//         .build()
//         .unwrap();

//     // Run your async function
//     rt.block_on(async {
//         setup_panic!();

//         let mut cli = match cli::Cli::new() {
//             Ok(cli) => cli,
//             Err(e) => {
//                 error!("{:?}", e);
//                 std::process::exit(1)
//             }
//         };

//         match cli.run().await {
//             Ok(_) => {}
//             Err(e) => {
//                 error!("{:?}", e);
//                 std::process::exit(1)
//             }
//         }
//     });
// }
