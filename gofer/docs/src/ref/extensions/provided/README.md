# Gofer's Extensions

Gofer comes with a set of extensions, a bit like a standard library. They're all maintained and released alongside
Gofer and share its version number. Some are basic enough that every install gets them; the rest are there when you
need them.

Each one's [manifest](../manifest.md) and image are built into Gofer. Gofer stores the images from its binary when it
starts, as `gofer://<name>:<gofer version>`, so they always match the version of Gofer you're running and starting them
never downloads anything. Upgrading Gofer upgrades them on its next start. Gofer serves the manifests at
`/extensions/manifests/<name>.toml` (for example `http://localhost:8080/extensions/manifests/cron.toml`) so you can read
exactly what it runs.

## Default extensions

These are always available: Gofer installs them unless the config turns them off with `enabled = false`.

| name                      | description                                                                                                       |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| [interval](./interval.md) | Interval triggers a run after a predetermined amount of time has passed.                                          |
| [cron](./cron.md)         | Cron is used for longer termed, more nuanced intervals. For instance, running a pipeline every year on Christmas. |

## Optional extensions

These are just as supported as the default ones, but not everyone needs them, so they aren't installed until you list
their id in your config. They don't need a `manifest` either.

| name                  | description                                                            |
| --------------------- | ---------------------------------------------------------------------- |
| [github](./github.md) | Allow your pipelines to run based on branch, tag, or release activity. |

```toml
[[extensions.install]]
id = "github"
[extensions.install.settings]
# ...
```

`gofer extension manifest github` prints a block like that with every setting github takes.
