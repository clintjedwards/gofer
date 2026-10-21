# Updating Pipelines

Every time you run `gofer up` with changes, Gofer stores your pipeline config as a new **version** (`v1`, `v2`, and so
on) instead of overwriting the old one. Then it **deploys** that version, which makes it the one new runs use.

Before it registers anything, `gofer up` shows what's different from the latest version and asks you to confirm:

```text
$ gofer up ./my-pipeline
 Changes since v2:
   │ ~ tasks.build.command:
   │       sh -c 'exec "$(command -v bash || echo sh)" -c "$1"' gofer-script 'echo one
   │     - echo two
   │     + echo 2
   │       echo three'
   │ ~ tasks.build.image: alpine:latest -> alpine:3.20
   │ + tasks.notify: new task using alpine:latest
 ? Register and deploy these changes? [y/N]: y
 ✓ Registered pipeline: [my-pipeline] 'My Pipeline' v3
```

Pass `--yes` to skip the question. Gofer also skips it when it isn't run from a terminal, like in CI. If nothing
changed, no new version is made; Gofer just deploys the latest version if it isn't live yet.

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
later, without it taking effect yet. When you're ready, `gofer pipeline config deploy <pipeline_id> <version>` makes
it live.

A brand new pipeline registered this way has no live version, so it can't run until one is deployed.

## Deployments

Each time a version goes live is a **deployment**. Deploying swaps which version is live: the new one becomes `Live`
and the old one becomes `Deprecated`. Runs that are already going keep running the version they started with.

`gofer pipeline deployment list <pipeline_id>` shows a pipeline's deployment history, which is useful for working out
when a change went out. Only one deployment can happen at a time per pipeline.

## Rolling back

To roll back, deploy the earlier version again. Find it with `gofer pipeline config list <pipeline_id>`, then:

```bash
gofer pipeline config deploy my-pipeline 2
```

That makes version 2 live and deprecates the current one, without registering anything new.

You can also `gofer up` the old code again, for example from an earlier commit. That registers it as a new version
and deploys it, which is the way to go if the version you want has been deleted.

## Cleaning up old versions

`gofer pipeline config delete <pipeline_id> <version>` removes a version. You can't delete the live version or a
pipeline's only version.
