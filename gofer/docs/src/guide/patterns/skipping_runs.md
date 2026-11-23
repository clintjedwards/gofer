# Skipping Runs

There are two ways to keep a pipeline from running. Disable it to turn it off from right now, for a while or for good.
List [skip dates](../../ref/pipeline_configuration/index.html#skipping-dates) in its config for days it should never
run, year after year.

## Give a pause an end date.

When you want a pipeline to stop for a while, tell Gofer when it should start again instead of trying to remember:

```bash
gofer pipeline disable simple --for 7d
gofer pipeline disable simple --until 2026-12-01
```

`--until` turns the pipeline back on at the start of that day in your local time. Until then nothing can start a run.
Runs that are already going finish normally, and runs still waiting in the queue are cancelled. Changed your mind?
`gofer pipeline enable simple` turns it back on early, and running `disable` again replaces the end date.

## Skip just the next run.

For a pipeline that runs once a day, `--for 24h` covers exactly one run no matter when you run it:

```bash
gofer pipeline disable simple --for 24h
```

For anything less regular, pick a time that's after the run you want to skip and before the one you want to keep.

## Skip US federal holidays.

Copy this into your pipeline config:

```go
SkipDates(
    "01/01",           // New Year's Day
    "01/15-01/21:mon", // Martin Luther King Jr. Day
    "02/15-02/21:mon", // Washington's Birthday
    "05/25-05/31:mon", // Memorial Day
    "06/19",           // Juneteenth
    "07/04",           // Independence Day
    "09/01-09/07:mon", // Labor Day
    "10/08-10/14:mon", // Columbus Day
    "11/11",           // Veterans Day
    "11/22-11/28:thu", // Thanksgiving
    "12/25",           // Christmas
)
```

Holidays with a fixed date are observed on the Friday before when they land on a Saturday, and the Monday after when
they land on a Sunday. If you get those days off, add these too. `07/03:fri` only matches in years where July 3rd is a
Friday, which are exactly the years the 4th is a Saturday.

```go
"12/31:fri", "01/02:mon", // New Year's Day
"06/18:fri", "06/20:mon", // Juneteenth
"07/03:fri", "07/05:mon", // Independence Day
"11/10:fri", "11/12:mon", // Veterans Day
"12/24:fri", "12/26:mon", // Christmas
```

## Skip dates that move each year.

Plenty of dates are really "the third Monday of January". Each of those always lands in the same seven days of its
month, so use that range with a weekday:

| Which one | Days                                            |
| --------- | ----------------------------------------------- |
| First     | 01-07                                           |
| Second    | 08-14                                           |
| Third     | 15-21                                           |
| Fourth    | 22-28                                           |
| Last      | The last seven, so 25-31 in a month with 31 days |

The day after Thanksgiving isn't always the fourth Friday, so it's a shifted copy of Thanksgiving's range instead:
`11/23-11/29:fri`.

## Skip weekends or a whole season.

A range can be as long as you like. `12/15-01/02` is an end of year freeze, and these two skip every weekend:

```go
SkipDates("01/01-12/31:sat", "01/01-12/31:sun")
```

If the pipeline only runs on a [cron](../../ref/extensions/provided/cron.md) schedule, leaving weekends out of the
expression is simpler. Skip dates are for days an expression can't describe, or pipelines that are started other ways.

## Check your dates against UTC.

A day starts and ends at midnight UTC. 8pm on December 25th in New York is already the 26th in UTC, so a job that runs
then needs `12/26` in its list. Work out which UTC day the run you want to skip falls on and list that one.

## Share one list between pipelines.

Pipeline configs are code, so keep your holidays in one place and pass them to each pipeline:

```go
var holidays = []string{"01/01", "07/04", "11/22-11/28:thu", "12/25"}

sdk.NewPipeline("nightly-report", "Nightly Report").SkipDates(holidays...)
```

## Know what else gets skipped.

Skip dates cover every way a run can start, not only schedules. If a pipeline also runs on pushes or pull requests,
those are skipped on the same days. Pull request checks from a skipped run show up as cancelled so they don't count as
passing.

When you need a run on one of those days, force it:

```bash
gofer pipeline run simple --force
```
