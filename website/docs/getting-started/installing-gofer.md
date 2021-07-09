---
id: installing-gofer
title: Installing Gofer
sidebar_position: 1
---

# Installing Gofer

Gofer comes as an easy to distribute pre-compiled binary that you can run on your machine locally, but you can always build Gofer from [source](#from-source) if need be.

## Pre-compiled (Recommended)

You can download the latest version for <code>linux and macos</code> here:

```shell
wget https://github.com/clintjedwards/gofer/releases/latest/download/gofer
```

## From Source

Gofer contains protobuf assets which will not get compiled if used via `go install`.
Alternatively we can use `make` to build ourselves an impromptu version.

```shell
git clone https://github.com/clintjedwards/gofer && cd gofer
make build path=/tmp/gofer
/tmp/gofer --version
```
