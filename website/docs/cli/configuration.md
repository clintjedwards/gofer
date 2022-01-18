# Configuration

The Gofer CLI accepts configuration through flags, environment variables, or configuration file.

When multiple configuration sources are used the hierarchy is (from lowest to highest) config file values -> environment variables -> flags. Meaning that if you give the same configurations different values through a configuration file and through flags, the value given in the flag will prevail.

## Flags

You can view the global flags in the [cli overview](gofer.md#global-options) or by simply typing `gofer -h`.

## Environment variables

You can also set configuration values through environment variables. Each environment variable has a prefix
of `GOFER_CLI_`.

For example, setting your API token:

```bash
export GOFER_CLI_TOKEN=mysupersecrettoken
gofer service token whoami
```

The most up to date list of possible environment variables [exists here](https://github.com/clintjedwards/gofer/blob/main/internal/config/cli.go#L11)

:::note

Remember that any environment variable found within the above struct is prefixed with `GOFER_CLI_`. So for example to set a new `host` variable you would write:

```bash
export GOFER_CLI_HOST=localhost:8080
```

:::

## Configuration file

For convenience reasons Gofer can also use a standard configuration file. The language of this file is [HCL](https://octopus.com/blog/introduction-to-hcl-and-hcl-tooling). Most of the options are simply in the form of `key=value`.

### Configuration file locations

You can put your CLI configuration file in any of the following locations and Gofer will automatically detect and read from it(in order of first searched):

1. The path given to the `--config` flag
2. $HOME/.gofer.hcl
3. $HOME/.config/gofer.hcl

### Configuration file options

The options available in the configuration file are the same as possible environment variables. To find the most up to date values you can use [the code here](https://github.com/clintjedwards/gofer/blob/main/internal/config/cli.go#L11). For convenience, a possible out of date list and explanation is listed below:

| configuration | type   | description                                                                                                                          |
| ------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------ |
| namespace     | string | The namespace ID of the namespace you'd like to default to. This is used to target specific namespaces when there might be multiple. |
| format        | string | Can be one of three values: `pretty`, `json`, `silent`. Controls the output of CLI commands.                                         |
| host          | string | The URL of the Gofer server; used to point the CLI and that correct host.                                                            |
| no_color      | bool   | Turns off color globally for all CLI commands.                                                                                       |
| token         | string | The authentication token passed Gofer for Ident and Auth purposes.                                                                   |

### Example configuration file

```hcl
// /home/clintjedwards/.gofer.hcl
namespace = "myNamespace"
format    = "pretty"
host      = "localhost:8080"
no_color  = false
token     = "mysupersecrettoken"
```
