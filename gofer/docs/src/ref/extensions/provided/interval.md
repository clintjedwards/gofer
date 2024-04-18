# Interval <small>Extension</small>

Interval simply runs the subscribed pipeline at the given time interval continously.

## Parameters/Pipeline Configuration

- `every` <string>: Specifies the time duration between events. Unless changed via the extension configuration, the minimum for this is 5 mins.

```bash
gofer pipeline subscribe simple interval every_five_mins -s every="5m"
```

## Extension Configuration

Extension configurations are set upon extension startup and cannot be changed afterwards without restarting said extension.

| EnvVar       | Default | Description                                               |
| ------------ | ------- | --------------------------------------------------------- |
| MIN_DURATION | "5m"    | The minimum duration users can set their pipelines to run |
