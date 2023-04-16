# Feature Guide

## Write your pipelines in a real programming language.

Other infrastructure tooling tried configuration languages(yaml, hcl).... and they kinda suck[^1]. The Gofer CLI allows you to create your pipelines in a fully featured programming language. Pipelines can be currently be written in Go or Rust[^2].

## DAG(Directed Acyclic Graph) Support.

Gofer provides the ability to run your containers in reference to your other containers.

With DAG support you can run containers:

- In parallel.
- After other containers.
- When particular containers fail.
- When particular containers succeed.

## GRPC API

Gofer uses [GRPC](https://grpc.io/) and [Protobuf](https://developers.google.com/protocol-buffers) to construct its API surface. This means that Gofer's API is easy to use, well defined, and can easily be developed for in any language.

The use of Protobuf gives us two main advantages:

1. The most up-to-date API contract can always be found by reading [the .proto files](https://github.com/clintjedwards/gofer/blob/main/proto/gofer.proto) included in the source.
2. Developing against the API for developers working within Golang/Rust simply means importing the [autogenerate proto package](https://pkg.go.dev/github.com/clintjedwards/gofer/proto).
3. Developing against the API for developers not working within the Go/Rust language means simply [importing the proto](https://github.com/clintjedwards/gofer/blob/main/proto/gofer.proto) files and generating them for the language you need.

You can find more information on protobuf, proto files, and how to autogenerate the code you need to use them to develop against Gofer in the [protobuf documentation.](https://developers.google.com/protocol-buffers/docs/overview)

## Namespaces

Gofer allows you to separate out your pipelines into different namespaces, allowing you to organize your teams and set permissions based on those namespaces.

## Extensions

Extensions are the way users can add extra functionality to their pipelines. For instance the ability to automate their pipelines by waiting on bespoke events (like the passage of time).

Extensions are nothing more than docker containers themselves that talk to the main process when they require activity.

Gofer out of the box provides some default extensions like [cron](./ref/extensions/provided/cron.md) and [interval](./ref/extensions/provided/interval.md). But even more powerful than that, it accepts any type of extension you can think up and code using the included [SDK](https://pkg.go.dev/github.com/clintjedwards/gofer/sdk).

Extensions are brought up alongside Gofer as long-running docker containers that it launches and manages.

## Object Store

Gofer provides a built in [object store](./ref/object_store/index.html) you can access with the Gofer CLI. This object store provides a caching and data transfer mechanism so you can pass values from one container to the next, but also store objects that you might need for all containers.

## Secret Store

Gofer provides a built in [secret store](./ref/secret_store/index.html) you can access with the Gofer CLI. This secret store provides a way to pass secret values needed by your pipeline configuration into Gofer.

## Events

Gofer provides a list of events for the most common actions performed. You can view this event stream via the Gofer API, allowing you to build on top of Gofer's actions and even using Gofer as a extension.

## External Events

Gofer allows extensions to consume external events. This allows for extensions to respond to webhooks from favorite sites like Github and more.

## Pluggable Everything

Gofer plugs into all your favorite backends your team is already using. This means that you never have to maintain things outside of your wheelhouse.

Whether you want to schedule your containers on [K8s](https://kubernetes.io/) or [AWS Lambda](https://aws.amazon.com/lambda/), or maybe you'd like to use an object store that you're more familiar with in [minio](https://min.io/) or [AWS S3](https://aws.amazon.com/s3/), Gofer provides either an already created plugin or an interface to write your own.

[^1]: Initally why configuration languages are used made sense, namely lowering the bar for users who might not know how to program and making it simplier overall to maintain(read: not shoot yourself in the foot with crazy inheritance structures). But, in practice, we've found that they kinda suck. Nobody wants to learn yet another language for this one specific thing. Furthermore, using a separate configuration language doesn't allow you to plug into years of practice/tooling/testing teams have with a certain favorite language.

<!-- prettier-ignore -->
[^2]: All pipelines eventualy reduce to protobuf so technically given the correct libraries your pipelines can be written in any language you like!

<!-- prettier-ignore -->
[^3]: Via GRPC.
