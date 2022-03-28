---
id: overview
title: Overview
sidebar_position: 1
---

# Log <small>Notifier</small>

## Pipeline Configuration

- `include_timestamp` [boolean]: Specifies whether to print the results in log format for simple plaintext.

```hcl
notifier "log" "logger" {
    include_timestamp = "false"
}
```

## Notifier Configuration

Notifier configurations are set upon startup and cannot be changed afterwards. They can be set via the [server configuration](../../server-configuration/overview) or the cli installation command.

| EnvVar   | Default | Description                                                     |
| -------- | ------- | --------------------------------------------------------------- |
| TEST_VAR | ""      | No-op; this is purely a development variable used to test with. |
