# Secret Store

Gofer provides a secret store as a way to enable users to pass secrets into pipeline configuration
files.

The secrets included in the pipeline file use a special syntax so that Gofer understands when it is given a
secret value instead of a normal variable.

```toml
[secret_store]
engine = "sqlite"
```

## Supported Secret Stores

The only currently supported secret store is the sqlite object store. Reference the
[configuration reference](../server_configuration/configuration_reference.md) for a full list of configuration
settings and options.

## How to add new Secret Stores?

Secret stores are pluggable, but for them to maintain good performance and simplicity the code that orchestrates them must
be added to the secret_store folder within Gofer(which means they have to be written in Rust).
