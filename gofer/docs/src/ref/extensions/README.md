# Extensions

Extensions are Gofer's way of adding additional functionality to pipelines. You can subscribe your pipeline to an
extension, allowing that extension to give your pipeline extra powers.

The most straight-forward example of this is the interval extension. It lets your pipeline run every time some amount
of time has passed. Say you have a pipeline that needs to run every 5 minutes; you'd subscribe it to the
[interval](./provided/interval.md) extension with an interval of `5m`:

```bash
gofer pipeline subscribe simple interval every_five_mins -s every="5m"
```

On startup, Gofer launches the interval extension as a long-running container. When your pipeline subscribes to it,
the interval extension starts a timer, and when 5 minutes have passed it sends an API request to Gofer, causing Gofer
to run your pipeline.

## Gofer's extensions

Gofer comes with [its own set of extensions](./provided/index.html), a bit like a standard library. They're all
maintained and released alongside Gofer, and they come in two kinds:

- **Default extensions**, [cron](./provided/cron.md) and [interval](./provided/interval.md), are basic enough that
  everyone gets them. They're installed unless you turn them off, and their manifests are built into Gofer.
- **Optional extensions**, like [github](./provided/github.md), are for things not everyone needs. You add them to
  your config when you want them, the same way you'd install any other extension.

You can also [write your own](./writing_extensions.md).

## Installing and configuring extensions

Extensions are installed and configured in one place: Gofer's server config. There's no separate install command;
you describe the extensions you want and Gofer makes it so.

Installing an extension takes four steps:

1. **Put the extension's secrets in Gofer's global secret store.** Extensions that talk to other services usually
   need credentials. Those never go in the config file itself.

   ```bash
   # Prompts for the value so it stays out of your shell history. You can also pipe in a file.
   gofer secret global put github-app-key < my-github-app.private-key.pem
   ```

2. **Add an `[[extensions.install]]` entry to Gofer's config** that points at the extension's
   [manifest](./manifest.md). The manifest is a small file that ships with each extension and tells Gofer everything
   about it: which container image to run, which settings it takes, and which parameters pipelines pass when they
   subscribe.

3. **Fill in the extension's settings.** Secrets are written as a reference to the global secret store, like
   `global_secret{{github-app-key}}`, instead of the value itself.

   ```toml
   [[extensions.install]]
   id = "github"
   manifest = "https://raw.githubusercontent.com/clintjedwards/gofer/v<gofer version>/containers/extensions/github/manifest.toml"
   [extensions.install.settings]
   app_id = "112348"
   app_installation = "99560091"
   app_key = "global_secret{{github-app-key}}"
   app_webhook_secret = "global_secret{{github-app-webhook-secret}}"
   ```

   You don't have to write this by hand. `gofer extension manifest <manifest URL or path>` prints a ready-to-paste
   block listing every setting the extension takes, with its documentation, plus the commands to create any secrets
   it needs.

4. **Reload.** Either restart Gofer, or run `gofer extension reload` to apply the change to the running server. See
   [Reloading](#reloading) below.

### The `[[extensions.install]]` entry

| name               | type   | default    | description                                                                                                                                                                                                   |
| ------------------ | ------ | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `id`               | string | `<required>` | A unique name for the extension. Pipelines subscribe to the extension by this id, so keep it the same across upgrades. Letters, numbers, and hyphens; 3 to 32 characters.                                    |
| `manifest`         | string | none       | Where to find the extension's [manifest](./manifest.md): an `https://` URL or a path on the Gofer server. Only Gofer's [default extensions](#default-extensions) (cron, interval) can leave this out; they use the manifest built into Gofer. |
| `enabled`          | bool   | `true`     | Set to `false` to stop the extension without removing it. Its subscriptions are kept.                                                                                                                         |
| `settings`         | table  | empty      | Values for the settings the extension takes, as listed in its manifest. Unknown keys are an error. Settings marked secret must be a `global_secret{{key}}` reference.                                         |
| `additional_roles` | list   | empty      | Extra [roles](../server_configuration/authz_n.md) to give the extension's API token, on top of the role every extension gets.                                                                                 |
| `registry_auth`    | table  | none       | `{ user = "...", pass = "global_secret{{...}}" }` for pulling the extension's image from a private registry. The password must be a global secret reference.                                                |

### Secrets

Any setting can be a global secret reference, and settings the extension marks as secret must be one. Gofer won't
accept a raw value for them. Gofer swaps references for the real values only when it starts the extension's
container, so the actual secret never appears in Gofer's config, its database, or `gofer extension get` output.

Global secrets normally have a list of namespaces that are allowed to use them. That list doesn't apply to extensions,
since extensions don't belong to a namespace. Only admins can configure extensions or manage global secrets, so an
extension can use any global secret.

### Default extensions

Every Gofer install gets the [cron](./provided/cron.md) and [interval](./provided/interval.md) extensions without
listing them. Their manifests are built into Gofer, so they always match the version of Gofer you're running and starting them never depends on downloading anything. Gofer serves them at
`/extensions/manifests/<name>.toml` if you want to read one.

To change one of them, add an entry with the same id. Anything you set replaces the default:

```toml
# Use a different minimum interval.
[[extensions.install]]
id = "interval"
[extensions.install.settings]
min_interval = "5m"

# Don't run cron at all.
[[extensions.install]]
id = "cron"
enabled = false
```

## Reloading

Restarting Gofer brings every extension in line with the config. Gofer starts what's new, restarts what changed, and
stops what was disabled or removed. An extension that can't start (a typo in its settings, a missing secret, a bad
image) doesn't stop Gofer from starting; it's marked as failed and `gofer extension list` shows why.

If you'd rather not restart Gofer, `gofer extension reload` does the same thing step by step:

1. Gofer rereads its config file and shows what would change for each extension: what's being installed, upgraded, or
   stopped, which settings changed, and which secrets were rotated. Nothing has changed yet at this point.
   `gofer extension reload --dry-run` stops here.
2. You confirm each extension one at a time, or skip it. `--yes` applies everything without asking.
3. If an extension's new version won't start, you're offered the version that was running before. With `--yes`
   that happens automatically. The extension then shows up in the next reload again until its config is fixed.
4. You get a summary of what happened to each extension.

```text
$ gofer extension reload
  slack (update)
  │ manifest: https://example.com/slack/1.3.0/manifest.toml
  │ ~ manifest: https://example.com/slack/1.2.0/manifest.toml -> https://example.com/slack/1.3.0/manifest.toml
  │ ~ image: ghcr.io/example/slack:1.2.0 -> ghcr.io/example/slack:1.3.0
  │ ~ settings.token: global_secret{{slack-token}} -> global_secret{{slack-token}} (secret value changed)

1 change(s) to apply, 2 extension(s) unchanged.
? update 'slack'? [y]es / [s]kip / [q]uit:
```

A few things to know:

- **Applying a change restarts the extension.** Gofer stops the old container before starting the new one, so the
  extension is briefly unavailable. Events it would have handled in that window can be missed; for example, GitHub
  doesn't resend webhooks that fail. Running both at once isn't an option, since two copies would both act on the
  same pipeline subscriptions.
- **Entries with problems are left alone.** If an entry's manifest can't be fetched, a required setting is missing,
  or a secret doesn't exist, reload reports it and skips it. Whatever was running keeps running with its old config.
- **Reload checks that nothing changed after you looked.** If the config, a manifest, or a secret changes between
  the summary and your confirmation, Gofer refuses to apply it and asks you to run reload again.
- **Only the extensions config is reloaded.** Changes to other parts of Gofer's config still need a restart. Reload
  rereads the config file Gofer was started with (`/etc/gofer/gofer_web.toml`, or the path given to
  `gofer service start --config`). Environment variables can't change while Gofer is running, so settings that come
  from them stay as they were at startup.

## Removing an extension

Removing an extension's entry from the config and reloading stops the extension, but keeps its pipeline
subscriptions and stored objects. If you add the entry back, it picks up where it left off.

To delete an extension for good, purge it once it's out of the config:

```bash
gofer extension purge github
```

Purge shows how many pipelines will lose their subscription and asks before deleting anything.

## Extension states

`gofer extension list` and `gofer extension get <id>` show two things about each extension.

The **status** says what the config wants:

| status         | meaning                                                                    |
| -------------- | -------------------------------------------------------------------------- |
| `enabled`      | In the config and should be running.                                       |
| `disabled`     | In the config with `enabled = false`.                                      |
| `unconfigured` | Removed from the config. Its data is kept until it's purged.               |

The **state** says what's actually happening:

| state     | meaning                                                                                  |
| --------- | ---------------------------------------------------------------------------------------- |
| `running` | Up and answering Gofer.                                                                  |
| `stopped` | Not running on purpose, because it's disabled or unconfigured.                           |
| `failed`  | Gofer couldn't start it. The reason column says why; `gofer extension logs <id>` can help. |

## Upgrading

See [Upgrading Extensions](./upgrading.md).

## How do extension settings reach the extension?

Gofer passes everything to the extension's container as environment variables:

- `GOFER_EXTENSION_SYSTEM_*` variables are set by Gofer itself: the extension's id, its API token, where to reach
  Gofer, TLS settings, and so on.
- `GOFER_EXTENSION_CONFIG_<KEY>` variables carry the settings from the config entry, with the key uppercased. The
  `app_key` setting arrives as `GOFER_EXTENSION_CONFIG_APP_KEY`. The SDK's `GetConfigFromEnv("app_key")` reads it for
  you.

Pipeline subscription parameters aren't environment variables; they're sent to the extension when a pipeline
subscribes.
