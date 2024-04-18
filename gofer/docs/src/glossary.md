# Glossary

- **Pipeline:** A pipeline is a collection of tasks that can be run at once. Pipelines can be defined via a [pipeline configuration file](./guide/create_your_first_pipeline_configuration.md). Once you have a pipeline config file you can [create a new pipeline via the CLI](./guide/register_your_pipeline.md) (recommended) or API.

- **Run:** A run is a single execution of a pipeline. A run can be started automatically via [extensions](./ref/extensions/index.html) or manually via the API or [CLI](./cli/index.html)

- **Extension:** A extension allow for the extension of pipeline functionality. Extension start-up with Gofer as long running containers and
  pipelines can subscribe to them to have additional functionality.

- **Task:** A task is the lowest unit in Gofer. It is a small abstraction over running a single container. Through tasks you can define what container you want to run, when to run it in relation to other containers, and what variables/secrets those containers should use.

- **Task Execution:** A task execution is the programmatic running of a single task container. Referencing a specific task execution is how you can examine the results, logs, and details of one of your tasks on any given run.
