# Glossary

- **Namespace:** A group of pipelines, usually one per team or project. Permissions and global secrets are scoped by
  namespace. Everything goes in the `default` namespace unless you say otherwise. See [Namespaces](./ref/namespaces.md).

- **Pipeline:** A pipeline is a collection of tasks that can be run at once. Pipelines can be defined via a [pipeline configuration file](./guide/create_your_first_pipeline_configuration.md). Once you have a pipeline config file you can [create a new pipeline via the CLI](./guide/register_your_pipeline.md) (recommended) or API.

- **Pipeline config (version):** One saved copy of a pipeline's configuration. Each `gofer up` stores a new version
  (`v1`, `v2`, ...) instead of replacing the old one. See [Updating Pipelines](./ref/pipeline_configuration/updating.md).

- **Deployment:** Making a pipeline config version the live one, which is the version new runs use. `gofer up` deploys
  by default.

- **Run:** A run is a single execution of a pipeline. A run can be started automatically via [extensions](./ref/extensions/index.html) or manually via the API or [CLI](./cli/index.html)

- **Task:** A task is the lowest unit in Gofer. It is a small abstraction over running a single container. Through tasks you can define what container you want to run, when to run it in relation to other containers, and what variables/secrets those containers should use.

- **Task Execution:** A task execution is the programmatic running of a single task container. Referencing a specific task execution is how you can examine the results, logs, and details of one of your tasks on any given run.

- **Extension:** An extension adds functionality to pipelines. Extensions start up with Gofer as long running
  containers, and pipelines subscribe to them to use them; most commonly to run automatically on a schedule or when
  something happens elsewhere.

- **Subscription:** A pipeline's link to an extension, along with the settings the extension needs for that pipeline
  (like how often to run). Created with `gofer pipeline subscribe`. See
  [Running Pipelines Automatically](./ref/pipeline_configuration/subscriptions.md).

- **Pipeline secret / Global secret:** Values kept in Gofer's [secret store](./ref/secret_store/index.html) for tasks
  to use. Pipeline secrets belong to one pipeline. Global secrets are managed by admins and can be shared with any
  namespaces they allow.

- **Pipeline object / Run object:** Values kept in Gofer's [object store](./ref/object_store/index.html). Pipeline
  objects stay until the pipeline hits its object limit; run objects belong to one run and are deleted after a few
  more runs. Run objects are how tasks pass values to each other.
