# Passing Data Between Tasks

Tasks don't share a filesystem, so use the [object store](../../ref/object_store/index.html) instead. It's Gofer's
replacement for a shared workspace.

- **Run objects** live for a single run (plus a few after it, see `run_object_expiry`). Use them for handing a value
  or a file from one task to the tasks after it.
- **Pipeline objects** last across runs until the pipeline hits `pipeline_object_limit`. Use them for caches.

A task stores objects through Gofer's API, so give it `InjectAPIToken(true)`. Gofer sets `GOFER_API_BASE_URL`,
`GOFER_NAMESPACE`, `GOFER_PIPELINE_ID` and `GOFER_RUN_ID` on every task, so the request needs no other setup. The Gofer
CLI reads the same variables, along with `GOFER_TOKEN`, so if your image has the `gofer` binary it works with no
setup either.

## Small values

For something like a version number, have the first task store it as a run object and have the tasks after it read
it as an environment variable:

```go
const storeVersion = `
VERSION="1.4.2" # Stand in for something your task works out, like a build number.
curl -sf -X POST \
  -H "Authorization: Bearer $GOFER_TOKEN" \
  -H "gofer-api-version: v0" \
  --data-binary "$VERSION" \
  "$GOFER_API_BASE_URL/api/namespaces/$GOFER_NAMESPACE/pipelines/$GOFER_PIPELINE_ID/runs/$GOFER_RUN_ID/objects/version"
`

err := sdk.NewPipeline("release", "Release").
    Tasks(
        sdk.NewTask("work-out-version", "curlimages/curl:latest").
            InjectAPIToken(true). // Gives the task a GOFER_TOKEN that can write this pipeline's objects.
            Command("sh", "-c", storeVersion),

        sdk.NewTask("publish", "alpine:latest").
            DependsOn("work-out-version", sdk.RequiredParentStatusSuccess).
            Variables(map[string]string{"VERSION": sdk.RunObject("version")}).
            Command("sh", "-c", "echo publishing version $VERSION"),
    ).Finish()
```

With the `gofer` binary, `echo "$VERSION" | gofer run object put version @` does the same thing as the curl call.

## Files and directories

For anything bigger, or anything binary, upload it in one task and download it in the next:

```bash
# Upload a directory as a run object.
tar -czf - ./dist | curl -sf -X POST \
  -H "Authorization: Bearer $GOFER_TOKEN" -H "gofer-api-version: v0" --data-binary @- \
  "$GOFER_API_BASE_URL/api/namespaces/$GOFER_NAMESPACE/pipelines/$GOFER_PIPELINE_ID/runs/$GOFER_RUN_ID/objects/dist"

# Download it in a later task.
curl -sf -H "Authorization: Bearer $GOFER_TOKEN" -H "gofer-api-version: v0" \
  "$GOFER_API_BASE_URL/api/namespaces/$GOFER_NAMESPACE/pipelines/$GOFER_PIPELINE_ID/runs/$GOFER_RUN_ID/objects/dist" \
  | tar -xzf -
```

The `gofer` binary does the tar work for you. Inside a task it picks up the pipeline and run from the environment, so
all you need is the key and the directory:

```bash
gofer run object put dist ./dist           # Uploads the directory as a tarball.
gofer run object get dist --extract ./dist # Downloads and unpacks it.
```

## Things that trip people up

- **The reading task has to depend on the writing one** with `DependsOn`. Tasks run in parallel by default, so
  without it the object might not exist yet. For variables, Gofer reads the object right before the task starts, so
  the task fails before its container even runs.
- **Download files through the API, not as variables.** `sdk.RunObject("key")` works well for small text values, but
  it converts the object to text, so it'll mangle a tarball or any other binary file.
- **Pipeline objects can disappear.** The oldest ones get deleted once the pipeline hits `pipeline_object_limit`, so
  write tasks that read them to still work when they're missing. To store one, use the same requests with
  `/pipelines/$GOFER_PIPELINE_ID/objects/<key>` in place of the run path, and `sdk.PipelineObject` to read it.
- **If tasks can't reach Gofer, check `server.extension_address`.** That's where `GOFER_API_BASE_URL` comes from,
  since tasks and extensions sit on the same container network.
