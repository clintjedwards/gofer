# Configuration

The Gofer CLI accepts configuration through a configuration file or environment variables.

When both are used, environment variables win. So if your config file sets `namespace = "default"` and you
`export GOFER_NAMESPACE=ops`, the CLI uses `ops`. Some commands also take flags like `--namespace` that override
both for that one command.

## Environment variables

Each configuration option can be set as an environment variable by upper casing it and adding a `GOFER_` prefix.

For example, setting your API token:

```bash
export GOFER_TOKEN=mysupersecrettoken
gofer token whoami
```

Or pointing the CLI at a different server:

```bash
export GOFER_API_BASE_URL=http://localhost:8080
```

## Configuration file

For convenience reasons Gofer can also use a standard configuration file. The language of this file is
[TOML](https://toml.io/en/). Each option is just `key = value`.

### Configuration file locations

Gofer reads the first of these it finds:

1. `$HOME/.gofer.toml`
2. `$HOME/.config/gofer.toml`

Development builds of Gofer read `$HOME/.gofer_dev.toml` and `$HOME/.config/gofer_dev.toml` instead, so you can
work on Gofer without touching the config you use for your real server.

### Configuration file options

| configuration            | type   | default                 | description                                                                                                                                                      |
| ------------------------ | ------ | ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| api_base_url             | string | `http://127.0.0.1:8080` | The URL of the Gofer server.                                                                                                                                     |
| token                    | string |                         | The API token the CLI sends with each request.                                                                                                                   |
| namespace                | string | `default`               | The namespace commands work in unless you pass `--namespace`.                                                                                                    |
| detail                   | bool   | `false`                 | Show extra detail for some commands (ex. exact time instead of humanized).                                                                                       |
| output_format            | string | `spinner`               | Can be one of `spinner`, `plain`, `silent`, `json`. Controls the output of CLI commands. `spinner` falls back to `plain` when output isn't a terminal.            |
| debug                    | bool   | `false`                 | Print debug statements.                                                                                                                                          |

### Example configuration file

```toml
# ~/.gofer.toml
api_base_url  = "https://gofer.example.com"
namespace     = "default"
output_format = "spinner"
token         = "mysupersecrettoken"
```
