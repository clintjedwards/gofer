---
id: overview
title: Overview
sidebar_position: 1
---

# Object Stores

Gofer provides an object store as a way to share values and objects between containers. It can also be used as a cache. It is common for one container to run, generate an artifact or values, and then store that object in to the object store for the next container or next run. The object store can be accessed through the [Gofer CLI](../cli/gofer_pipeline_store) or through the normal Gofer API.

Gofer divides the objects stored into two different lifetime groups:

## Pipeline-level objects

Gofer can store objects permanently for each pipeline. You can store objects at the pipeline-level by using the [gofer pipeline store](../cli/gofer_pipeline_store) command:

```bash
gofer pipeline store put my-pipeline my_key1=my_value5
gofer pipeline store get my-pipeline my_key1
#output: my_value5
```

The limitation to pipeline level objects is that they have a limit of the number of objects that can be stored per-pipeline. Once that limit is reached the oldest object in the store will be removed for the newest object.

## Run-level objects

Gofer can also store objects on a per-run basis. Unlike the pipeline-level objects run-level do not have a limit to how many can be stored, but instead have a limit of how long they last. Typically after a certain number of runs a object stored at the run level will expire and that object will be deleted.

You can access the run-level store using the [run level store CLI commands](../cli/gofer_run_store). Here is an example:

```bash
gofer run store put simple_pipeline my_key=my_value
gofer run store get simple_pipeline my_key
#output: my_value
```

## Supported Object Stores

The only currently supported object store is the [boltdb object store](bolt/overview).

## How to add new Object Stores?

Object stores are pluggable! Simply implement a new object store by following [the given interface.](https://github.com/clintjedwards/gofer/blob/main/internal/objectStore/objectStore.go#L23)

```go
type Engine interface {
	GetObject(key string) ([]byte, error)
	PutObject(key string, content []byte, force bool) error
	DeleteObject(key string) error
}
```
