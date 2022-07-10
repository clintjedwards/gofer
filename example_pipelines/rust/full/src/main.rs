use gofer_sdk;

fn main() {
    gofer_sdk::config::Pipeline::new("test_pipeline", "hello")
        .description("weowa")
        .finish();
}
