---
sidebar_position: 3
---

# 3) Create your pipeline

Now that we have a pipeline config/definition we can create our first pipeline.

## More CLI to the rescue

You generally have two ways to pass a pipeline to Gofer. You can create one locally like we just have or you can check it into source control and point Gofer to it.

Lets use the local method. Running this command should contact the gofer service and create our pipeline:

```shell
gofer pipeline create ./example.gofer.hcl
```

:::tip

Anytime you make changes to a pipeline config you can check that it is valid by using the `validate` command:

```shell
gofer config validate ./example.gofer.hcl
```

There are many other commands that work with pipeline config files. You can view them by using:

```shell
gofer config -h
```

:::

:::note

Passing a url to the `pipeline create` command will instead tell Gofer to look for the pipeline remotely.
It's common to store your pipeline file alongside your code and update it using source control.

```shell
gofer pipeline create https://raw.githubusercontent.com/clintjedwards/gofer/examplePipelines/simple.hcl
```

```shell
gofer pipeline create github.com/clintjedwards/gofer.git//myFolderPipeline
```

You can find an explanation on the format of the URL here on the [pipeline create cli page](../../cli/gofer_pipeline_create).

:::

## Examine created pipeline

It's that easy! You should have received a success message and some suggested commands:

```shell
✓ Created pipeline: [example_pipeline] "[example] Gofer Example Pipeline"

  View details of your new pipeline: gofer pipeline get example_pipeline
  Start a new run: gofer run start example_pipeline
```

We can view the details of our new pipeline by running:

```shell
gofer pipeline get example_pipeline
```

If you ever forget your pipeline ID you can list all pipelines that you own by using:

```shell
gofer pipeline list
```
