# Scheduler

Gofer runs the containers you reference in the pipeline configuration via a container orchestrator referred to here as a "scheduler".

The vision of Gofer is for you to use whatever scheduler your team is most familiar with.

## Supported Schedulers

The only currently supported scheduler is [local docker](../scheduler/docker.md). This scheduler is used for small deployments
and development work.

## How to add new Schedulers?

Schedulers are pluggable, but for them to maintain good performance and simplicity the code that orchestrates them must
be added to the schedulers folder within Gofer(which means they have to be written in Rust).
