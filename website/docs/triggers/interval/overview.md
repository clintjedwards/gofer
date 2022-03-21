---
id: overview
title: Overview
sidebar_position: 1
---

# Interval <small>Trigger</small>

## Pipeline Configuration

- `every` [string]: Specifies the time duration between events. Unless changed via the trigger configuration, the minimum for this is 5 mins.

```hcl
trigger "interval" "every_five_mins" {
    every = "5m"
}
```

## Trigger Configuration

Trigger configurations are set upon trigger startup and cannot be changed afterwards. They are set via the [server configuration](../../server-configuration/overview).

| EnvVar       | Default | Description                                               |
| ------------ | ------- | --------------------------------------------------------- |
| MIN_DURATION | "5m"    | The minimum duration users can set their pipelines to run |
