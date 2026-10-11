# Installing Gofer

Gofer comes as an easy to distribute pre-compiled binary that you can run on your machine locally, but you can always build Gofer from [source](#from-source) if need be.

## Pre-compiled (Recommended)

You can download the latest version for linux here:

```bash
wget -O gofer https://github.com/clintjedwards/gofer/releases/latest/download/gofer_amd64_linux_gnu
chmod +x gofer
```

## From Source

You'll need Rust, plus mdbook and mdbook-linkcheck (`cargo install mdbook mdbook-linkcheck`) to build the
documentation site that Gofer serves at `/docs`.

```bash
git clone https://github.com/clintjedwards/gofer && cd gofer
make build
ls ./target/release/gofer
```
