---
id: task-stanza
title: Task stanza
sidebar_position: 1
---

# Task <small>_Stanza_</small>

The Task stanza represents a single container in your pipeline. It can be configured to depend on other containers. You can use multiple task blocks to declare multiple containers you'd like to execute for each pipeline run.

## Task Parameters

| Param       | Type                            | Description                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ----------- | ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [label]     | `string: <required>`            | The name of your task. This string cannot have any spaces or special characters and is limited to 70 characters.                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| Description | `string: <optional>`            | A short description of the purpose of your pipeline. Limited to 3k characters.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ImageName   | `string: <required>`            | The docker repository and image name of your docker image.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| EnvVars     | `map[string]string: <optional>` | The main mechanism of passing configuration to your container the environment variables mentioned here are passed to your container by gofer and the associated scheduler. The correct way to define these are in the form: `env_vars = { "WAIT_DURATION": "10s",}`                                                                                                                                                                                                                                                                                           |
| Secrets     | `map[string]string: <optional>` | The main mechanism of secret handling to your container. **DO NOT PUT PLAINTEXT SECRETS IN THIS FIELD**. Instead the key is the environment variable of the secret and the value is the secret "path" or config string. <br/><br/> Since Gofer relies on the scheduler to handle secrets view the scheduler documentation on how secrets should be handled. For example, for a scheduler that uses vault as a backend, the value of a secret field might be the path of that secret in vault:`secrets = { "A_SIMPLE_SECERT": "/some/path/within/vault:key",}` |

## Referencing images in private docker registries

Gofer pipeline configurations do not support secret values, but you might have to pass authentication in order for your scheduler to be able to pull the docker images that you need from a private repository. To solve this, Gofer supports [registry auth](../../cli/gofer_service_registry) which requires an admin to insert the requried docker registry authentication before private registries can be accessed.

Once a registry is entered, images from that registry will be pulled using the provided credentials.

## Task Examples

### A simple task with no dependencies

```hcl
// Tasks are the building blocks of a pipeline. They represent individual containers and can be
// configured to depend on one or multiple other tasks.
task "no_dependencies" {
	description = "This task has no dependencies so it will run immediately"

    // Environment variables are the way in which your container is configured.
    // These are passed into your container at runtime.
    env_vars = {
        "WAIT_DURATION": "20s",
    }

    // Secrets are specified here to be pulled in from the scheduler.
    // Scheduler configuration determines how the scheduler actually retrieves these secrets
    // the key is what env var the secret should be injected as, the value is any extra configuration
    // the scheduler might need to retrieve the secert(for vault it might be the path to the secret)
    secrets = {
        "SECRET_LOGS_HEADER": "example/config/for/secrets"
    }

	image_name = "ghcr.io/clintjedwards/experimental:wait"
}
```

### A task with a dependency on another task with success requirement

```hcl
task "depends_on_one" {
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
	image_name = "ghcr.io/clintjedwards/experimental:log"
}
```
