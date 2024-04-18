#![allow(dead_code)]

mod extension;

fn main() {
    let path = std::path::PathBuf::from("./openapi.json");
    extension::write_openapi_spec(path).unwrap();
}
