fn main() {
    // Build protobuf files.
    tonic_build::configure()
        .out_dir("src")
        .compile(
            &[
                "gofer.proto",
                "gofer_transport.proto",
                "gofer_message_api.proto",
                "gofer_message_sdk.proto",
            ],
            &["../"],
        )
        .expect("failed compiling protos");
}
