# Register your pipeline

Now we will register your newly created pipeline configuration with Gofer!

## More CLI to the rescue

From your terminal, lets use the Gofer binary to run the following command, pointing Gofer at your newly created pipeline folder:

```bash
gofer pipeline create ./tmp/simple_pipeline
```

## Examine created pipeline

It's that easy!

The Gofer command line application uses your local Golang compiler to compile, parse, and upload your pipeline configuration to Gofer.

You should have received a success message and some suggested commands:

```bash
 ✓ Created pipeline: [simple] "Simple Pipeline"

  View details of your new pipeline: gofer pipeline get simple
  Start a new run: gofer runs start simple
```

We can view the details of our new pipeline by running:

```bash
gofer pipeline get example_pipeline
```

If you ever forget your pipeline ID you can list all pipelines that you own by using:

```bash
gofer pipeline list
```
