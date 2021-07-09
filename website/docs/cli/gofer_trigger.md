## gofer trigger

Get details about Gofer triggers

### Synopsis

Get details about Gofer triggers.

Triggers act as plugins for Gofer that execute a run for a pipeline based on some criteria.

An example of a trigger might be the simply the passing of time for the "interval" trigger. A user will _subscribe_ to
this trigger in their pipeline configuration file and based on settings used in that file interval will alert Gofer
when the user's intended interval of time has passed. This automatically then kicks off a new instance of a run for
that specific pipeline.

### Options

```
  -h, --help   help for trigger
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

- [gofer](gofer.md) - Gofer is a distributed, continuous thing do-er.
- [gofer trigger get](gofer_trigger_get.md) - Get a specific trigger by name.
- [gofer trigger list](gofer_trigger_list.md) - List all triggers
