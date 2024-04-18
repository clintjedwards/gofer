#![allow(dead_code)]

mod api;
mod cli;
mod conf;
mod object_store;
mod scheduler;
mod secret_store;
mod storage;

fn main() {
    let path = std::path::PathBuf::from("docs/src/assets/openapi.json");
    api::write_openapi_spec(path).unwrap();
}
