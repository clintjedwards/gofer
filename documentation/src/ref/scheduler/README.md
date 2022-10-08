# Scheduler

Gofer runs the containers you reference in the pipeline configuration via a container orchestrator referred to here as a "scheduler".

The vision of Gofer is for you to use whatever scheduler your team is most familiar with.

## Supported Schedulers

The only currently supported scheduler is [local docker](docker/overview). This scheduler is used for small deployments
and development work.

## How to add new Schedulers?

Schedulers are pluggable! Simply implement a new scheduler by following [the given interface.](https://github.com/clintjedwards/gofer/blob/main/internal/scheduler/scheduler.go#L63)

```go
{{#include ../../../../internal/scheduler/scheduler.go:81:}}
```
