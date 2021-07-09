---
sidebar_position: 4
---

# 4) Start a run

Now that we've set up Gofer, defined our pipeline, and registered it we're ready to actually run our containers.

## Press start

```shell
gofer run start example_pipeline
```

## What happens now?

When you start a run Gofer will attempt to schedule all your tasks according to their dependencies onto your chosen scheduler. In this case that scheduler is your local instance of Docker.

Your run should be chugging along now!

#### View a list of runs for your pipeline:

```shell
gofer run list example_pipeline
```

#### View details about your run:

```shell
gofer run get example_pipeline 1
```

#### View a particular container's details during the run:

```shell
gofer taskrun get example_pipeline 1 <task_id>
```

#### Stream a particular container's logs during the run:

```shell
gofer taskrun logs example_pipeline 1 <task_id>

```
