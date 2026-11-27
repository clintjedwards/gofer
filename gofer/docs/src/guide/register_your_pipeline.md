# Register your pipeline

Now we will register your newly created pipeline configuration with Gofer!

## More CLI to the rescue

From your terminal, let's use the Gofer binary to run the following command, pointing Gofer at your newly created pipeline folder:

```bash
gofer up /tmp/simple_pipeline
```

## Examine created pipeline

It's that easy!

The Gofer command line application uses your local Golang compiler to compile and run your pipeline configuration.
(For a Rust pipeline it uses `cargo` instead.) Then it makes each task's image on your machine, and uploads the
images and the config to Gofer. Each version of a pipeline keeps the images it was registered with, so it runs the
same thing every time.

You should have received a success message and some suggested commands:

```bash
 New pipeline 'simple' with 1 task(s): simple-task
 ✓ simple-task  built in 1.2s
 ✓ Registered pipeline: [simple] 'Simple Pipeline' v1
   View details of your pipeline: gofer fetch simple
   Start a new run: gofer pipeline run simple
```

This task is only a base image and a command, so making its image doesn't need Docker. A task that copies files in or
runs commands is built with Docker on your machine; see [the task's image](../ref/pipeline_configuration/tasks.md#the-tasks-image).
Run `gofer up` again after changing your pipeline, and it only builds the tasks that changed.

We can view the details of our new pipeline by running:

```bash
gofer fetch simple
```

If you ever forget your pipeline ID you can list all pipelines in your namespace by using:

```bash
gofer pipeline list
```
