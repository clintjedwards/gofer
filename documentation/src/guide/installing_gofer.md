# Installing Gofer

Gofer comes as an easy to distribute pre-compiled binary that you can run on your machine locally, but you can always build Gofer from [source](#from-source) if need be.

## Pre-compiled (Recommended)

You can download the latest version for linux here:

```bash
wget https://github.com/clintjedwards/gofer/releases/latest/download/gofer
```

## From Source

Gofer contains protobuf assets which will not get compiled if used via `go install`.
To solve for this we can use `make` to build ourselves an impromptu version.

```bash
git clone https://github.com/clintjedwards/gofer && cd gofer
make build OUTPUT=/tmp/gofer SEMVER=0.0.dev
/tmp/gofer --version
```
