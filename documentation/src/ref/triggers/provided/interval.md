# Interval <small>Trigger</small>

Interval simply triggers the subscribed pipeline at the given interval.

## Parameters/Pipeline Configuration

- `every` <string>: Specifies the time duration between events. Unless changed via the trigger configuration, the minimum for this is 5 mins.

```go
...
WithTriggers(
    *sdk.NewTrigger("interval", "every_five_mins").WithSetting("every", "5m"),
)
...
```

## Trigger Configuration

Trigger configurations are set upon trigger startup and cannot be changed afterwards without restarting said trigger.

| EnvVar       | Default | Description                                               |
| ------------ | ------- | --------------------------------------------------------- |
| MIN_DURATION | "5m"    | The minimum duration users can set their pipelines to run |
