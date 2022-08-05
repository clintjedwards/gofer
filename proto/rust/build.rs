fn main() {
    // Build protobuf files.
    tonic_build::configure()
        .out_dir("src")
        .compile(
            &[
                "gofer.proto",
                "gofer_transport.proto",
                "gofer_message.proto",
            ],
            &["../"],
        )
        .expect("failed compiling protos");
}
