---
id: trigger-stanza
title: Trigger stanza
sidebar_position: 1
---

# Trigger <small>_Stanza_</small>

The Trigger stanza is the way in which you can automate your pipeline to automatically run based on some other event. That event can be anything! For example the [interval](../../triggers/interval/overview) Trigger allows you to run your pipeline simply via the passage of time while the [github](../../triggers/github/overview) Trigger allows your pipeline to run via external github events.

## Trigger Parameters

| Param   | Type                                     | Description                                                                                                                                                                                                                                                                                                                                                                          |
| ------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [label] | `string: <required>`                     | The ID of the trigger you would like to use. You can list all triggers that your gofer instance supports by using the command line: `gofer trigger list`.                                                                                                                                                                                                                            |
| [label] | `string: <required>`                     | The name of your trigger. This string cannot have any spaces or special characters and is limited to 70 characters. This name is purely so you can differentiate your triggers from one another. Use it to provide a short description on what this trigger should be doing. For example: an interval trigger which runs on a five minute interval might be called `every_five_min`. |
| config  | `string key -> string value: <optional>` | Each trigger Kind has specific configuration values which can then be used with each trigger stanza. View the documentation for the trigger kind you would like to use to understand which values can be passed to each stanza.                                                                                                                                                      |

## Trigger Examples

### Simple [Interval](../../triggers/interval/overview) trigger

```hcl
// Triggers are plugins that control the automatic execution of pipeline.
// They typically take some kind of configuration which controls the behavior of the trigger.
// The name here "interval" denotes the "kind" of trigger. Gofer supports multiple trigger kinds.
// A list of trigger kinds can be found in the documentation or via the command line:
// `gofer trigger list`
trigger "interval" "every_one_minute" {
    every = "1m"
}
```

### Multiple of the same trigger kind

```hcl
trigger "interval" "every_one_minute" {
    every = "1m"
}

trigger "interval" "every_two_minutes" {
    every = "2m"
}
```

### Multiple triggers of different kinds

```hcl
trigger "interval" "every_one_minute" {
    every = "1m"
}

trigger "cron" "every_single_minute" {
    expression = "* * * * * *"
}
```

### Trigger with secret

```hcl
trigger "github" "every_commit" {
    key = "secret{{my_github_key}}"
}
```
