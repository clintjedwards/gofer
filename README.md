# [Gofer](https://clintjedwards.com/gofer/assets/urban_dictionary_gofer.png): Run short-lived jobs easily.

<p align="center">
    <img src="https://clintjedwards.com/gofer/assets/logo-name-hq.png" alt="gofer" width="200"/>
</p>

[![godoc for clintjedwards/gofer][godoc-badge]][godoc-url]
[![docs site][website-badge]][website-url]
[![project status][project-status]][project-status]

Gofer is an opinionated, cloud-native, container-focused, continuous thing do-er. It focuses on simplicity and usability for both developers and ops.

You deploy it as a single static binary service, pass it declarative configurations written in real programming languages, and watch as it automatically handles periodic scheduling of your automation workloads.

Gofer runs your workloads on whatever your choice of container scheduler: Nomad, K8s, Local Docker.

It's purpose is to run short term jobs such as: code linters, build tools, tests, port-scanners, ETL tooling and anything else you can package into a Docker container and run as a result of some other event happening.

## Features:

- Deploy Gofer as a single static binary, manage Gofer through the included command line interface.
- Write your pipelines in a programming language you're familiar with; stop cobbling together unfamiliar yaml. (**Go** or **Rust** for now).
- Test and run your pipelines locally; No more <i>"commit it and see"</i> testing.
- Pluggable: Write your own extensions, backends, and more in any language (through GRPC).
- Included Object and Secret store.
- DAG(Directed Acyclic Graph) support.
- Reliability tooling: Automatically version, Blue/Green deploy, and [canary][canarying-url] deploy updated versions of your pipelines.

## Demo:

<img src="https://clintjedwards.com/gofer/assets/demo.gif" />

## Documentation & Getting Started

If you want to fully dive into Gofer, check out the [documentation site][website-url]!

## Install

Extended installation information is available through the [documentation site](https://clintjedwards.com/gofer/guide/installing_gofer.html).

### Download a specific release:

You can [view and download releases by version here][releases-url].

### Download the latest release:

- **Linux:** `wget https://github.com/clintjedwards/gofer/releases/latest/download/gofer`

### Build from source:

You'll need to install [protoc and its associated golang/grpc modules first](https://grpc.io/docs/languages/go/quickstart/)

1. `git clone https://github.com/clintjedwards/gofer && cd gofer`
2. `make build OUTPUT=/tmp/gofer`

The Gofer binary comes with a CLI to manage the server as well as act as a client.

## Dev Setup

Gofer is setup such that the base run mode is the development mode. So simply running the binary
without any additional flags allows easy authless development.

### You'll need to install the following first:

To run Gofer dev mode:

- [Docker](https://www.docker.com/)

To build protocol buffers:

- [protoc](https://grpc.io/docs/protoc-installation/)
- [protoc gen plugins go/grpc](https://grpc.io/docs/languages/go/quickstart/)

### Run from the Makefile

Gofer uses flags, env vars, and files to manage configuration (in order of most important). The Makefile already includes all the commands and flags you need to run in dev mode by simply running `make run`.

In case you want to run without the make file simply run:

```bash
export GOFER_LOG_LEVEL=debug
go build -o /tmp/$gofer
/tmp/gofer service start --dev-mode
```

### Editing Protobufs

Gofer uses grpc and protobufs to communicate with both plugins and provide an external API. These protobuf
files are located in `/proto`. To compile new protobufs once the original `.proto` files have changed you can use the `make build-protos` command.

### Editing Documentation

Documentation is done with [mdbook](https://rust-lang.github.io/mdBook/).

To install:

```bash
cargo install mdbook
cargo install mdbook-linkcheck
```

Once you have mdbook you can simply run `make run-docs` to give you an auto-reloading dev version of the documentation in a browser.

### Regenerating Demo Gif

The Gif on the README page uses [vhs](https://github.com/charmbracelet/vhs); a very handy tool that allows you to write a configuration file which will pop out
a gif on the other side.

In order to do this VHS has to run the commands so we must start the server first before we regenerate the gif.

```bash
rm -rf /tmp/gofer* # Start with a fresh database
make run # Start the server in dev mode
cd documentation/src/assets
vhs < demo.tape # this will start running commands against the server and output the gif as demo.gif.
```

## Authors

- **Clint Edwards** - [Github](https://github.com/clintjedwards)

This software is provided as-is. It's a hobby project, done in my free time, and I don't get paid for doing it.

[godoc-badge]: https://pkg.go.dev/badge/github.com/clintjedwards/gofer
[godoc-url]: https://pkg.go.dev/github.com/clintjedwards/gofer
[goreport-badge]: https://goreportcard.com/badge/github.com/clintjedwards/gofer
[website-badge]: https://img.shields.io/badge/docs-learn%20more-3498db?style=flat-square
[website-url]: https://clintjedwards.github.io/gofer
[concourse-url]: https://concourse-ci.org/
[canarying-url]: https://sre.google/workbook/canarying-releases/
[releases-url]: https://github.com/clintjedwards/gofer/releases
[project-status]: https://img.shields.io/badge/Project%20Status-Alpha-orange?style=flat-square
