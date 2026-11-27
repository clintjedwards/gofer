use gofer_sdk::config::{Pipeline, RequiredParentStatus, Task};
use std::time::Duration;

/// Fetches only the commit being tested into the run folder, so every task after it tests the same tree. The GitHub
/// extension passes in the commit that started the run. Without it, REF picks a branch, tag or commit.
const CHECKOUT: &str = r#"
set -eu
REF="${GOFER_EXTENSION_GITHUB_PULLREQUEST_HEAD_SHA:-${GOFER_EXTENSION_GITHUB_HEAD_COMMIT_ID:-$REF}}"

git init --quiet /gofer/run/src
cd /gofer/run/src
git remote add origin https://github.com/clintjedwards/gofer.git
git fetch --quiet --depth 1 origin "$REF"
git checkout --quiet FETCH_HEAD
echo "testing $REF at $(git rev-parse --short HEAD)"
"#;

const RUST_CHECKS: &str = "
set -eu
cd /gofer/run/src

# A commit that changes the pinned toolchain needs these for the new one too. It does nothing otherwise.
rustup component add clippy rustfmt

cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
";

const GO_CHECKS: &str = r#"
set -eu
cd /gofer/run/src

unformatted=$(gofmt -l sdk/go examplePipelines/go)
if [ -n "$unformatted" ]; then
    echo "these files need gofmt:"
    echo "$unformatted"
    exit 1
fi

(cd sdk/go && go vet ./... && go test ./...)

# The examples build against the SDK in this checkout, so they break when it changes in a way they don't expect.
for example in examplePipelines/go/*/; do
    (cd "$example" && go vet ./...)
done
"#;

fn main() {
    let mut pipeline = Pipeline::new("run-tests", "Run Gofer's Tests").description(
        "Runs the checks you'd run on your machine before pushing: formatting, lints and tests for the Rust \
workspace, and for the Go SDK and examples. It tests the commit the GitHub extension started the run for, or main \
when there isn't one.

To test something else: gofer pipeline run run-tests -v REF=<branch, tag or commit>",
    );

    pipeline.task(
        Task::new("checkout", "alpine:3.22")
            .description("Fetches the commit being tested into the run folder")
            .run("apk add --no-cache git")
            .variables([("REF", "main")])
            .script(CHECKOUT),
    );

    // Rust and Go run at the same time from the same checkout.
    pipeline.task(
        Task::new("test-rust", "rust:1")
            .description("Checks formatting and lints, then runs every test in the Rust workspace")
            .depends_on("checkout", RequiredParentStatus::Success)
            // The toolchain the repo pins goes into the image, so runs don't download it each time.
            .copy(
                "../../rust-toolchain.toml",
                "/toolchain/rust-toolchain.toml",
            )
            .run("cd /toolchain && rustup toolchain install && rustup component add clippy rustfmt")
            // Incremental builds only help when a build is reused, and every run starts from nothing.
            .variables([("CARGO_INCREMENTAL", "0"), ("CARGO_TERM_COLOR", "always")])
            .timeout(Duration::from_secs(60 * 60))
            .script(RUST_CHECKS),
    );

    pipeline.task(
        Task::new("test-go", "golang:1")
            .description("Checks formatting, then vets and tests the Go SDK and vets the examples")
            .depends_on("checkout", RequiredParentStatus::Success)
            .timeout(Duration::from_secs(15 * 60))
            .script(GO_CHECKS),
    );

    pipeline.finish().unwrap();
}
