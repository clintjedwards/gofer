# Create Your First Pipeline Configuration

Before you can start running containers you must tell Gofer what you want to run. To do this we create what is called
a `pipeline configuration`.

The creation of this pipeline configuration is very easy and can be done in either Golang or Rust. This allows you to
use a fully-featured programming language to organize your pipelines, instead of dealing with YAML mess.

## Let's Go!

As an example, let's just copy a pipeline that has been given to us already. We'll use Go as our language, which
means you'll need to [install it](https://go.dev/doc/install) if you don't have it. The Gofer repository gives
us a [simple pipeline](https://github.com/clintjedwards/gofer/tree/main/examplePipelines/go/simple) that we can
copy and use.

### Let's first create a folder where we'll put our pipeline:

```bash
mkdir /tmp/simple_pipeline
```

### Then let's copy the Gofer provided pipeline's main file into the correct place:

```bash
cd /tmp/simple_pipeline
wget https://raw.githubusercontent.com/clintjedwards/gofer/main/examplePipelines/go/simple/main.go
```

This should create a `main.go` file inside our `/tmp/simple_pipeline` directory.

### Lastly, let's initialize the new Golang program:

To complete our Go program we simply have to initialize it with the `go mod` command.

```bash
go mod init test/simple_pipeline
go mod tidy
```

The pipeline we generated above gives you a very simple pipeline with a few pre-prepared testing containers. You
should be able to view it using your favorite IDE.

The configuration itself is very simple. Essentially a pipeline contains of a few parts:

#### > Some basic attributes so we know what to call it and how to document it.

```go
err := sdk.NewPipeline("simple", "Simple Pipeline").
		Description("This pipeline shows off a very simple Gofer pipeline that simply pulls in " +
...
```

#### > The containers we want to run are defined through [tasks](../ref/pipeline_configuration/tasks.md).

```go
...
sdk.NewTask("simple_task", "ubuntu:latest").
    Description("This task simply prints our hello-world message and exits!").
    Command("echo", "Hello from Gofer!").Variable("test", "sample"),
...
```
