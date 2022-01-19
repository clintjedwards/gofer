---
id: overview
title: Overview
sidebar_position: 1
---

# Secret Stores

Gofer provides a secret store as a way to enable users to pass secrets into pipeline configuration
files.

The secrets included in the pipeline file use a special syntax so that Gofer understands when it is given a secret value instead of a normal variable.

```hcl
...
env_vars = {
  "SOME_SECRET_VAR" = "secret{{my_key_here}}"
}
...
```

## Supported Secret Stores

The only currently supported secret store is the [boltdb secret store](bolt/overview).

## How to add new Secret Stores?

Secret stores are pluggable! Simply implement a new secret store by following [the given interface.](https://github.com/clintjedwards/gofer/blob/main/internal/secretStore/secretStore.go#L23)

```go
type Engine interface {
	GetSecret(key string) (string, error)
	PutSecret(key string, content string, force bool) error
	DeleteSecret(key string) error
}
```
