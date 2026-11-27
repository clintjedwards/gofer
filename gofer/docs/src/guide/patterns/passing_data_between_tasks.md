# Passing Data Between Tasks

Every task in a run gets the same folder, at `/gofer/run`. Whatever one task leaves there, the tasks after it can read,
the same as files on one machine. Each run starts with an empty one, and it's deleted when the run ends.

The [run folder example pipeline](https://github.com/clintjedwards/gofer/tree/main/examplePipelines/go/run_folder)
puts this together: it clones a repository once, shares a dependency cache between tasks that run at the same time,
packages what was built as another user, and keeps a summary after the run.

## Small values

For something like a version number, have the first task write it to a file and the tasks after it read it back:

```go
err := sdk.NewPipeline("release", "Release").
    Tasks(
        sdk.NewTask("work-out-version", "alpine:3.22").
            // Stand in for something your task works out, like a build number.
            Script(`echo "1.4.2" > /gofer/run/version`),

        sdk.NewTask("publish", "alpine:3.22").
            DependsOn("work-out-version", sdk.RequiredParentStatusSuccess).
            Script(`echo "publishing version $(cat /gofer/run/version)"`),
    ).Finish()
```

## Files and directories

Anything bigger works the same way. Write it into the folder, and read it from there:

```go
sdk.NewTask("build", "golang:1.25").
    Copy("./app", "/src").
    Script("cd /src && go build -o /gofer/run/app ."),

sdk.NewTask("smoke-test", "debian:13-slim").
    DependsOn("build", sdk.RequiredParentStatusSuccess).
    Script("/gofer/run/app --version"),
```

## Keeping something after the run

The folder goes when the run ends, so nothing in it can be fetched afterward. To keep something, like a build artifact
or a report, have a task put it somewhere that lasts: a registry, a bucket, or a
[pipeline object](../../ref/object_store/index.html).

A task stores a pipeline object through Gofer's API, so give it `InjectAPIToken(true)`. Gofer sets `GOFER_TOKEN`,
`GOFER_API_BASE_URL`, `GOFER_NAMESPACE` and `GOFER_PIPELINE_ID` on the task, so the request needs no other setup:

```bash
curl -sf -X POST \
  -H "Authorization: Bearer $GOFER_TOKEN" -H "gofer-api-version: v0" \
  --data-binary @/gofer/run/report.html \
  "$GOFER_API_BASE_URL/api/namespaces/$GOFER_NAMESPACE/pipelines/$GOFER_PIPELINE_ID/objects/report?force=true"
```

If your image has the `gofer` binary, `gofer pipeline object put report /gofer/run/report.html --force` does the same
thing; it picks up the pipeline from the environment. Pipeline objects last across runs, but the oldest ones are
deleted once a pipeline has `pipeline_object_limit` of them, and every run writes to the same keys.

If tasks can't reach Gofer, check `server.extension_address`. That's where `GOFER_API_BASE_URL` comes from, since
tasks and extensions sit on the same container network.

## Things that trip people up

- **The reading task has to depend on the writing one** with `DependsOn`. Tasks run in parallel by default, so
  without it the file might not be there yet.
- **Tasks that run at the same time share the folder too.** Two of them writing the same file overwrite each other,
  so give each one a folder of its own inside `/gofer/run`.
- **What a task makes belongs to the user it ran as.** Any user can write to `/gofer/run` itself, but if a task whose
  image runs as another user needs to change something an earlier task made, have the earlier task open it up, like
  `chmod -R a+w /gofer/run/dist`.
- **Don't put anything at `/gofer` in your images.** Gofer mounts the folder there, which hides whatever the image
  had.
- **It takes disk space while the run is going.** The folder is a Docker volume, so it lives with the rest of
  Docker's data, usually under `/var/lib/docker`, and not in Gofer's own paths. Docker can't limit its size. Only runs
  that are going have one, and a pipeline's
  [`Parallelism`](./designing_pipelines.md#limit-pipelines-that-cant-overlap) caps how many of those it has at once.
