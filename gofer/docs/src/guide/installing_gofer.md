# Installing Gofer

Gofer comes as an easy to distribute pre-compiled binary that you can run on your machine locally, but you can always
build Gofer from [source](#from-source) if need be.

## Pre-compiled (Recommended)

You can download the latest version for linux here:

```bash
wget -O gofer https://github.com/clintjedwards/gofer/releases/latest/download/gofer_amd64_linux_musl
chmod +x gofer
```

## From Source

You'll need Rust and musl-gcc, from the `musl` package on Arch or `musl-tools` on Debian and Ubuntu. Gofer builds as a
static binary, so it runs on any Linux machine and can put itself in every task's container as the `gofer` CLI. To
build the documentation site that Gofer serves at `/docs`, you'll also need mdbook and mdbook-linkcheck
(`cargo install mdbook mdbook-linkcheck`).

```bash
git clone https://github.com/clintjedwards/gofer && cd gofer
make build
ls ./target/x86_64-unknown-linux-musl/release/gofer
```
