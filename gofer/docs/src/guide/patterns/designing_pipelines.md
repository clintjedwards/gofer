# Designing Pipelines

## Keep things simple.

You could build a huge graph of containers that all depend on each other. But the main theme of Gofer is simplicity.
Before splitting work into separate tasks, think about whether it actually helps. Separate tasks are good for running
things in parallel, using different images, or reacting differently to one step failing. They cost you a container
start each and you have to [pass files between them](./passing_data_between_tasks.md) through the object store. When
in doubt, pick whichever version is easier for the next person to understand.

## Use `Any` for cleanup and notifications.

A task that depends on another with `RequiredParentStatusSuccess` is skipped when that parent fails, and so is
everything below it. For tasks that should run no matter what, like tearing down a test environment or posting the
result to chat, depend on the parent with `RequiredParentStatusAny`. Use `RequiredParentStatusFailure` for things that
should only happen when something breaks. `gofer run debug <pipeline> <run>` shows why each task was skipped when the
graph doesn't do what you expected.

## Limit pipelines that can't overlap.

By default a pipeline can have many runs going at once. That's fine for tests, but not for a deploy, or anything that
updates a shared pipeline object like a cache, where the last run to finish wins. Set
[`Parallelism(1)`](../../ref/pipeline_configuration/index.html#limiting-concurrent-runs) on those pipelines so new runs
wait for the current one to finish.

## Keep your pipeline config next to your code.

Pipeline configs are code, so keep them in the repository they build and review them like any other change. Run
`gofer up` from the commit you want live, and roll back by running it again from an older commit. If you want a new
version stored in Gofer without it going live yet, use `gofer up --deploy false`. See
[updating pipelines](../../ref/pipeline_configuration/updating.md) for how versions and deployments work.
