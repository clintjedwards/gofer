# Skipping Runs

## Give a pause an end date.

When you want a pipeline to stop for a while, tell Gofer when it should start again instead of trying to remember:

```bash
gofer pipeline disable simple --for 7d
gofer pipeline disable simple --until 2026-12-01
```

`--until` turns the pipeline back on at the start of that day in your local time. Until then nothing can start a run.
Runs that are already going finish normally, and runs still waiting in the queue are cancelled. Changed your mind?
`gofer pipeline enable simple` turns it back on early, and running `disable` again replaces the end date.

## Skip just the next run.

For a pipeline that runs once a day, `--for 24h` covers exactly one run no matter when you run it:

```bash
gofer pipeline disable simple --for 24h
```

For anything less regular, pick a time that's after the run you want to skip and before the one you want to keep.
