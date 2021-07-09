## gofer pipeline create

Create a new pipeline

### Synopsis

Create a new pipeline.

Creating a Pipeline requires a "pipeline config file". You can find documentation on how create a pipeline configuration
file [here](https://clintjedwards.com/gofer/docs/getting-started/first-steps/generate-pipeline-config).

Gofer can accept a configuration file from your local machine or checked into a repository.

Pipeline configuration files can be a single file or broken up into multiple files. Pointing the create command
at a single file or folder will both work.

Remote configuration file syntax is based off hashicorp's go-getter syntax(https://github.com/hashicorp/go-getter#protocol-specific-options).
Allowing the user to use many remote protocols, authentication schemes, and pass in options.

```
gofer pipeline create <url|path> [flags]
```

### Examples

```
$ gofer pipeline create github.com/clintjedwards/gofer.git//gofer
$ gofer pipeline create somefile.hcl
$ gofer pipeline create ./gofer/test.hcl
```

### Options

```
  -h, --help   help for create
```

### Options inherited from parent commands

```
      --config string      configuration file path
      --detail             show extra detail for some commands (ex. Exact time instead of humanized)
      --format string      output format; accepted values are 'pretty', 'json', 'silent'
      --host string        specify the URL of the server to communicate to
      --namespace string   specify which namespace the command should be run against
      --no-color           disable color output
```

### SEE ALSO

- [gofer pipeline](gofer_pipeline.md) - Manage pipelines
