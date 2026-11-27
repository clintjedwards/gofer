# Object Store

Gofer provides an object store for values and files that need to outlast a run, like caches, or a report someone looks
at later. It can be accessed through the [Gofer CLI](../../cli/index.html) or through the normal Gofer API. Tasks can
also receive objects directly as environment variables; see
[Using Secrets and Objects in Variables](../pipeline_configuration/tasks.md#using-secrets-and-objects-in-variables).

To hand files from one task to the next within a run, use the folder every task in a run shares instead, at
`/gofer/run`. See [passing data between tasks](../../guide/patterns/passing_data_between_tasks.md).

## Pipeline objects

Gofer stores objects for each pipeline, and they last across runs. You can store them with the gofer pipeline object
commands:

```bash
gofer pipeline object put my_key1 ./some_file --pipeline my-pipeline
echo "my_value" | gofer pipeline object put my_key2 @ --pipeline my-pipeline
gofer pipeline object get my_key1 --pipeline my-pipeline
```

Inside a task you can leave off `--pipeline`. The CLI falls back to `GOFER_PIPELINE_ID`, which Gofer sets on every
task.

The limitation to pipeline objects is that there's a limit to how many can be stored per pipeline
(`pipeline_object_limit`). Once that limit is reached the oldest object in the store will be removed for the newest
object. Overwriting an existing object with `--force` counts it as the newest.

## Keys

Object keys can be up to 128 characters of letters, numbers, `_`, `.`, and `-`, so file names like `build.tar.gz`
work as keys. Keys only need to be unique within their pipeline.

## Storing directories

If you `put` a directory, the CLI uploads it as a gzipped tarball. `get --extract <dir>` downloads and unpacks it,
so you don't have to write your own tar commands:

```bash
gofer pipeline object put node_modules ./node_modules --force # In a task that builds the cache.
gofer pipeline object get node_modules --extract ./node_modules # In a later run.
```

## Supported Object Stores

The only currently supported object store is the [filesystem object store](./filesystem.md). Reference the
[configuration reference](../server_configuration/configuration_reference.md) for a full list of configuration settings
and options.

## How to add new Object Stores?

Object stores are pluggable, but for them to maintain good performance and simplicity the code that orchestrates them
must be added to the object_store folder within Gofer(which means they have to be written in Rust).
