---
id: overview
title: Overview
sidebar_position: 1
---

# Cron <small>Trigger</small>

Cron allows users to schedule events on long term intervals and specific days.

It uses a stripped down version of the cron syntax to do so:

    Field           Allowed values  Allowed special characters

    Minutes         0-59            * , -
    Hours           0-23            * , -
    Day of month    1-31            * , -
    Month           1-12            * , -
    Day of week     0-6             * , -
    Year            1970-2100       * , -

## Pipeline Configuration

- `expression` [string]: Specifies the cron expression of the interval desired.

### Every year on Xmas

```hcl
trigger "cron" "yearly_on_xmas" {
    expression = "0 1 25 12 * *"
}
```

## Trigger Configuration

None
