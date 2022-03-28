---
sidebar_position: 1
---

# Glossary

- **Pipeline:** A pipeline is a collection of tasks that can be run at once. Pipelines can be defined via a [pipeline configuration file](pipeline-configuration/overview.md). Once you have a pipeline config file you can create a new pipeline via the [API](API.md) or [CLI](cli/gofer_pipeline_create.md).

- **Run:** A run is a single execution of a pipeline. A run can be started automatically via [triggers](triggers/overview.md) or manually via the [API](API.md) or [CLI](cli/gofer_run_start.md)

- **Trigger:** A trigger is an automatic way to run your pipeline. Once mentioned in your [pipeline configuration file](pipeline-configuration/overview.md), your pipeline _subscribes_ to those triggers, passing them conditions on when to run. Once those conditions are met, those triggers will then inform Gofer that a new run should be launched for that pipeline.

- **Task:** A task is the lowest unit in Gofer. It is a small abstraction over running a single container. Through tasks you can define what container you want to run, when to run it in relation to other containers, and what variables/secrets those containers should use.

- **Task Run:** A task run is an execution of a single task container. Referencing a specific task run is how you can examine the results, logs, and details of one of your tasks.

- **Notifier:** A notifier is a task that runs after your task runs have all finished and reports the result to some other entity.
