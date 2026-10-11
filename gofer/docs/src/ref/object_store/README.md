# Object Store

Gofer provides an object store as a way to share values and objects between containers. It can also be used as a cache.
It is common for one container to run, generate an artifact or values, and then store that object in the object store
for the next container or next run. The object store can be accessed through the [Gofer CLI](../../cli/index.html) or
through the normal Gofer API. Tasks can also receive objects directly as environment variables; see
[Using Secrets and Objects in Variables](../pipeline_configuration/tasks.md#using-secrets-and-objects-in-variables).

Gofer divides the objects stored into two different lifetime groups:

## Pipeline-level objects

Gofer can store objects permanently for each pipeline. You can store objects at the pipeline-level by using the
gofer pipeline object store command:

```bash
gofer pipeline object put my-pipeline my_key1 ./some_file
echo "my_value" | gofer pipeline object put my-pipeline my_key2 @
gofer pipeline object get my-pipeline my_key1
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
gofer run object put my-pipeline 1 my_key ./some_file
gofer run object get my-pipeline 1 my_key
```

## Passing data between tasks

The most common use of run objects is one task handing a value to the tasks after it. The first task stores the
object through Gofer's API, and the tasks that depend on it read it as an environment variable.

```go
const storeVersion = `
VERSION="1.4.2" # Stand in for something your task works out, like a build number.
curl -sf -X POST \
  -H "Authorization: Bearer $GOFER_TOKEN" \
  -H "gofer-api-version: v0" \
  --data-binary "$VERSION" \
  "$GOFER_URL/api/namespaces/default/pipelines/$GOFER_PIPELINE_ID/runs/$GOFER_RUN_ID/objects/version"
`

err := sdk.NewPipeline("release", "Release").
    Tasks(
        sdk.NewTask("work-out-version", "curlimages/curl:latest").
            InjectAPIToken(true). // Gives the task a GOFER_TOKEN that can write this pipeline's objects.
            Variables(map[string]string{"GOFER_URL": "http://172.17.0.1:8080"}).
            Command("sh", "-c", storeVersion),

        sdk.NewTask("publish", "alpine:latest").
            DependsOn("work-out-version", sdk.RequiredParentStatusSuccess).
            Variables(map[string]string{"VERSION": sdk.RunObject("version")}).
            Command("sh", "-c", "echo publishing version $VERSION"),
    ).Finish()
```

A few things to note:

- **Tasks need to know where Gofer is.** Gofer doesn't tell tasks its own address, so you pass it in yourself
  (`GOFER_URL` above). It has to be an address the container can reach. For a local Gofer using the docker scheduler,
  that's the docker bridge's host address, `http://172.17.0.1:8080` on Linux; it's usually the same address you set
  for `server.extension_address`.
- **The child has to depend on the parent.** Gofer reads `run_object{{version}}` right before `publish` starts, so
  `publish` has to wait for `work-out-version` with `DependsOn`. Without it both tasks start at once and `publish`
  fails because the object doesn't exist yet.
- **The namespace isn't passed to tasks either**, so it's written into the URL. Change `default` if your pipeline
  lives somewhere else.
- **Use pipeline objects for things that should outlast the run**, like a cache: the same request with
  `/pipelines/$GOFER_PIPELINE_ID/objects/<key>` instead, and `sdk.PipelineObject` to read it.

## Supported Object Stores

The only currently supported object store is the [filesystem object store](./filesystem.md). Reference the [configuration reference](../server_configuration/configuration_reference.md) for a full list of configuration settings and options.

## How to add new Object Stores?

Object stores are pluggable, but for them to maintain good performance and simplicity the code that orchestrates them must
be added to the object_store folder within Gofer(which means they have to be written in Rust).
