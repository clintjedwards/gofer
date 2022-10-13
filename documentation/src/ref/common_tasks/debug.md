# Debug <small>Common Task</small>

## Pipeline Configuration

```go
...
WithTasks(sdk.NewCommonTask("debug", "debug_task"))
...
```

## Common Task Configuration

Common Task configurations are set upon common task startup and cannot be changed afterwards. They are set via the [server configuration](../../server-configuration/overview).

| EnvVar | Default | Description                                                        |
| ------ | ------- | ------------------------------------------------------------------ |
| FILTER | false   | Don't print any env vars that contain the strings "key" or "token" |
