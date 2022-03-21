---
id: overview
title: Overview
sidebar_position: 1
---

# Pipeline Configuration

A pipeline is a directed acyclic graph of tasks that run together. A single execution of a pipeline is called a run. Gofer allows users to configure their pipeline via a configuration file written in [HCL](https://github.com/hashicorp/hcl).

This pipeline file is usually stored alongside code or can be anywhere the gofer service can reach it remotely. You can also leverage the [CLI](../cli/gofer) and locally create and upload pipeline files.

The general hierarchy for a pipeline is:

```
pipeline
    \_ run
         \_ task
```

Each execution of a pipeline is a run and every run consists of one or more tasks. These tasks are where users specify their containers and settings.

## Example

This example shows a sample pipeline configuration file. We tried to keep it as simple as possible, while still showcasing some common use cases that you might have. For a more detailed explanation of these fields, please use the navbar to dive deeper.

```hcl
id          = "trigger_test_pipeline"
name        = "[with_trigger] Gofer Test Pipeline"
description = <<EOT
This pipeline shows off the various features of a simple Gofer pipeline. Triggers, Tasks, and
dependency graphs are all tools that can be wielded to create as complicated pipelines as need be.
EOT

// Triggers are plugins that control the automatic execution of pipeline.
// They typically take some kind of configuration which controls the behavior of the trigger.
// The name here "interval" denotes the "kind" of trigger. Gofer supports multiple trigger kinds.
// A list of trigger kinds can be found in the documentation or via the command line:
// `gofer trigger list`
//
// You can tie triggers to a label; useful for remembering why they were created and
// referencing multiple instances of a single trigger kind quickly. The label "every_one_minute"
// here is a short and quick summary of our interval trigger's purpose.
trigger "interval" "every_one_minute" {
  every = "1m"
}

// Tasks are the building blocks of a pipeline. They represent individual containers and can be
// configured to depend on one or multiple other tasks.
task "no_dependencies" "ghcr.io/clintjedwards/gofer-containers/debug/wait:latest" {
  description = "This task has no dependencies so it will run immediately"

  // Environment variables are the way in which your container is configured.
  // These are passed into your container at runtime. The way to pass variables to your container
  // should always be through environment variables except in very rare circumstances: https://12factor.net/config
  env_vars = {
    "WAIT_DURATION" : "10s",

    // Gofer handles secrets also! Simply use the command-line to enter secrets and then reference them
    // in your config file by key.
    "SECRET_LOGS_HEADER" : "secret{{secret_log_key}}"
  }
}

task "depends_on_one" "ghcr.io/clintjedwards/gofer-containers/debug/log:latest" {
  description = <<EOT
This task depends on the first task to finish with a successfull result. This means
that if the first task fails this task will not run.
EOT
  depends_on = {
    "no_dependencies" : "successful",
  }
  env_vars = {
    "LOGS_HEADER" : "This string can be anything you want it to be",
  }
}

task "depends_on_all" "ghcr.io/clintjedwards/gofer-containers/debug/log:latest" {
  description = <<EOT
This task depends on all other tasks completing successfully. This means that even though task "no_dependencies" has
finished it will wait until "depends_on_one" has exited.
EOT
  depends_on = {
    "no_dependencies" : "successful",
    "depends_on_one" : "successful",
  }
  env_vars = {
    "LOGS_HEADER" : "This string can be anything you want it to be",
  }
}
```
