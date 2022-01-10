---
id: auth
title: Auth
sidebar_position: 3
---

# Auth

Gofer's auth system is meant to be extremely lightweight and a stand-in for a more complex auth system.

## How auth works

Gofer uses API Tokens for authorization. You pass a given token in whenever talking to the API and Gofer will evaluate internally what type of token you possess and for which namespaces does it posses access.

### Management Tokens

The first type of token is a management token. Management tokens essentially act as root tokens and have access to all routes.

It is important to be extremely careful about where your management tokens end up and how they are used.

Other than system administration, the main use of management tokens are the creation of new tokens. You can explore [token creation though the CLI.](../cli/gofer_service_token_create)

It is advised that you use a single management token as the root token by which you create all user tokens.

### Client Tokens

The most common token type is a client token. The client token simply controls which namespaces a user might have access to.

During token creation you can choose one or multiple namespaces for the token to have access to.

## How to auth via the API

The Gofer API uses GRPC's metadata functionality to read tokens from requests:

```go
md := metadata.Pairs("Authorization", "Bearer "+<token>)
```

## How to auth via the CLI

The Gofer CLI accepts [many ways setting a token once you have one.](../cli/configuration)
