### Top remaining features before v1.0.0

### Github common task

We need a common way to alert on a PR or something that a task has succeeded or failed or similar.

### API

- Write/Ensure proper validation for all endpoints.
- User should also be able to give their builds a per-task timeout. Limit this to 5 min - 8 hours. If this is not set the scheduler should set it to 8 hours.
- We should have a feature that allows individual tasks and pipelines the ability to turn off logging.
- Implement a Global timeout for all runs.
- Implement the feature allowing people to attach into their containers and allow maintainers to turn that off.
- Offer canary type deployments for these. Allow the user to easily rollback pipeline configuration. Allow measuring failure rate and auto-rollback.
- Offer the ability to run two different versions of a container at the same time so that people can slowly roll out new jobs in a safe fashion. (green blue/canary)
  - You can all the different versions "revision"
  - This can possibly be done in the run step by specifying a different image name than is currently set to run.
- Purely for recovering lost containers, we can usually query the engine to ask it what time this container
  stopped and started. This way we can have more accurate running times instead of just leaving it as whenever
  the server restarted.
- Pipeline errors needs a bit of work, the idea isn't fully formed yet. We need a way to alert users that because of Gofer changes their
  pipeline wont work any more. Some type of notification system that pertinent to the pipelines. We could make pipeline errors their own endpoint
  which would be a good structurally.
  - What types of errors do we need to account for?
    - A trigger subscription has failed due to the trigger being not found or unable to be contacted.
    - A pipeline's config for trigger/commontask is invalid
    - A Gofer has uninstalled a trigger/commontask that a pipeline previously depended on.
  - When we uninstall common tasks or triggers we can list all pipelines that currently use those, disable them and add an error.
  - Create an API side pipeline validate that uses the sdk validate but also implements some things the SDK cannot do, like for instance check that all the triggers mentioned in the pipeline config are registered
- When you ask to list a pipeline or run or task run, if that thing or it's parent does not exist we should tell the user it doesn't.(This is most likely just simply passing up the not found error from the db)

### SDK

Update rust sdk library to be equal to golangs.

### Common Tasks

- It would be nice to create a common task with some basic "the user wants to do something when this condition
  happens". What is the best way to do this?
  - If pipeline fails 3 runs in a row.
  - If pipeline failure rate ever dives below certain percentage.
  - If total time of a run exceeds a given duration.
  - When a run finishes.
  - When a run fails.
  - If a particular task run fails or succeeds.

### CLI

- Biggest feature missing is we are not currently doing proper json output.
- Provide custom errors downstream via grpc metadata so the CLI can pass back intelligent errors to users.
- Add command line options for controlling pagination
- Combined logs feature: Ability to attach to all task runs at the same time and dump from the run.
- We should have a watch function for run that nicely displays the task runs as they start and finish.
  (We could even have the active task_runs display the last 5 log lines as it runs each in their own personal terminal print load bar thing)
- Create a namespace set command that allows the user to switch between namespaces and save it in their configuration file (CLI).
- CLI now just compiles from language. This means that we can also just straight up read from things like json/toml since it all compiles back to json anyway.
- https://github.com/clintjedwards/gofer/commit/955e1b7da76fdfa5aa26bcb5dd0b138af605aa45
- Pipeline get needs to put more detail (list tasks, triggers, commontasks)
- Create an init function for both rust and golang(simply just prompts you for your language) and then creates a new default vanilla pipeline. (similar to the old "config init" command)

### Scheduler

- Implement CPU/MEMORY per task values since all non-local schedulers will need this.
- It would be cool to have at least one other scheduler. Nomad is a great scheduler for this.

### SecretStore

- It would be cool to get at least one other secret store implementation like Vault.
  - For an extension like vault we manage the read and write in the same way we would for bolt. So vault gives us a prefix
    path and we essentially just used that prefix path to store secrets.
- Write a function to clean up secret store and object store stuff when a user deletes a namespace or pipeline. Have the user
  be able to turn this off in the config.

### Triggers

- Test that unsubscribing works with all triggers. And create a test suite that triggers can run against.
- The interval trigger should create jitter of about 2-5 mins. During that time it can choose when to start counting to trigger an event. This is so that when we restart the server all events don't perfectly line up with each other.
- Triggers should follow semver. Triggers that use the same major version of Gofer should be compatible.
- If a trigger by the same name is already installed, we should refuse to install another but instead allow the user to update it.
- Should triggers be able to pass "metadata" values back to pipelines that are secret?

### Things I need to do but probably will never get around to

- Test registry auth.

### General

- For FromProto methods where the reference might be nil; auto-create the reference before attempting to fill it. Look at registry auth for an example.
- Metrics via openTelemetry
- Database functions need to be more flexible. The caller should be able to mix and match and start/stop transactions at will.
- Check that when we create the run specific token for a run and enter it into the user's run stuff. We also need to make sure we clean
  that token up after the run is done.

### Rough spots in design

- It currently runs as a singleton, not distributed.
- Because things are handled at the current abstraction layer for users who just want to throw code and have it work it can be difficult. Users who operate within Gofer will have to do at least some thought about repositories downloads, possibly caching, transferring between containers, etc. These are all things that some CI/CD systems give for free. The managing of large git repos is the biggest pain point here.
  - To give people the ability to cache certain important items like repositories we can create a special ubuntu container with a fuse file system. We can then allow people to use this container to connect back to the object fs and make common tasks like storing your repo easy.
- The umbrella for this tool is large. There is a reason Jenkins still leads, the plugin ecosystem needs significant time to catch up to its large ecosystem and then to do it properly would require non-insignificant maintenance.
- It is possible for a trigger subscription to be disabled due to network error and the trigger to still send it a successful event, but
  not understand that it wasn't successfully delivered. Overtime this might cause drift between what events triggers should actually be sending back.
- Give a thought to models as they move through different phases from the Config -> SDK -> Proto -> Models.
  Right now it can be kinda hard to figure out which ends of the program might produce which objects.
  - For instance when dealing with common tasks, they move from commonTask models to commonTaskConfigs and the transition doesn't seem to make a bunch of sense.
  - At the very least we should maybe make a table that relates the models to each other and document why
    they might be in a certain shape.

### Documentation

- Document the different env variables that get injected into each Trigger, Task, Notifier.
- Trigger documentation:
  - Triggers now have two required functions, trigger installations and trigger runs
    - Run is the service, Install runs a small program meant to help with installation.
  - How to test triggers
  - How to work with triggers locally
  - Explanation of the SDK on writing triggers
- Add interval as the example for new triggers in the docs
- Document why common tasks are designed the way they are first-class citizens.
  - Why is this? Because of authentication. It's nice to set up the Slack app once and protect the credentials such that any user for you application can use it.
- Improve documentation and examples for features.
  - For example: writing custom notifiers allows you to implement Google style static analysis
- We can probably bring up a public version of Gofer in which the timeout is super low. How do we properly secure this? Can we prevent root containers when using the Docker mode? Maybe this is a thing for Nomad? Can we prevent root containers there? This might mean we need to add quotas and rate limiting to the main process
  which would suck and is boring to implement, but having the functionality there might make this a more scale-able tool.
- Secrets explanation. Why is there global secrets and pipelines secrets? Whats the difference.
  - We needed a way to store secrets for common tasks which might be used for any pipeline
    and a way to store secrets for user's individual pipelines.
  - Global secrets can only be set by administrators
- Write a small RFC for Gofer. Why were the decisions made the way they were, what was the purpose of the project, etc etc.
- Write copius notes on commontasks and triggers layout. The difference between user passed config and system passed config. And suggest a way to collect those.
  - Gofer passes them one set of env vars from the gofer system itself
    These are prefixed with `gofer_plugin_system_{var}`
  - Gofer then passes them another set of env vars from the admin that was set up through registration.
    These are prefixed with `gofer_plugin_config_{var}`
  - Gofer then passes them another set of env vars from the user's own config.
    These are prefixed with `gofer_plugin_param_{var}`

### On the floor

- Use Mdbook for documentation.

  - After mdbook upgrade update all code links to it.
  - Document the debug containers also
  - Replace blurry png for readme.
  - Add an example of entrypoint/command running a multi-line script
  - Take API.md and combine it with general how to use Gofer docs
  - Review all links to make sure they're not broken. Lots of stuff changed with mdbook.

* Orphaned run recovery is currently broken.

- TestGetALL fails with race condition, check it out. I think it's a known issue.
- Pipeline updates for CLI is broken.
<!--

## Auth

You can authenticate to Gofer using GRPC's metadata pair:

```go
md := metadata.Pairs("Authorization", "Bearer "+<token>)
```

More details about auth [can be found here.](server-configuration/auth) -->
