use std::{env, path::PathBuf};

fn main() {
    // Build protobuf files.
    tonic_build::configure()
        .file_descriptor_set_path(
            PathBuf::from(env::var("OUT_DIR").unwrap()).join("reflection.bin"),
        )
        .out_dir("src")
        .compile(
            &[
                "gofer.proto",
                "gofer_transport.proto",
                "gofer_message.proto",
            ],
            &["./"],
        )
        .expect("failed compiling protos");
}
