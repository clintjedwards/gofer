# Register your pipeline

Now we will register your newly created pipeline configuration with Gofer!

## More CLI to the rescue

From your terminal, let's use the Gofer binary to run the following command, pointing Gofer at your newly created pipeline folder:

```bash
gofer up /tmp/simple_pipeline
```

## Examine created pipeline

It's that easy!

The Gofer command line application uses your local Golang compiler to compile and run your pipeline configuration,
then uploads the result to Gofer. (For a Rust pipeline it uses `cargo` instead.)

You should have received a success message and some suggested commands:

```bash
 ✓ Registered pipeline: [simple] 'Simple Pipeline' v1

  View details of your pipeline: gofer fetch simple
  Start a new run: gofer pipeline run simple
```

We can view the details of our new pipeline by running:

```bash
gofer fetch simple
```

If you ever forget your pipeline ID you can list all pipelines in your namespace by using:

```bash
gofer pipeline list
```
