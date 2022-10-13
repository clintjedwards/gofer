# [Gofer](./urban_dictionary_gofer.png): Run short-lived jobs easily.

<p align="center">
    <img src="website/static/img/logo-name-hq.png" alt="gofer" width="200"/>
</p>

[![godoc for clintjedwards/gofer][godoc-badge]][godoc-url]
[![docs site][website-badge]][website-url]
[![project status][project-status]][project-status]

Gofer is an opinionated, cloud-native, container-focused, continuous thing do-er, that focuses on simplicity and usability for both developers and ops.

## Features:

- Deploy it as a single static binary
- Write your pipelines in **Go** or **Rust**
- Pluggable: Write your own triggers, shared tasks, and more in any language (through GRPC).
- DAG(Directed Acyclic Graph) support.
- Reliability tooling: A/B test, version, and canary new pipelines.
- Bring your own everything! Secret store, object store, container scheduler. Gofer has the interfaces to support all of them.

## Philosophy:

It uses a philosophy similar to [concourse][concourse-url], leveraging the industry popular Docker container as the packaging method for code running on infrastructure. The benefits of this is _simplicity_. No foreign agents, no cluster setup, no yaml mess. Everything is based on running a container, mirroring what most companies already understand and use on a day to day basis.

[You can read more about Gofer and it's philosophy here.](https://clintjedwards.com/gofer/docs/intro)

## Demo:

<a href="https://asciinema.org/a/459946">
    <img src="demo.png" title="Click on image for demo" />
</a>

## Documentation & Getting Started

If you want to fully dive into Gofer, check out the [documentation site][website-url]!

## Install

Extended installation information is available through the [documentation site](https://clintjedwards.com/gofer/docs/getting-started/installing-gofer).

### Download a specific release:

You can [view and download releases by version here][releases-url].

### Download the latest release:

- **Linux:** `wget https://github.com/clintjedwards/gofer/releases/latest/download/gofer`

### Build from source:

You'll need to install [protoc and its associated golang/grpc modules first](https://grpc.io/docs/languages/go/quickstart/)

1. `git clone https://github.com/clintjedwards/gofer && cd gofer`
2. `make build OUTPUT=/tmp/gofer`

The Gofer binary comes with a CLI to manage the server as well as act as a client.

## Why not use <insert favorite tool\> instead ?

| Tool                                                                                                                            | Category                         | Why not?                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ------------------------------------------------------------------------------------------------------------------------------- | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [Jenkins](https://www.jenkins.io/)                                                                                              | General thing-doer               | Supports generally anything you might want to do ever, but because of this it can be operationally hard to manage, usually has massive security issues and isn't by default opinionated enough to provide users a good interface into how they should be managing their workloads.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| [Buildkite](https://buildkite.com/)/[CircleCI](https://circleci.com/)/[Github actions](https://github.com/features/actions)/etc | Gitops cloud builders            | Gitops focused cloud build tooling is great for most situations and probably what most companies should start out using. The issue is that running your workloads can be hard to test since these tools use custom agents to manage those jobs. This causes local testing to be difficult as the custom agents generally work very differently locally. Many times users will fight with yaml and make commits just to test that their job does what they need due to their being no way to determine that beforehand.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| [ArgoCD](https://argo-cd.readthedocs.io/en/stable/)                                                                             | Kubernetes focused CI/CD tooling | In the right direction with its focus on running containers on already established container orchstrators, but Argo is tied to gitops making it hard to test locally, and also closely tied to Kubernetes.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| [ConcourseCI](https://concourse-ci.org/)                                                                                        | Container focused thing do-er    | Concourse is great and where much of this inspiration for this project comes from. It sports a sleek CLI, great UI, and cloud-native primatives that makes sense. The drawback of concourse is that it uses a custom way of managing docker containers that can be hard to reason about. This makes testing locally difficult and running in production means that your short-lived containers exist on a platform that the rest of your company is not used to running containers on.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| [Airflow](https://airflow.apache.org/)                                                                                          | ETL systems                      | I haven't worked with large scale data systems enough to know deeply about how ETL systems came to be, but (maybe naively) they seem to fit into the same paradigm of "run _x_ thing every time _y_ happens". Airflow was particularly rough to operate in the early days of its release with security and UX around DAG runs/management being nearly non-existent. As an added bonus the scheduler regularly crashed from poorly written user workloads making it a reliability nightmare. <br /><br /> Additionally, Airflow's models of combining the execution logic of your DAGs with your code led to issues of testing and iterating locally. <br /><br /> Instead of having tooling specifically for data workloads, instead it might be easier for both data teams and ops teams to work in the model of distributed cron as Gofer does. Write your stream processing using dedicated tooling/libraries like [Benthos](https://www.benthos.dev/) (or in whatever language you're most familiar with), wrap it in a Docker container, and use Gofer to manage which containers should run when, where, and how often. This gives you easy testing, separation of responsibilities, and no python decorator spam around your logic. |
| [Cadence](https://cadenceworkflow.io/)                                                                                          | ETL systems                      | I like Uber's cadence, it does a great job at providing a platform that does distributed cron and has some really nifty features by choosing to interact with your workflows at the code level. The ability to bake in sleeps and polls just like you would regular code is awesome. But just like Airflow, I don't want to marry my scheduling platform with my business logic. I write the code as I would for a normal application context and I just need something to run that code. When we unmarry the business logic and the scheduling platform we are able to treat it just like we treat all our other code, which means code workflows(testing, for example) we were all already used to and the ability to foster code reuse for these same processes.                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |

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
go build -o /tmp/gofer
DEBUG=true; SEMVER=0.0.0; /tmp/gofer service start
```

### Editing Protobufs

Gofer uses grpc and protobufs to communicate with both plugins and provide an external API. These protobuf
files are located in `/proto`. To compile new protobufs once the original `.proto` files have changed you can use the `make build-protos` command.

### Editing Documentation

Documentation is done with [mdbook](https://rust-lang.github.io/mdBook/). Installation instructions should be on the main site.

Once you have mdbook you can simply run `make run-docs` to give you an auto-reloading dev version of the documentation in a browser.

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
[12factor-url]: https://12factor.net/
[project-status]: https://img.shields.io/badge/Project%20Status-Alpha-orange?style=flat-square
