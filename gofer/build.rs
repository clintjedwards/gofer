use std::{env, fmt::Write, path::PathBuf, process::Command};

fn get_build_commit() -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    String::from_utf8(output.stdout).unwrap()
}

/// Writes the list of Gofer's own extensions' images into the build, so Gofer can store them when it starts. They're
/// the `docker save` tars `make extension-images` leaves in `target/extension-images/`, one per extension and named
/// after it. A build without them still works; those extensions then fail to start and say why.
fn embed_extension_images() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let images = manifest_dir.join("../target/extension-images");

    // Cargo reruns a build script when a path it was told to watch changes, but a missing one counts as changed every
    // time, which would rebuild Gofer on every build.
    std::fs::create_dir_all(&images).expect("Could not create the extension images folder");
    println!("cargo:rerun-if-changed={}", images.display());

    // Watching anything at all stops Cargo from rerunning this when the rest of the package changes, which it otherwise
    // does, and which is what keeps BUILD_COMMIT current.
    println!("cargo:rerun-if-changed=.");

    let mut tars: Vec<PathBuf> = std::fs::read_dir(&images)
        .expect("Could not read the extension images folder")
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "tar"))
        .collect();
    tars.sort();

    let mut code =
        String::from("/// Gofer's own extensions' images, by name, as `docker save` tars.\n");
    code.push_str("pub const BUILTIN_EXTENSION_IMAGES: &[(&str, &[u8])] = &[\n");
    for tar in tars {
        let name = tar.file_stem().unwrap().to_string_lossy().to_string();
        let path = tar.canonicalize().unwrap();
        writeln!(
            code,
            "    ({name:?}, include_bytes!({:?})),",
            path.display().to_string()
        )
        .unwrap();
    }
    code.push_str("];\n");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("extension_images.rs");
    std::fs::write(out, code).expect("Could not write the extension images list");
}

fn main() {
    // Set binary specific compile time variables.
    println!("cargo:rustc-env=BUILD_SEMVER={}", env!("CARGO_PKG_VERSION"));
    println!("cargo:rustc-env=BUILD_COMMIT={}", get_build_commit());

    // The documentation site is embedded into the binary, so its folder has to exist even before the docs are built.
    std::fs::create_dir_all("docs/book/html")
        .expect("Could not create documentation output folder");

    embed_extension_images();
}
