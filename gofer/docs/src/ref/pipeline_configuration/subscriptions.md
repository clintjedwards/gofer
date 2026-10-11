# Running Pipelines Automatically

A pipeline config describes what to run, but not when. Pipelines only run when someone starts them, unless they're
subscribed to an [extension](../extensions/index.html). Extensions watch for something to happen, like a time passing
or a push to a Github repository, and start a run when it does.

Subscriptions aren't part of the pipeline config. You add them with the CLI after the pipeline exists, and they stay
in place when you register new versions of the pipeline with `gofer up`.

## Subscribing

```bash
gofer pipeline subscribe <pipeline_id> <extension_id> <label> -s key=value
```

- `extension_id` is the extension to subscribe to. `gofer extension list` shows the ones your Gofer has.
- `label` is a name you pick for this subscription. A pipeline can subscribe to the same extension more than once,
  for example to run every hour and also every Monday, and the label tells them apart.
- `-s key=value` sets the subscription's parameters. Each extension has its own, listed on its page; repeat `-s` for
  more than one.

For example, to run the `simple` pipeline every five minutes with the [interval](../extensions/provided/interval.md)
extension:

```bash
gofer pipeline subscribe simple interval every_five_mins -s every="5m"
```

Or every Christmas at 1am with [cron](../extensions/provided/cron.md):

```bash
gofer pipeline subscribe simple cron yearly_on_xmas -s expression="0 1 25 12 * *"
```

Parameters can be [secrets](../secret_store/index.html) too, using the same `pipeline_secret{{key}}` or
`global_secret{{key}}` strings as task variables. Gofer swaps in the real value when it sends the subscription to the
extension.

## Seeing and removing subscriptions

`gofer fetch <pipeline_id>` lists a pipeline's subscriptions. To remove one:

```bash
gofer pipeline unsubscribe simple interval every_five_mins
```

## What the extension passes to your run

When an extension starts a run it can pass in run variables, which your tasks receive as environment variables. The
github extension, for example, passes in details about the event that triggered it. Each extension's page lists what
it passes, and `gofer task get <pipeline_id> <run_id> <task_id>` shows exactly what a task received.

If an extension is stopped, disabled, or failed to start, its subscriptions stay in place but nothing triggers them.
If a pipeline that should be running on its own isn't, check `gofer extension list`.
