---
id: overview
title: Overview
sidebar_position: 1
---

# Docker <small>scheduler</small>

The docker scheduler uses the machine's local docker engine to run containers. This is great for small or development workloads and very simple to implement. Simply download docker and go!

```hcl
scheduler {
  engine = "docker"
  docker {
    prune          = true
    prune_interval = "24h"
  }
}
```

## Secrets

Local Docker does not have native secret storage so instead the secret storage solution is a text file kept locally. You input the location of this text file via the `secrets_path` parameter in server configuration.

The format of this secret file is `ENVIRONMENT_VARIABLE=secret`; one variable per line.

An example:

```bash
# ./docker_secrets
TEST_SECRET=mysupersecretthing
ANOTHER_SECRET=myothersupersecretthing
```

Create this file anywhere you feel is secure, make sure Gofer has read access to it, and point Gofer at it using the `secrets_path` parameter.

## Configuration

Docker needs to be installed and the Gofer process needs to have the required permissions to run containers upon it.

Other than that the docker scheduler just need to know how to clean up after itself and provide secrets.

| Parameter      | Type             | Default         | Description                                                                                                                                                                                                     |
| -------------- | ---------------- | --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| prune          | bool             | false           | Whether or not to periodically clean up containers that are no longer in use. If prune is not turned on eventually the disk of the host machine will fill up with different containers that have run over time. |
| prune_interval | string(duration) | 24h             | How often to run the prune job. Depending on how many containers you run per day this value could easily be set to monthly.                                                                                     |
| secrets_path   | string           | None (Optional) | The location of the secrets file.                                                                                                                                                                               |
