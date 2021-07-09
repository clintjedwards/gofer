## gofer pipeline update

Update pipeline

### Synopsis

Update pipeline via pipeline configuration file.

Warning! Updating a pipeline requires disabling that pipeline and pausing all trigger events.
This may cause those events while the pipeline is being upgraded to be discarded.

Updating a Pipeline requires a "pipeline config file". You can find documentation on how create a pipeline configuration
file here: https://clintjedwards.com/gofer/docs/pipeline-configuration/overview

Gofer can accept a configuration file from your local machine or checked into a repository.

Pipeline configuration files can be a single file or broken up into multiple files. Pointing the create command
at a single file or folder will both work.

Remote configuration file syntax is based off hashicorp's go-getter syntax(https://github.com/hashicorp/go-getter#protocol-specific-options).
Allowing the user to use many remote protocols and pass in options.

```
gofer pipeline update <id> <url|file> [flags]
```

### Examples

```
$ gofer pipeline update aup3gq github.com/clintjedwards/gofer.git//gofer
$ gofer pipeline update simple_test_pipeline somefile.hcl
$ gofer pipeline update simple_test_pipeline ./gofer/test.hcl
```

### Options

```
  -f, --force           Stop all runs and update pipeline immediately
  -g, --graceful-stop   Stop all runs gracefully; sends a SIGTERM to all task runs for all in-progress runs and then waits for them to stop.
  -h, --help            help for update
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
