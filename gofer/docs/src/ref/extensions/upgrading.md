# Upgrading Extensions

An extension's version comes from its [manifest](./manifest.md), which pins an exact image. Upgrading means pointing
the extension at a newer manifest and reloading.

## Upgrading an extension

1. Change the `manifest` in the extension's `[[extensions.install]]` entry to the new version's manifest.
2. Check the new version's manifest or docs for settings that were added, removed, or renamed. If a new required
   setting is missing, the reload will tell you which one.
3. Run `gofer extension reload`. It shows the image change and any setting changes before anything happens, then
   restarts the extension once you confirm.
4. If the new version won't start, accept the offer to put the previous version back. The extension keeps running
   the old version until you fix the config and reload again.

The extension is briefly unavailable while it restarts. Events it would have handled during that window can be
missed; GitHub, for example, doesn't resend webhooks that fail. Plan upgrades of webhook driven extensions for a quiet
moment.

## Default extensions

The manifests for Gofer's default extensions (cron and interval) are built into Gofer, so upgrading Gofer upgrades
them: the next restart, or `gofer extension reload`, moves them to the versions released with the new Gofer. If you've
pointed one at your own manifest in the config, Gofer leaves that alone.

Gofer's optional extensions, like github, upgrade like any other extension: change the version in their manifest URL
to match your new Gofer and reload.

## Compatibility

Gofer and its extensions follow one rule: **within a major version of Gofer, extensions stay backwards compatible.**

- An extension has to keep reading the data it stored with earlier versions (its objects in Gofer's object store,
  for example). Upgrading it should never require the operator to migrate anything by hand.
- Settings shouldn't disappear or change meaning. A new setting should either be optional or come with a default.
- Breaking changes wait for Gofer's next major version, and the extension's docs should spell out the upgrade path
  from each earlier version.

Before Gofer 1.0, minor versions count as major ones; Gofer 0.11 can make breaking changes from Gofer 0.10.

### Notes for extension authors

The simplest way to stay compatible is to keep reading old formats rather than rewriting them. If you change how you
store something, read both the old and new format and write the new one. An extension that converts old data the
first time it starts works too, but then a revert to the previous version can't read the converted data, so prefer
reading both whenever you can.
