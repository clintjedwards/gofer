---
id: pipeline-stanza
title: Pipeline settings
sidebar_position: 1
---

# Pipeline <small>_Stanza_</small>

The Pipeline stanza is the top most configuration layer. It contains not only general information about your pipeline, but also all other stanzas that might be defined.

## Pipeline Parameters

| Param                                | Type                     | Description                                                                                                                   |
| ------------------------------------ | ------------------------ | ----------------------------------------------------------------------------------------------------------------------------- |
| id                                   | `string: <required>`     | The id of your pipeline. This should be a short, non-whitespaced name. You'll use this to refer to this pipeline              |
| name                                 | `string: <required>`     | The name of your pipeline. This should be a short, recognizable moniker as the limit is 70 characters.                        |
| description                          | `string: <optional>`     | A short description of the purpose of your pipeline. Limited to 3k characters.                                                |
| sequential                           | `bool: <optional:false>` | Limit pipeline to only one run at a time.                                                                                     |
| [task](../task/task-stanza)          | `Task { <required>`      | One or more [Task](../task/task-stanza) stanzas where you define the settings for the containers you want to run.             |
| [trigger](../trigger/trigger-stanza) | `Trigger { <optional>`   | One or more [Triggers](../trigger/trigger-stanza) can be used automate your pipeline runs. Gofer supports many trigger types. |

## Pipeline Examples

### Simple Pipeline with a trigger

```hcl
id   = "pipeline_w_trigger"
name = "[with_trigger] Gofer Test Pipeline"
description = <<EOT
This pipeline shows of the various features of a simple gofer pipeline. Triggers, Tasks, and
dependency graphs are all tools that can be wielded to create as complicated pipelines as need be.
EOT

// Triggers are plugins that control the automatic execution of pipeline.
// They typically take some kind of configuration which controls the behavior of the trigger.
// The name here "interval" denotes the "kind" of trigger. Gofer supports multiple trigger kinds.
// A list of trigger kinds can be found in the documentation or via the command line:
// `gofer trigger list`
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
        "WAIT_DURATION": "10s",
        "SECRET_LOGS_HEADER": "secret{{secret_key}}"
    }
}

task "depends_on_one" "ghcr.io/clintjedwards/gofer-containers/debug/log:latest" {
	description = <<EOT
This task depends on the first task to finish with a successfull result. This means
that if the first task fails this task will not run.
EOT
    depends_on = {
        "no_dependencies": "successful",
    }
    env_vars = {
        "LOGS_HEADER": "This string can be anything you want it to be",
    }
}
```
