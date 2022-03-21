---
id: task-stanza
title: Task stanza
sidebar_position: 1
---

# Task <small>_Stanza_</small>

The Task stanza represents a single container in your pipeline. It can be configured to depend on other containers. You can use multiple task blocks to declare multiple containers you'd like to execute for each pipeline run.

## Task Parameters

| Param         | Type                            | Description                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| ------------- | ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [label]       | `string: <required>`            | The name of your task. This string cannot have any spaces or special characters and is limited to 70 characters.                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| [label]       | `string: <required>`            | The docker repository and image name of your docker image.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| description   | `string: <optional>`            | A short description of the purpose of your pipeline. Limited to 3k characters.                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| env_vars      | `map[string]string: <optional>` | The main mechanism of passing configuration to your container. The variables mentioned here are passed to your container by Gofer and the associated scheduler. The correct way to define these are in the form: `env_vars = { "WAIT_DURATION": "10s",}` <br/> <br/> The `env_vars` attribute also supports secret/store substitution by using Gofer's secret or object store and special syntax to denote a variable exists in this field. `env_vars = { "SECRET_VALUE": "secret{{secret_key}}"}`<br/><br/> **DO NOT PUT PLAINTEXT SECRETS IN THIS FIELD** |
| registry_auth | `block: <optional>`             | If your image needs registry authentication you can pass the user/pass combo in here as a block. `registry_auth { user = "me", pass = "secret{{my_pass}}"}`. <br/><br/> You can also use Gofer's secret store here to substitute secret values.                                                                                                                                                                                                                                                                                                             |
| exec          | `block: <optional>`             | Exec allows you to run simple shell commands against your container. Useful for debugging or small programs.                                                                                                                                                                                                                                                                                                                                                                                                                                                |

## Task Examples

### A simple task with an exec statement

```hcl
task "simple_task" "ubuntu:latest" {
  description = "This task simply prints our hello-world message and exits!"
  exec "/bin/bash" {
    script = <<EOT
    echo Hello from Gofer!
    EOT
  }
}
```

### A task with a dependency on another task with success requirement

```hcl
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

### A task with registry authentication

```hcl
task "depends_on_one" "ghcr.io/clintjedwards/gofer-containers/debug/log:latest" {
	description = <<EOT
This task depends on the first task to finish with a successfull result. This means
that if the first task fails this task will not run.
EOT
    registry_auth {
        user = "my_username"
        pass = "secret{{my_key_to_pass}}"
    }
    depends_on = {
        "no_dependencies": "successful",
    }
    env_vars = {
        "LOGS_HEADER": "This string can be anything you want it to be",
    }
}
```

### A task with variable substitution from the pipeline object store

```hcl
task "no_dependencies" "ghcr.io/clintjedwards/gofer-containers/debug/log:latest" {
  description = "This task has no dependencies so it will run immediately"

  // The env_variable mentioned here is special, for this example we're pretending its a value we've stored in our
  // pipeline store.
  // As such we use the special secret syntax to convey to Gofer that the valuhe must be retrieved from the object store
  // beforehand.
  env_vars = {
    "SOME_VARIABLE" : "something here",
    "LOGS_HEADER" : "pipeline{{ logs_header }}",
  }
}
```
