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


## What is in it?

Here is `main.go` with its long description shortened:

```go
package main

import (
	"log"

	sdk "github.com/clintjedwards/gofer/sdk/go/config"
)

func main() {
	err := sdk.NewPipeline("simple", "Simple Pipeline").
		Description("This pipeline shows off a very simple Gofer pipeline...").
		Tasks(
			sdk.NewTask("simple-task", "ubuntu:latest").
				Description("This task simply prints our hello-world message and exits!").
				Command("echo", "Hello from Gofer!").Variables(map[string]string{"test": "sample"}),
		).Finish()
	if err != nil {
		log.Fatal(err)
	}
}
```

A pipeline consists of a few parts:

- **An id and a name.** `simple` is the id you'll use in commands; "Simple Pipeline" is the name people see.
- **A description** so others know what it's for.
- **One or more [tasks](../ref/pipeline_configuration/tasks.md).** Each task is a container to run. This one runs
  `ubuntu:latest`, prints a message, and gets an environment variable `TEST=sample`.
- **`Finish()`**, which checks the pipeline for mistakes and prints it out.

## Try it

A pipeline config is an ordinary program, so you can run it yourself:

```bash
go run .
```

It prints your pipeline as JSON. That JSON is what Gofer actually receives; in the next step, `gofer up` runs this
program for you and uploads what it prints. If something is wrong with the pipeline, like a task that depends on one
that doesn't exist, `go run .` tells you here before anything reaches Gofer.
