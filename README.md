<!-- ANCHOR: before_demo -->
# [Gofer](https://gofer.clintjedwards.com/docs/assets/urban_dictionary_gofer.png): Run short-lived jobs easily.

## Summary

<p align="center">
    <img src="https://gofer.clintjedwards.com/docs/assets/logo-name-hq.png" alt="gofer" width="200"/>
</p>

[![website-badge](https://img.shields.io/badge/docs-learn%20more-3498db?style=flat-square)](https://gofer.clintjedwards.com/docs)
[![project status](https://img.shields.io/badge/Project%20Status-Alpha-orange?style=flat-square)](https://github.com/clintjedwards/gofer/releases)

Gofer is an opinionated automation engine designed for the cloud-native era. It's basically remote code execution as
a platform.

Gofer allows you to write pipelines in real code and keep those pipelines in sync with your business logic without
having to merge the two.

It specializes in executing your custom scripts in a containerized environment, making it versatile for both developers
and operations teams. Deploy Gofer effortlessly as a single static binary, and manage it using expressive, declarative
configurations written in familiar programming languages.

Its primary function is to execute short-term jobs like code linting, build automation, testing, port scanning, ETL
operations, or any task you can containerize and trigger based on events.

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

- **Simple Deployment**: Install Gofer effortlessly with a single static binary and manage it through its intuitive
  command-line interface.
- **Language Flexibility**: Craft your pipelines in programming languages you're already comfortable with, such as Go or
  Rust, instead of wrestling with unfamiliar YAML.
- **Local Testing**: Validate and run your pipelines locally, eliminating the guesswork of "commit and see" testing.
- **Extensible Architecture**: Easily extend Gofer's capabilities by writing your own plugins, backends, and more, in
  any language via OpenAPI.
- **Built-In Storage**: Comes with an integrated Object and Secret store for your convenience.
- **DAG Support**: Harness the power of Directed Acyclic Graphs (DAGs) for complex workflow automation.
- **Versioned Pipelines**: Every update to a pipeline is kept as a new version, so you can see exactly what each run ran
  with and roll back to an earlier one.
- **Build Images Easily**: Instead of having to manage Dockerfiles and image building Gofer hands you an API to wrap
  your code in and build the image to run it automagically.

## Demo:

<!-- ANCHOR_END: before_demo -->
https://github.com/user-attachments/assets/98d96053-9b62-44ab-8f89-1b504bd86508
<!-- ANCHOR: after_demo -->

## Documentation & Getting Started

If you want to fully dive into Gofer, check out the [documentation site][website-url]!

## Install

Extended installation information is available through the
[documentation site](https://gofer.clintjedwards.com/docs/guide/installing_gofer.html).

### Download a specific release:

You can [view and download releases by version here][releases-url].

### Download the latest release:

- **Linux:** `wget -O gofer https://github.com/clintjedwards/gofer/releases/latest/download/gofer_amd64_linux_gnu`

### Build from source:

1. `git clone https://github.com/clintjedwards/gofer && cd gofer`
2. `make build`
3. `ls ./target/release/gofer`

You'll need Rust, plus mdbook and mdbook-linkcheck (`cargo install mdbook mdbook-linkcheck`) to build the
documentation site that Gofer serves at `/docs`.

The Gofer binary comes with a CLI to manage the server as well as act as a client.

## Development

Working on Gofer itself? [DEVELOPMENT.md](https://github.com/clintjedwards/gofer/blob/main/DEVELOPMENT.md) covers
running it locally, regenerating the OpenAPI spec and SDKs, editing the docs and how releases are cut.

## Authors

- **Clint Edwards** - [Github](https://github.com/clintjedwards)

This software is provided as-is. It's a hobby project, done in my free time, and I don't get paid for doing it.

[website-url]: https://gofer.clintjedwards.com/docs
[concourse-url]: https://concourse-ci.org/
[canarying-url]: https://sre.google/workbook/canarying-releases/
[releases-url]: https://github.com/clintjedwards/gofer/releases
<!-- ANCHOR_END: after_demo -->
