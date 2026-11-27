# Common Patterns and Best Practices

These are the habits that make Gofer pipelines easy to run and easy to trust. Most of them come from the same facts:
every task is its own container, and Gofer only knows what you tell it. It also
doesn't tie pipelines to a git repository.

Here's everything at a glance:

**[Writing Tasks](./writing_tasks.md)**

- [Make tasks safe to run twice.](./writing_tasks.md#make-tasks-safe-to-run-twice)
- [Take configuration from environment variables.](./writing_tasks.md#take-configuration-from-environment-variables)
- [Pin your base images.](./writing_tasks.md#pin-your-base-images)
- [Keep your images lean.](./writing_tasks.md#keep-your-images-lean)
- [Fail loudly.](./writing_tasks.md#fail-loudly)

**[Designing Pipelines](./designing_pipelines.md)**

- [Keep things simple.](./designing_pipelines.md#keep-things-simple)
- [Use `Any` for cleanup and notifications.](./designing_pipelines.md#use-any-for-cleanup-and-notifications)
- [Limit pipelines that can't overlap.](./designing_pipelines.md#limit-pipelines-that-cant-overlap)
- [Keep your pipeline config next to your code.](./designing_pipelines.md#keep-your-pipeline-config-next-to-your-code)

**[Skipping Runs](./skipping_runs.md)**

- [Give a pause an end date.](./skipping_runs.md#give-a-pause-an-end-date)
- [Skip just the next run.](./skipping_runs.md#skip-just-the-next-run)
- [Skip US federal holidays.](./skipping_runs.md#skip-us-federal-holidays)
- [Skip dates that move each year.](./skipping_runs.md#skip-dates-that-move-each-year)
- [Skip weekends or a whole season.](./skipping_runs.md#skip-weekends-or-a-whole-season)
- [Check your dates against UTC.](./skipping_runs.md#check-your-dates-against-utc)
- [Share one list between pipelines.](./skipping_runs.md#share-one-list-between-pipelines)
- [Know what else gets skipped.](./skipping_runs.md#know-what-else-gets-skipped)

**[Passing Data Between Tasks](./passing_data_between_tasks.md)**

- [Small values](./passing_data_between_tasks.md#small-values)
- [Files and directories](./passing_data_between_tasks.md#files-and-directories)
- [Things that trip people up](./passing_data_between_tasks.md#things-that-trip-people-up)

**[Working with Git Repositories](./working_with_git_repositories.md)**

- [Getting access](./working_with_git_repositories.md#getting-access)
- [Getting the code](./working_with_git_repositories.md#getting-the-code)

**[Secrets and Access](./secrets_and_access.md)**

- [Prefer pipeline secrets over global ones.](./secrets_and_access.md#prefer-pipeline-secrets-over-global-ones)
- [Keep secrets out of images.](./secrets_and_access.md#keep-secrets-out-of-images)
- [Only give API tokens to tasks that need them.](./secrets_and_access.md#only-give-api-tokens-to-tasks-that-need-them)
