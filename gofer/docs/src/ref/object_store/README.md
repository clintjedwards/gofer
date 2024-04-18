# Object Store

Gofer provides an object store as a way to share values and objects between containers. It can also be used as a cache.
It is common for one container to run, generate an artifact or values, and then store that object in the object store
for the next container or next run. The object store can be accessed through the [Gofer CLI](../../cli/index.html) or
through the normal Gofer API.

Gofer divides the objects stored into two different lifetime groups:

## Pipeline-level objects

Gofer can store objects permanently for each pipeline. You can store objects at the pipeline-level by using the
gofer pipeline object store command:

```bash
gofer pipeline object put my-pipeline my_key1=my_value5
gofer pipeline object get my-pipeline my_key1
```

The limitation to pipeline level objects is that they have a limit of the number of objects that can be stored
per-pipeline. Once that limit is reached the oldest object in the store will be removed for the newest object.

## Run-level objects

Gofer can also store objects on a per-run basis. Unlike the pipeline-level objects run-level do not have a limit to how
many can be stored, but instead have a limit of how long they last. Typically after a certain number of runs a object
stored at the run level will expire and that object will be deleted.

You can access the run-level store using the run level store CLI commands. Here is an example:

```bash
gofer run object put simple_pipeline my_key=my_value
gofer run object get simple_pipeline my_key
```

## Supported Object Stores

The only currently supported object store is the sqlite object store. Reference the [configuration reference](../server_configuration/configuration_reference.md) for a full list of configuration settings and options.

## How to add new Object Stores?

Object stores are pluggable, but for them to maintain good performance and simplicity the code that orchestrates them must
be added to the object_store folder within Gofer(which means they have to be written in Rust).
