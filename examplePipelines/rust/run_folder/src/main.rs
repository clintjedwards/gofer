use gofer_sdk::config::{Pipeline, RequiredParentStatus, Task};

/// Points Go's caches into the run folder, so the tasks share what the first one downloads and builds. GOPROXY=off
/// on the tasks after it proves they don't download anything again.
fn go_caches(proxy: &str) -> [(&str, &str); 3] {
    [
        ("GOMODCACHE", "/gofer/run/cache/mod"),
        ("GOCACHE", "/gofer/run/cache/build"),
        ("GOPROXY", proxy),
    ]
}

fn main() {
    Pipeline::new("run-folder", "Run Folder Example Pipeline")
        .description(
            "This pipeline shows what tasks can do with the folder they share at /gofer/run. Each run gets an \
empty one, every task in the run can read and write it, and Gofer deletes it when the run ends.

It clones Gofer's own repository once, downloads its Go dependencies once, then tests the Go SDK and builds a small \
program at the same time from that one checkout and cache. A task running as a different user packages what was \
built, and the last task writes a summary and keeps it as a pipeline object, since the folder is gone afterward:

    gofer pipeline object get summary --pipeline run-folder --stringify

To build another branch or tag: gofer pipeline run run-folder -v REF=<branch or tag>",
        )
        .tasks(vec![
            // Clone once, and only the folders the run uses. Every task after this reads the same files.
            Task::new("checkout", "alpine:3.22")
                .description("Clones the repository into the run folder and works out the version")
                .run("apk add --no-cache git")
                .variables([
                    ("REPO", "https://github.com/clintjedwards/gofer.git"),
                    ("REF", "main"),
                ])
                .script(
                    r#"
                    set -eu
                    git clone --quiet --depth 1 --filter=blob:none --sparse --branch "$REF" "$REPO" /gofer/run/src
                    git -C /gofer/run/src sparse-checkout set sdk/go examplePipelines/go/simple
                    git -C /gofer/run/src rev-parse --short HEAD > /gofer/run/version
                    echo "checked out $REF at $(cat /gofer/run/version)"
                    "#,
                ),
            Task::new("download", "golang:1")
                .description("Downloads the Go modules once, for every task after it")
                .depends_on("checkout", RequiredParentStatus::Success)
                .variables(go_caches("https://proxy.golang.org,direct"))
                .script(
                    "
                    set -eu
                    cd /gofer/run/src/sdk/go && go mod download
                    cd /gofer/run/src/examplePipelines/go/simple && go mod download
                    ",
                ),
            // Test and build run at the same time. Each writes to a folder of its own so they don't overwrite each
            // other, and Go's caches are safe to share between them.
            Task::new("test", "golang:1")
                .description("Tests the Go SDK and saves the results for the summary")
                .depends_on("download", RequiredParentStatus::Success)
                .variables(go_caches("off"))
                .script(
                    "
                    set -eu
                    mkdir -p /gofer/run/reports
                    cd /gofer/run/src/sdk/go
                    go test -json ./... > /gofer/run/reports/tests.json
                    ",
                ),
            Task::new("build", "golang:1")
                .description("Builds the simple example pipeline for two platforms")
                .depends_on("download", RequiredParentStatus::Success)
                .variables(go_caches("off"))
                .script(
                    "
                    set -eu
                    cd /gofer/run/src/examplePipelines/go/simple
                    for arch in amd64 arm64; do
                        CGO_ENABLED=0 GOOS=linux GOARCH=$arch go build -o /gofer/run/dist/linux-$arch/simple .
                    done
                    ",
                ),
            // The Dockerfile makes this run as nobody, while the tasks before it ran as root. It can read what they
            // made and write new files to the run folder, but it couldn't change their files.
            Task::dockerfile("package", "./package/Dockerfile", "./package")
                .description("Packages each build with a checksum, as a user other than root")
                .depends_on("build", RequiredParentStatus::Success),
            // Runs even when tests fail or nothing got built, since that's when a summary is most useful.
            Task::new("summary", "python:3-slim")
                .description("Writes a summary of the run and keeps it as a pipeline object")
                .depends_on("test", RequiredParentStatus::Any)
                .depends_on("package", RequiredParentStatus::Any)
                .copy("./summary.py", "/summary.py")
                .inject_api_token(true)
                .command(vec!["python".into(), "/summary.py".into()]),
        ])
        .finish()
        .unwrap();
}
