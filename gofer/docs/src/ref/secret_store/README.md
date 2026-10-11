# Secret Store

Gofer provides a secret store as a way to enable users to pass secrets into pipeline configuration
files.

The secrets included in the pipeline file use a special syntax so that Gofer understands when it is given a
secret value instead of a normal variable:

```go
Variables(map[string]string{
    "DEPLOY_KEY":  sdk.PipelineSecret("deploy_key"),  // or "pipeline_secret{{deploy_key}}"
    "SLACK_TOKEN": sdk.GlobalSecret("slack_token"),   // or "global_secret{{slack_token}}"
})
```

Gofer swaps in the real value right before the task's container starts, so the secret never appears in your
pipeline config. There are two kinds of secrets:

- **Pipeline secrets** belong to a single pipeline and are set with
  `gofer secret pipeline put <pipeline> <key>`.
- **Global secrets** are shared across pipelines and only admins can manage them. Each one has a list of
  namespaces (regexes) that are allowed to use it, set with `gofer secret global put <key> -n <namespace regex>`.

See [Using Secrets and Objects in Variables](../pipeline_configuration/tasks.md#using-secrets-and-objects-in-variables)
for more detail.

```toml
[secret_store]
engine = "sqlite"
```

## Supported Secret Stores

The only currently supported secret store is the [sqlite secret store](./sqlite.md). Reference the
[configuration reference](../server_configuration/configuration_reference.md) for a full list of configuration
settings and options.

## How to add new Secret Stores?

Secret stores are pluggable, but for them to maintain good performance and simplicity the code that orchestrates them must
be added to the secret_store folder within Gofer(which means they have to be written in Rust).
