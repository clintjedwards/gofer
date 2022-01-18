---
sidebar_position: 2
---

# API

Gofer uses [GRPC](https://grpc.io/) and [Protobuf](https://developers.google.com/protocol-buffers) to construct its API surface. This means that Gofer's API is easy to use, well defined, and can easily be developed for in any language.

The use of Protobuf gives us two main advantages:

1. The most up-to-date API contract can always be found by reading [the .proto files](https://github.com/clintjedwards/gofer/blob/main/proto/gofer.proto) included in the source.
2. Developing against the API for developers working within Golang simply means importing the [autogenerate proto package](https://pkg.go.dev/github.com/clintjedwards/gofer/proto).
3. Developing against the API for developers not working within the Go language means simply [importing the proto](https://github.com/clintjedwards/gofer/blob/main/proto/gofer.proto) files and generating them for the language you need.

You can find more information on protobuf, proto files, and how to autogenerate the code you need to use them to develop against Gofer in the [protobuf documentation.](https://developers.google.com/protocol-buffers/docs/overview)

## Auth

You can authenticate to Gofer using GRPC's metadata pair:

```go
md := metadata.Pairs("Authorization", "Bearer "+<token>)
```

More details about auth [can be found here.](server-configuration/auth)
