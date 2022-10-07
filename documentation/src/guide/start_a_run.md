# Start a Run

Now that we've set up Gofer, defined our pipeline, and registered it we're ready to actually run our containers.

## Press start

```bash
gofer pipelines run simple
```

## What happens now?

When you start a run Gofer will attempt to schedule all your tasks according to their dependencies onto your chosen scheduler. In this case that scheduler is your local instance of Docker.

Your run should be chugging along now!

#### View a list of runs for your pipeline:

```bash
gofer runs list simple
```

#### View details about your run:

```bash
gofer runs get simple 1
```

#### List the containers that executed during the run:

```bash
gofer taskruns list simple 1
```

#### View a particular container's details during the run:

```bash
gofer taskruns get simple 1 <task_id>
```

#### Stream a particular container's logs during the run:

```shell
gofer taskruns logs simple 1 <task_id>

```
