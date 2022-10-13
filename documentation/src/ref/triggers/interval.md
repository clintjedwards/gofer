# Interval <small>Trigger</small>

## Pipeline Configuration

- `every` [string]: Specifies the time duration between events. Unless changed via the trigger configuration, the minimum for this is 5 mins.

```go
...
WithTriggers(
    *sdk.NewTrigger("interval", "every_five_mins").WithSetting("every", "5m"),
)
...
```

## Trigger Configuration

Trigger configurations are set upon trigger startup and cannot be changed afterwards.

| EnvVar       | Default | Description                                               |
| ------------ | ------- | --------------------------------------------------------- |
| MIN_DURATION | "5m"    | The minimum duration users can set their pipelines to run |
