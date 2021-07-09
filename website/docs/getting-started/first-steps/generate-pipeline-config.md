---
sidebar_position: 2
---

# 2) Generate a Pipeline Config

Before you can start running containers you must tell Gofer what you want to run. Lets open up another terminal
and get ready to run some more Gofer commands.

## Generate a sample config

The Gofer command line provides a sample configuration for kicking the tires. You can generate it by using:

```shell
gofer config init
```

:::tip

You can also define the path of the created pipeline file by using the `-f` flag.

```shell
gofer config init -f /tmp/myPipeline.hcl
```

:::

## Examining the pipeline config

The generated pipeline config gives you a very simple pipeline with a few pre-prepared testing docker containers. You should be able to view it using your favorite IDE. The command line will have the path that the file was written to: `vim ./example.gofer.hcl`

The sample pipeline also comes with comments to help you understand the basic components of a pipeline file. The foundational knowledge isn't hard though:

Essentially a pipeline consists of:

- Some basic attributes so we know what to call it.
- The containers we want to run are defined through [tasks](/pipeline-configuration/task/task-stanza.md).
- And when we want to automate when the pipeline runs automatically we can do that through [triggers](/pipeline-configuration/trigger/trigger-stanza.md).
