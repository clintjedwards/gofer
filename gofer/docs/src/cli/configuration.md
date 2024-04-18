# Configuration

The Gofer CLI accepts configuration through flags, environment variables, or a configuration file.

When multiple configuration sources are used the hierarchy is (from lowest to highest)
config file values -> environment variables -> flags. Meaning that if you give the same configurations different
values through a configuration file and through flags, the value given in the flag will prevail.

## Flags

You can view Gofer's global flags by simply typing `gofer -h`.

## Environment variables

You can also set configuration values through environment variables. Each environment variable has a prefix
of `GOFER_`.

For example, setting your API token:

```bash
export GOFER_TOKEN=mysupersecrettoken
gofer service token whoami
```

Each environment variable available is just the flag with a prefix of `GOFER_`.

```bash
export GOFER_HOST=localhost:8080
```

## Configuration file

For convenience reasons Gofer can also use a standard configuration file. The language of this file is
[TOML](https://toml.io/en/). Most of the options are simply in the form of `key=value`.

### Configuration file locations

You can put your CLI configuration file in any of the following locations and Gofer will automatically
detect and read from it(in order of first searched):

1. The path given to the `--config` flag
2. $HOME/.gofer.toml
3. $HOME/.config/gofer.toml

### Configuration file options

The options available in the configuration file are the same as the global flags:

```bash
gofer -h

...
Flags:
   --detail
...

# The flag 'detail' maps back to the configuration file as the same name

# gofer.toml
detail = false
```

| configuration | type   | description                                                                                                                          |
| ------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------ |
| namespace     | string | The namespace ID of the namespace you'd like to default to. This is used to target specific namespaces when there might be multiple. |
| detail        | string | Show extra detail for some commands (ex. Exact time instead of humanized)                                                            |
| output_format | string | Can be one of three values: `plain`, `silent`, `json`. Controls the output of CLI commands.                                          |
| api_base_url  | string | The URL of the Gofer server; used to point the CLI and that correct host.                                                            |
| token         | string | The authentication token passed Gofer for Ident and Auth purposes.                                                                   |
| debug         | bool   | Print debug statements.                                                                                                              |

### Example configuration file

```toml
# /home/clintjedwards/.gofer.toml
api_base_url  = "http://127.0.0.1:8080/"
debug         = false
detail        = false
namespace     = "default"
output_format = "plain"
token         = "mysupersecrettoken"
```
