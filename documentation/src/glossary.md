# Glossary

- **Pipeline:** A pipeline is a collection of tasks that can be run at once. Pipelines can be defined via a [pipeline configuration file](./guide/create_your_first_pipeline_configuration.md). Once you have a pipeline config file you can [create a new pipeline via the CLI](./guide/register_your_pipeline.md) (recommended) or API.

- **Run:** A run is a single execution of a pipeline. A run can be started automatically via [extensions](./ref/extensions/README.md) or manually via the API or [CLI](./cli/README.md)

- **Extension:** A extension is an automatic way to run your pipeline. Once mentioned in your [pipeline configuration file](./guide/create_your_first_pipeline_configuration.md), your pipeline _subscribes_ to those extensions, passing them conditions on when to run. Once those conditions are met, those extensions will then inform Gofer that a new run should be launched for that pipeline.

- **Task:** A task is the lowest unit in Gofer. It is a small abstraction over running a single container. Through tasks you can define what container you want to run, when to run it in relation to other containers, and what variables/secrets those containers should use.

- **Task Run:** A task run is an execution of a single task container. Referencing a specific task run is how you can examine the results, logs, and details of one of your tasks.
