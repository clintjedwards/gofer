<!-- ANCHOR: before_demo -->
# [Gofer](https://gofer.clintjedwards.com/docs/assets/urban_dictionary_gofer.png): Run short-lived jobs easily.

## Summary

<p align="center">
    <img src="https://gofer.clintjedwards.com/docs/assets/logo-name-hq.png" alt="gofer" width="200"/>
</p>

[![website-badge](https://img.shields.io/badge/docs-learn%20more-3498db?style=flat-square)](https://gofer.clintjedwards.com/docs)
[![project status](https://img.shields.io/badge/Project%20Status-Alpha-orange?style=flat-square)](https://github.com/clintjedwards/gofer/releases)

Gofer is an opinionated, streamlined automation engine designed for the cloud-native era. It's basically remote code execution as a platform.

Gofer focuses on the "what" and "when" of your workloads, leaving the "how" and "where" to pluggable, more sophisticated container orchestrators (such as K8s or Nomad or even local Docker).

It specializes in executing your custom scripts in a containerized environment, making it versatile for both developers and operations teams. Deploy Gofer effortlessly as a single static binary, and manage it using expressive, declarative configurations written in real programming languages.

Its primary function is to execute short-term jobs like code linting, build automation, testing, port scanning, ETL operations, or any task you can containerize and trigger based on events.

## Low Priority

Previously I had discontinued Gofer, but I've been using it personally so much that I figured it would be good to keep
it un-archived to push bug fixes and with LLMs being so good now maybe start rolling new features/refactors. But it's
pretty low priority for me.

## Why?:

- This is my idea of fun.
- Modern solutions...
  - are too complicated to setup and/or manage.
  - lack tight feedback loops while developing pipelines.
  - require you to marry your business logic code to pipeline logic code.
  - use configuration languages (or sometimes worse...their own DSL) as the interface to express what you want.
  - lack extensibility
- It is an experiment to see if theses are all solveable problems in the effort to create a simpler, faster solution.

## Features:

- **Simple Deployment**: Install Gofer effortlessly with a single static binary and manage it through its intuitive command-line interface.
- **Language Flexibility**: Craft your pipelines in programming languages you're already comfortable with, such as Go or Rust—no more wrestling with unfamiliar YAML.
- **Local Testing**: Validate and run your pipelines locally, eliminating the guesswork of "commit and see" testing.
- **Extensible Architecture**: Easily extend Gofer's capabilities by writing your own plugins, backends, and more, in any language via OpenAPI.
- **Built-In Storage**: Comes with an integrated Object and Secret store for your convenience.
- **DAG Support**: Harness the power of Directed Acyclic Graphs (DAGs) for complex workflow automation.
- **Robust Reliability**: Automatic versioning, Blue/Green deployments, and canary releases ensure the stability and dependability of your pipelines.

## Demo:

<!-- ANCHOR_END: before_demo -->
https://github.com/user-attachments/assets/ff9a39ca-c85a-4cfc-8c99-da510c01e3c2
<!-- ANCHOR: after_demo -->

## Documentation & Getting Started

If you want to fully dive into Gofer, check out the [documentation site][website-url]!

## Install

Extended installation information is available through the [documentation site](https://gofer.clintjedwards.com/docs/guide/installing_gofer.html).

### Download a specific release:

You can [view and download releases by version here][releases-url].

### Download the latest release:

- **Linux:** `wget -O gofer https://github.com/clintjedwards/gofer/releases/latest/download/gofer_amd64_linux_gnu`

### Build from source:

1. `git clone https://github.com/clintjedwards/gofer && cd gofer`
2. `make build`
3. `ls ./target/release/gofer`

The Gofer binary comes with a CLI to manage the server as well as act as a client.

## Dev Setup

Gofer is setup such that the base run mode is the development mode. So simply running the binary
without any additional flags allows easy auth-less development. You can read more about how to deploy Gofer in a
production environment [here](https://gofer.clintjedwards.com/docs/ref/server_configuration/index.html)

This is really helpful for users and developers alike since it allows easy access to a runnable server to test pipelines
against.

### You'll need to install the following first:

To run Gofer dev mode:

- [Docker](https://www.docker.com/)

### Run from the Makefile

Gofer uses flags, env vars, and files to manage configuration (in order of most important). The Makefile already includes all the commands and flags you need to run in dev mode by simply running `make run`.

In case you want to run without the make file simply run:

```bash
cd gofer
export GOFER_WEB_API__LOG_LEVEL=debug
cargo run --bin gofer -- service start
```

### Env aware configuration

To avoid issues when developing Gofer, the development build of Gofer(`any binary that was not built with --release`)
looks for the CLI config file at `.gofer_dev.toml` instead of `.gofer.toml`.

This avoids the headache of having to swap configuration files while actively developing Gofer. But is noted here since
it can be confusing if not known.

### Editing OpenAPI spec files

#### Where are the openapi spec files?

Gofer uses OpenAPI to generate a REST API in which is uses both to communicate with extensions and the main web service.

- You can find the OpenAPI spec files located in `sdk/openapi.json` and `gofer/docs/src/assets/openapi.json`.
- The documentation site has an API reference section generated from the spec, available at `/docs/api/index.html`
  in the main web service. `make generate-api-docs` (included in `make generate-openapi`) regenerates those pages
  into `gofer/docs/src/api/` via the `generate_api_docs` binary.

#### How do we generate new spec files?

Gofer uses [oapi-codegen](https://github.com/deepmap/oapi-codegen) to generate the Golang sdk and [progenitor](https://github.com/oxidecomputer/progenitor) to generate the Rust SDK.

You can download oapi-codegen by performing `go install github.com/deepmap/oapi-codegen/v2/cmd/oapi-codegen@latest`.
Progenitor is already included as a lib within the generation code.

The OpenAPI Spec files are generated by the web framework used [dropshot](https://github.com/oxidecomputer/). It
generates the files directly from the API code using Rust proc macros over the direct API function handlers.
This creates a sort of chicken and egg problem when attempting to change things and the compile times from using many
proc macros are long. This will soon be resolved [by using a more trait based approach.](https://rfd.shared.oxide.computer/rfd/0479)

You can run the generate script by using `make generate-openapi` from the root directory.

### Editing Documentation

Documentation is done with [mdbook](https://rust-lang.github.io/mdBook/).

To install:

```bash
cargo install mdbook
cargo install mdbook-linkcheck
```

Once you have mdbook you can simply run `make run-docs` to give you an auto-reloading dev version of the documentation
in a browser.

### Regenerating the Demo Video

The demo video is recorded with [vhs](https://github.com/charmbracelet/vhs); a very handy tool that allows you to write a
configuration file which will pop out a video on the other side.

In order to do this VHS has to run the commands so we must start the server first before we regenerate the video.

```bash
rm -rf /tmp/gofer* # Start with a fresh database
make run # Start the server in dev mode
cd gofer/docs/src/assets
vhs demo.tape # this will start running commands against the server and output the video as demo.mp4.
```

The video shows up in two places, and each one gets it differently:

- **The documentation site** plays `demo.mp4` straight from the repo. `introduction.md` includes this README in two
  pieces using the `before_demo` and `after_demo` anchor comments around the Demo section, and puts a `<video>` tag in
  between. Committing the new `demo.mp4` is all it needs.
- **GitHub** strips `<video>` tags out of READMEs and won't play videos hosted anywhere else, so the README needs its
  own copy uploaded to GitHub by hand:
  1. Upload it by attaching it to an issue. This needs a `gh` new enough to have `--attach`:

     ```bash
     gh issue create --title "Demo video upload" --body "Hosts the demo video for the README." \
       --attach gofer/docs/src/assets/demo.mp4
     ```

     The issue body ends up with a `https://github.com/user-attachments/assets/...` URL; `gh issue view <number>`
     shows it. Close the issue afterwards with `gh issue close <number>`; the upload stays around. Uploads are
     limited to 10MB on a free plan.

  2. Put that URL on its own line in the README's Demo section, replacing whatever is between the
     `ANCHOR_END: before_demo` and `ANCHOR: after_demo` comments. GitHub turns a bare user-attachments URL into a
     video player.

  The docs site skips everything between those two comments, since mdbook would show the URL as a plain link.

## Authors

- **Clint Edwards** - [Github](https://github.com/clintjedwards)

This software is provided as-is. It's a hobby project, done in my free time, and I don't get paid for doing it.

If you're looking for the previous Golang version you can [find it here.](https://github.com/clintjedwards/gofer/tree/e83adcd5c5164bba791f06e38702d81621b5624b)

[website-url]: https://clintjedwards.github.io/gofer
[concourse-url]: https://concourse-ci.org/
[canarying-url]: https://sre.google/workbook/canarying-releases/
[releases-url]: https://github.com/clintjedwards/gofer/releases
<!-- ANCHOR_END: after_demo -->
