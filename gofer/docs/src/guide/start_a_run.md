# Start a Run

Now that we've set up Gofer, defined our pipeline, and registered it we're ready to actually run our containers.

## Press start

```bash
gofer pipeline run simple
```

## What happens now?

When you start a run Gofer will attempt to schedule all your tasks according to their dependencies onto your chosen scheduler. In this case that scheduler is your local instance of Docker.

Your run should be chugging along now!

## Watch it in the browser

The easiest way to follow a run is the web UI:

```bash
gofer web simple
```

This opens the pipeline's latest run, where you can see each task's state and logs as they happen.

## Or from the terminal

`gofer fetch` is a shortcut for looking things up. Each argument goes one level deeper (pipeline, then run, then
task), and a `+` on the end lists everything at the next level instead.

#### View details about the pipeline:

```bash
gofer fetch simple
```

#### List the pipeline's runs:

```bash
gofer fetch simple +
```

#### View details about run 1:

```bash
gofer fetch simple 1
```

#### List the tasks that ran in run 1:

```bash
gofer fetch simple 1 +
```

#### View the details of one task, including the environment variables it was given:

```bash
gofer fetch simple 1 simple-task
```

#### Show a task's output:

```bash
gofer task logs simple 1 simple-task
```

## When something goes wrong

If a run fails, `gofer run debug simple 1` shows what happened: every task's result, why any were skipped, and the
last lines of output from the ones that failed. The [troubleshooting](../troubleshooting.md) page has more.
