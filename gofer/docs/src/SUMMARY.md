# Gofer

[Introduction](introduction.md)
[How Does Gofer Work?](./how_does_gofer_work.md)
[Glossary](./glossary.md)
[FAQ](./faq.md)
[Feature Guide](./features.md)
[Best Practices](./best_practices.md)
[Troubleshooting](./troubleshooting.md)
[Philosophy](./philosophy.md)

# User Guide

- [Getting Started](./guide/README.md)
  - [Installing Gofer](./guide/installing_gofer.md)
  - [Running the Server Locally](./guide/running_the_server_locally.md)
  - [Create Your First Pipeline Config](./guide/create_your_first_pipeline_configuration.md)
  - [Register your pipeline](./guide/register_your_pipeline.md)
  - [Start a Run](./guide/start_a_run.md)
  - [What's Next?](./guide/whats_next.md)

# Reference

- [Pipeline Configuration](./ref/pipeline_configuration/README.md)
  - [Tasks](./ref/pipeline_configuration/tasks.md)
- [Server Configuration](./ref/server_configuration/README.md)
  - [Configuration Reference](./ref/server_configuration/configuration_reference.md)
  - [Authentication and Authorization](./ref/server_configuration/authz_n.md)
  - [External Events](./ref/server_configuration/external_events.md)
- [Scheduler](./ref/scheduler/README.md)
  - [Docker](./ref/scheduler/docker.md)
- [Object Store](./ref/object_store/README.md)
  - [Filesystem](./ref/object_store/filesystem.md)
- [Secret Store](./ref/secret_store/README.md)
  - [Sqlite](./ref/secret_store/sqlite.md)
- [Extensions](./ref/extensions/README.md)
  - [Manifests](./ref/extensions/manifest.md)
  - [Upgrading Extensions](./ref/extensions/upgrading.md)
  - [Writing Extensions](./ref/extensions/writing_extensions.md)
  - [Gofer's Extensions](./ref/extensions/provided/README.md)
    - [Cron](./ref/extensions/provided/cron.md)
    - [Interval](./ref/extensions/provided/interval.md)
    - [Github](./ref/extensions/provided/github.md)

# CLI

- [Command Line](./cli/README.md)
  - [Configuration](./cli/configuration.md)

# API

<!-- Everything between these markers is written by `make generate-api-docs`; don't edit by hand. -->
<!-- API_DOCS_GEN_START -->
- [API Reference](./api/README.md)
  - [Configs](./api/configs.md)
  - [Deployments](./api/deployments.md)
  - [Events](./api/events.md)
  - [Extensions](./api/extensions.md)
  - [Namespaces](./api/namespaces.md)
  - [Objects](./api/objects.md)
  - [Permissions](./api/permissions.md)
  - [Pipelines](./api/pipelines.md)
  - [Runs](./api/runs.md)
  - [Secrets](./api/secrets.md)
  - [Subscriptions](./api/subscriptions.md)
  - [System](./api/system.md)
  - [Tasks](./api/tasks.md)
  - [Tokens](./api/tokens.md)
  - [Schemas](./api/schemas.md)
<!-- API_DOCS_GEN_END -->
