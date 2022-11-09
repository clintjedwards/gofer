# Provided Triggers

Gofer provides some pre-written triggers for quick use:

| name                      | image                                                | included by default | description                                                                                                       |
| ------------------------- | ---------------------------------------------------- | ------------------- | ----------------------------------------------------------------------------------------------------------------- |
| [interval](./interval.md) | ghcr.io/clintjedwards/gofer/triggers/interval:latest | yes                 | Interval triggers an event after a predetermined amount of time has passed.                                       |
| [cron](./cron.md)         | ghcr.io/clintjedwards/gofer/triggers/cron:latest     | yes                 | Cron is used for longer termed, more nuanced intervals. For instance, running a pipeline every year on Christmas. |
| [github](./github.md)     | ghcr.io/clintjedwards/gofer/triggers/github:latest   | no                  | Allow your pipelines to run based on branch, tag, or release activity.                                            |
