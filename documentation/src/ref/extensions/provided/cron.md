# Cron <small>Extension</small>

Cron allows users to schedule pipeline runs on long term intervals and specific days.

It uses a stripped down version of the cron syntax to do so:

    Field           Allowed values  Allowed special characters

    Minutes         0-59            * , -
    Hours           0-23            * , -
    Day of month    1-31            * , -
    Month           1-12            * , -
    Day of week     0-6             * , -
    Year            1970-2100       * , -

---

```
┌───────────── minute (0 - 59)
│ ┌───────────── hour (0 - 23)
│ │ ┌───────────── day of the month (1 - 31)
│ │ │ ┌───────────── month (1 - 12)
│ │ │ │ ┌───────────── day of the week (0 - 6) (Sunday to Saturday)
│ │ │ │ │ ┌───────────── Year (1970-2100)
│ │ │ │ │ │
│ │ │ │ │ │
│ │ │ │ │ │
* * * * * *
```

## Pipeline Configuration

- `expression` <string>: Specifies the cron expression of the interval desired.

### Every year on Xmas

```go
...
WithExtensions(
    *sdk.NewExtension("cron", "yearly_on_xmas").WithSetting("expression", "0 1 25 12 * *"),
)
...
```

## Extension Configuration

None
