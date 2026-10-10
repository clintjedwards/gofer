# Interval <small>Extension</small>

Interval simply runs the subscribed pipeline at the given time interval continously.

## Parameters/Pipeline Configuration

- `every` \<string\>: Specifies the time duration between events. Unless changed via the extension configuration, the minimum for this is 1 minute.

```bash
gofer pipeline subscribe simple interval every_five_mins -s every="5m"
```

## Extension Configuration

Interval is installed by default. Its settings go under an `[[extensions.install]]` entry with `id = "interval"` in
Gofer's config; run `gofer extension reload` after changing them.

| Key          | Default | Description                                                                                                                                         |
| ------------ | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| min_interval | "1m"    | The minimum interval pipelines can set for `every`. Supports Go duration strings: https://pkg.go.dev/time#ParseDuration. Examples: '1m', '60s', '3h'. |

### Example

```toml
[[extensions.install]]
id = "interval"
[extensions.install.settings]
min_interval = "5m"
```
