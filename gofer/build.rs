use std::{env, process::Command};

fn get_build_commit() -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    String::from_utf8(output.stdout).unwrap()
}

fn main() {
    // Set binary specific compile time variables.
    println!("cargo:rustc-env=BUILD_SEMVER={}", env!("CARGO_PKG_VERSION"));
    println!("cargo:rustc-env=BUILD_COMMIT={}", get_build_commit());

    // The documentation site is embedded into the binary, so its folder has to exist even before the docs are built.
    std::fs::create_dir_all("docs/book/html").expect("Could not create documentation output folder");
}
