# Object Store

Gofer provides an object store as a way to share values and objects between containers. It can also be used as a cache.
It is common for one container to run, generate an artifact or values, and then store that object in the object store
for the next container or next run. The object store can be accessed through the [Gofer CLI](../../cli/index.html) or
through the normal Gofer API. Tasks can also receive objects directly as environment variables; see
[Using Secrets and Objects in Variables](../pipeline_configuration/tasks.md#using-secrets-and-objects-in-variables).
For examples of handing values and files from one task to the next, see
[passing data between tasks](../../guide/patterns/passing_data_between_tasks.md).

Gofer divides the objects stored into two different lifetime groups:

## Pipeline-level objects

Gofer can store objects permanently for each pipeline. You can store objects at the pipeline-level by using the
gofer pipeline object store command:

```bash
gofer pipeline object put my_key1 ./some_file --pipeline my-pipeline
echo "my_value" | gofer pipeline object put my_key2 @ --pipeline my-pipeline
gofer pipeline object get my_key1 --pipeline my-pipeline
```

The limitation to pipeline level objects is that they have a limit of the number of objects that can be stored
per-pipeline (`pipeline_object_limit`). Once that limit is reached the oldest object in the store will be removed for
the newest object. Overwriting an existing object with `--force` counts it as the newest.

## Run-level objects

Gofer can also store objects on a per-run basis. Unlike the pipeline-level objects run-level do not have a limit to how
many can be stored, but instead have a limit of how long they last. Only the most recent runs keep their objects
(`run_object_expiry`, 2 by default); once a pipeline moves past that, the oldest run's objects are deleted.

You can access the run-level store using the run level store CLI commands. Here is an example:

```bash
gofer run object put my_key ./some_file --pipeline my-pipeline --run 1
gofer run object get my_key --pipeline my-pipeline --run 1
```

Inside a task you can leave off `--pipeline` and `--run`. The CLI falls back to `GOFER_PIPELINE_ID` and
`GOFER_RUN_ID`, which Gofer sets on every task.

## Keys

Object keys can be up to 128 characters of letters, numbers, `_`, `.`, and `-`, so file names like `build.tar.gz`
work as keys. Keys only need to be unique within their pipeline or run.

## Storing directories

If you `put` a directory, the CLI uploads it as a gzipped tarball. `get --extract <dir>` downloads and unpacks it,
so you don't have to write your own tar commands:

```bash
gofer run object put dist ./dist           # In the task that builds it.
gofer run object get dist --extract ./dist # In a task that depends on it.
```

## Supported Object Stores

The only currently supported object store is the [filesystem object store](./filesystem.md). Reference the [configuration reference](../server_configuration/configuration_reference.md) for a full list of configuration settings and options.

## How to add new Object Stores?

Object stores are pluggable, but for them to maintain good performance and simplicity the code that orchestrates them must
be added to the object_store folder within Gofer(which means they have to be written in Rust).
