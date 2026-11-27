use gofer_sdk::config::{Pipeline, RequiredParentStatus, Task};

fn main() {
    let mut pipeline = Pipeline::new("images", "Images Example Pipeline")
        .description(
            "This pipeline shows the ways Gofer can build a task's image. `gofer up` builds them on your machine, \
uploads them to Gofer, and ties them to the version it registers, so a version always runs exactly what it was \
registered with.

Each task starts from a base image. It can copy files in from next to this config and run commands, which happen in \
the order they're written, and it can keep only some of what it built on a smaller base. Anything those can't do can \
be a Dockerfile instead.

Gofer only builds a task again when its files or how it's built changed, and it keeps the Rust and Go compilers' \
caches between builds on its own.",
        );

    // Compile a folder, then keep only the binary on a small base. The compiler isn't in the final image.
    pipeline.task(
        Task::new("report", "rust:1-trixie")
            .description("Compiles a small Rust program and runs it")
            .copy("./report", "/src")
            .run("cd /src && cargo build --release")
            .keep(["/src/target/release/report"], "debian:13-slim")
            .command(vec!["/src/target/release/report".into()]),
    );

    // Copy a script into an image that can run it.
    pipeline.task(
        Task::new("cleanup", "python:3-slim")
            .description("Runs a Python script that's copied in")
            .copy("./cleanup.py", "/cleanup.py")
            .command(vec!["python".into(), "/cleanup.py".into()]),
    );

    // A script written right here. It still gets an image of its own.
    pipeline.task(
        Task::new("notify", "alpine:3.22")
            .description("Runs once the others have finished, whether they worked or not")
            .depends_on("report", RequiredParentStatus::Any)
            .depends_on("cleanup", RequiredParentStatus::Any)
            .depends_on("scan", RequiredParentStatus::Any)
            .script("echo notify: everything else finished"),
    );

    // Anything fancier is a Dockerfile. The last argument is the folder it builds from.
    pipeline.task(
        Task::dockerfile("scan", "./scan/Dockerfile", "./scan")
            .description("Builds from a Dockerfile, the same as `docker build ./scan` would"),
    );

    pipeline.finish().unwrap();
}
