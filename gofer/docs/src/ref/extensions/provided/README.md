# Gofer's Extensions

Gofer comes with a set of extensions, a bit like a standard library. They're all maintained and released alongside
Gofer and share its version number. Some are basic enough that every install gets them; the rest are there when you
need them.

## Default extensions

These are always available: Gofer installs them unless the config turns them off with `enabled = false`.

| name                      | image                                                       | description                                                                                                       |
| ------------------------- | ----------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| [interval](./interval.md) | ghcr.io/clintjedwards/gofer/extensions/interval:\<version\> | Interval triggers a run after a predetermined amount of time has passed.                                          |
| [cron](./cron.md)         | ghcr.io/clintjedwards/gofer/extensions/cron:\<version\>     | Cron is used for longer termed, more nuanced intervals. For instance, running a pipeline every year on Christmas. |

Their [manifests](../manifest.md) are built into Gofer, so they don't need a `manifest` in the config, always match
the version of Gofer you're running, and never depend on downloading anything to start. Gofer serves them at
`/extensions/manifests/<name>.toml` (for example `http://localhost:8080/extensions/manifests/cron.toml`) so you can
read exactly what it runs. Upgrading Gofer upgrades them on the next restart or `gofer extension reload`.

If you need a different version of one, set `manifest` in its `[[extensions.install]]` entry to your own copy and
Gofer uses that instead.

## Optional extensions

These are just as supported as the default ones, but not everyone needs them, so they aren't installed until you add
them to your config. You install them with a manifest URL, the same way as any other extension.

| name                  | image                                                     | description                                                            |
| --------------------- | --------------------------------------------------------- | ---------------------------------------------------------------------- |
| [github](./github.md) | ghcr.io/clintjedwards/gofer/extensions/github:\<version\> | Allow your pipelines to run based on branch, tag, or release activity. |

Each one keeps its manifest in the Gofer repository, next to its code, and every Gofer release tag has a copy pinned
to the images built for that release:

```text
https://raw.githubusercontent.com/clintjedwards/gofer/v<gofer version>/containers/extensions/<name>/manifest.toml
```

They're released alongside Gofer and share its version number, so use the manifest from the same release as your
Gofer server, and change the version in the URL when you upgrade Gofer. See [Upgrading Extensions](../upgrading.md).

Images are still published under floating tags (`0.11`, `0`, and `latest`) as well as the full version, but manifests
always pin the full version so that a reload only ever changes an extension when you change its manifest.
