# Provided Extensions

Gofer provides some pre-written extensions for quick use:

| name                      | image                                                     | included by default | description                                                                                                       |
| ------------------------- | --------------------------------------------------------- | ------------------- | ----------------------------------------------------------------------------------------------------------------- |
| [interval](./interval.md) | ghcr.io/clintjedwards/gofer/extensions/interval:\<version\> | yes                 | Interval triggers a run after a predetermined amount of time has passed.                                          |
| [cron](./cron.md)         | ghcr.io/clintjedwards/gofer/extensions/cron:\<version\>     | yes                 | Cron is used for longer termed, more nuanced intervals. For instance, running a pipeline every year on Christmas. |
| [github](./github.md)     | ghcr.io/clintjedwards/gofer/extensions/github:\<version\>   | no                  | Allow your pipelines to run based on branch, tag, or release activity.                                            |

## Versions

Each extension release is published under its full version (`0.10.1`) and also under floating tags that always point
at the newest release in that line (`0.10`, `0`, and `latest`).

An extension is compatible with Gofer when the part of the version that signals breaking changes matches. Before 1.0
that's the minor version, so Gofer 0.10.x works with any 0.10.x extension. From 1.0 on it's the major version, so Gofer
1.4.2 works with any 1.x.y extension.

The extensions Gofer installs by default use the floating tag for the running version of Gofer (`cron:0.10` for Gofer
0.10.x). Gofer pulls the image every time it starts, so new compatible releases are picked up on restart. When you
upgrade Gofer to a new breaking version, it moves those default extensions to the matching tag on startup.

If you'd rather control the version yourself, install the extension with a full version tag such as `cron:0.10.1`.
Gofer won't change an extension installed with a full version tag or an image from somewhere else.

When installing an extension that isn't included by default, like github, use the floating tag that matches your Gofer
version (e.g. `github:0.10`).
