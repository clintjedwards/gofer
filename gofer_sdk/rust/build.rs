use std::fs::canonicalize;
use std::path::PathBuf;

fn main() {
    let proto_path = canonicalize(PathBuf::from("../")).unwrap();
    let proto_path = proto_path.to_string_lossy();

    // Build protobuf files.
    tonic_build::configure()
        .out_dir("src")
        .compile(&["sdk.proto"], &[proto_path.to_string()])
        .expect("failed compiling protos");
}
