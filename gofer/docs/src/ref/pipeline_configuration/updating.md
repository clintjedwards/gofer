# Updating Pipelines

Every time you run `gofer up`, Gofer stores your pipeline config as a new **version** (`v1`, `v2`, and so on) instead
of overwriting the old one. Then it **deploys** that version, which makes it the one new runs use.

```text
$ gofer up ./my-pipeline
 ✓ Registered pipeline: [my-pipeline] 'My Pipeline' v3
```

Keeping every version means you can always see exactly what a past run ran with. Each run records its version, and
`gofer run get <pipeline_id> <run_id>` shows it next to the run number.

## Config states

Each version is in one of three states:

| State        | Meaning                                                              |
| ------------ | -------------------------------------------------------------------- |
| `Unreleased` | Registered but never deployed.                                       |
| `Live`       | The deployed version. New runs use this one. There's only ever one. |
| `Deprecated` | Was live before a newer version was deployed.                        |

`gofer pipeline config list <pipeline_id>` shows every version and its state, and
`gofer pipeline config get <pipeline_id> <version>` shows what's in one.

## Registering without deploying

`gofer up --deploy false ./my-pipeline` stores the new version without making it live. Runs keep using the current
live version until a new one is deployed. This is handy when you want the new version in Gofer, to look over or deploy
later, without it taking effect yet.

A brand new pipeline registered this way has no live version, so it can't run until one is deployed.

## Deployments

Each time a version goes live is a **deployment**. Deploying swaps which version is live: the new one becomes `Live`
and the old one becomes `Deprecated`. Runs that are already going keep running the version they started with.

`gofer pipeline deployment list <pipeline_id>` shows a pipeline's deployment history, which is useful for working out
when a change went out. Only one deployment can happen at a time per pipeline.

## Rolling back

The simplest way to roll back is to `gofer up` the old code again, for example from an earlier commit. That registers
it as a new version and deploys it.

You can also deploy any existing version directly through the API:

```bash
curl -X POST \
  -H "Authorization: Bearer $GOFER_TOKEN" \
  -H "gofer-api-version: v0" \
  http://localhost:8080/api/namespaces/default/pipelines/my-pipeline/configs/2
```

## Cleaning up old versions

`gofer pipeline config delete <pipeline_id> <version>` removes a version. You can't delete the live version or a
pipeline's only version.
