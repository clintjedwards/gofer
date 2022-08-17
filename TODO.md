### Top remaining features before v1.0.0

#### Git downloader container

- Support auth, support downloading into run or pipeline objects, support checking out specific commit.
- Support sparse checkouts?
- Support caching, don't re-download the container every run just simply git pull it.
- Should we include this as a feature of the github plugin? When a new trigger event comes in we can give the option
  to go ahead and make sure the github repo is up to date in the object store.

#### Nomad scheduler integration

#### Github notifier

#### Enable the ability to install triggers and notifiers by CLI. Config will just be one of two ways to install.

- This will allow us to uncomplicate some installations like installing the Github trigger
- Is there a way we could build this into triggers so that we simply run the container and connect to it in order
  to run the installation. (then the container can pass orchestrate the entire thing, we can also communicate with
  Gofer automatically to install it)

### API

- Write/Ensure proper validation for all endpoints.
- User should also be able to give their builds a per-task timeout. Limit this to 5 min - 8 hours. If this is not set the scheduler should set it to 8 hours.
- We should have a feature that allows individual tasks and pipelines the ability to turn off logging.
  and anything else that is hard to glean from other locations.
- Implement a Global timeout for all runs.
- Implement the feature allowing people to ssh into their containers and allow maintainers to turn that off.
- Offer canary type deployments for these. Allow the user to easily rollback pipeline configuration. Allow measuring failure rate and auto-rollback.
- Offer the ability to run two different versions of a container at the same time so that people can slowly roll out new jobs in a safe fashion. (green blue/canary)
- You can all the different versions "revision"
- This can possibly be done in the run step by specifying a different image name than is currently set to run.
- DeleteNamespace should abandon all pipelines. Technically if you know the namespace name you're still allowed to run jobs.
- For security reasons we probably need to narrow the amount of places you can import remote files from. Because we need to hide any auth information if included.
- How do we make it easy for users to request authentication? Should that be an ops problem?

### Notifiers

- It would be nice to provide notifiers with some basic "the user wants to do something when this condition
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
- Implement json output. This should involve separating the output of what should be humanized vs regular.
- Combined logs feature: Ability to attach to all task runs at the same time and dump from the run.

### Scheduler

- Implement CPU/MEMORY per task values since all non-local schedulers will need this.

### SecretStore

- Vault impl
- Check that the cli properly prevents people from requesting any secret, in any pipeline. This should be a simple namespace check.
- For an extension like vault we manage the read and write in the same way we would for bolt. So vault gives us a prefix
  path and we essentially just used that prefix path to store secrets.
- It might be possible to just insert secrets anywhere you want if you set it correctly. We need to check that
  the user has permissions before put or getting secrets once more.
- You should be able to list secret keys for your pipeline.

### Triggers

- Test that unsubscribing works with all triggers. And create a test suite that triggers can run against.
- The interval trigger should create jitter of about 2-5 mins. During that time it can choose when to start counting to trigger an event. This is so that when we restart the server all events don't perfectly line up with each other.
- triggers should follow semver. Triggers that use the same major version of Gofer should be compatible.

### General

- Metrics via openTelemetry

### Rough spots in design

- It currently runs as a singleton, not distributed.
- Because things are handled at the current abstraction layer for users who just want to throw code and have it work it can be difficult. Users who operate within Gofer will have to do at least some thought about repositories downloads, possibly caching, transferring between containers, etc. These are all things that some CI/CD systems give for free. The managing of large git repos is the biggest pain point here.
- The umbrella for this tool is large. There is a reason Jenkins still leads, the plugin ecosystem needs significant time to catch up to its large ecosystem and then to do it properly would require non-insignificant maintenance.
- It is possible for a trigger subscription to be disabled due to network error and the trigger to still send it a successful event.
  Overtime this might cause drift between what events triggers should actually be sending back.

### Documentation

- Document the different env variables that get injected into each Trigger, Task, Notifier.
- Trigger documentation:
  - Triggers now have two required functions, trigger installations and trigger runs
    - Run is the service, Install runs a small program meant to help with installation.
  - How to test triggers
  - How to work with triggers locally
  - Explanation of the SDK on writing triggers
- Add interval as the example for new triggers in the docs
- Write a design document
  - Document why notifiers are designed the way they are first-class citizens.
  - Why is this? Because of authentication. It's nice to set up the Slack app once and protect the credentials such that any user for you application can use it.
- Improve documentation and examples for features.
  - For example: writing custom notifiers allows you to implement Google style static analysis

* We can probably bring up a public version of Gofer in which the timeout is super low. How do we properly secure this? Can we prevent root containers when using the Docker mode? Maybe this is a thing for Nomad? Can we prevent root containers there? This might mean we need to add quotas and rate limiting to the main process
  which would suck and is boring to implement, but having the functionality there might make this a more scale-able tool.

### On the floor

- Create a namespace set command that allows the user to switch between namespaces and save it in their configuration file (CLI).
- Write SDK library for rust both trigger and pipeline config.
- Write code to detect which language and then call the appropriate compiler.
- We can also just straight up read from things like json/toml since it all compiles back to json anyway.
- In our integration testing for storage, test that cascading deletes works
- Separate store_keys into it's own table
- Purely for recovering lost containers, we can usually query the engine to ask it what time this container
  stopped and started. This way we can have more accurate running times instead of just leaving it as whenever
  the server restarted.
- Install trigger should be able to be called again to update the trigger's configuration.
- Pipeline validate - must have at least one task - limit parallesim to something like 20 - Make sure there are no cycles.
- If a trigger by the same name is already installed, we should refuse to install another.
  - Maybe have a force function in the CLI to say "hey, if you want we'll uninstall this for you".
- Make sure to go back and make sure that all secret values persisted to the database are encrypted.
- We should have a watch function for run that nicely displays the task runs as they start and finish.
  (We could even have the active task_runs display the last 5 log lines as it runs each in their own personal terminal print load bar thing)
  -- When we instll/uninstall triggers the first line should be which trigger we're setting up. and query lines should be
  prefixed with >2
  -- Write the logic for installing triggers in the new world.
- Switch gofer-containers to point to the main branch and not rust rewrite
- trigger-install cli cmd should force the user to chose either use -i or -v. Even if that means -v must be empty
- Add an example of entrypoint/command running a multi-line script
- Include interpolation wrappers in the gofer sdk for pipelines. Should just simply wrap values and provide the string format.
- In the CLI put a 'pipeline run' and a 'run start' that both just call the same endpoint.
- In the SDK make it so that people can mix both gofer tasks and regular tasks and then unmix them in the actual thing.
- Remember to pass back common tasks in the pipeline get call to the client for the cli.
- For all models -> Implement from for both proto -> model and model -> proto.
- Use Mdbook for documentation.
- Upgrade all packages.
- Swap API service to using go 1.19's new atomic pointer stuff.
- For trigger regs and common task reg when we install them we need a better way to figure out what things are okay
  and not okay to show back to regular users. For example we may want to show the user the this trigger has a global
  limit of some setting, but we DON'T want to show the user that this trigger has 'y' API key.
- Get rid of the schedulerID
- We should get rid of the variable ownership/privacy stuff. It makes things needlessly complicated. Instead lets create a new
  "global" secret object which can store secrets for all pipelines/runs/etc. This makes it so that system can register secrets
  that are available to all pipelines/runs/etc and it operates in the same way all other secrets etc works.
- https://github.com/clintjedwards/gofer/commit/955e1b7da76fdfa5aa26bcb5dd0b138af605aa45
- Implement a check for triggers when we call the watch function if that ever fails change the trigger state and try to relaunch/reconnect it/etc
- We need to take a look at DB best practices. The flexibility of transactions and which functions can be put into them by the api is not great.
- Should triggers be able to pass "metadata" values back to pipelines that are secret?
- When we uninstall common tasks or triggers we can list all pipelines that currently use those, disable them and add an error.
- Pipeline errors needs a bit of work, the idea isn't fully formed yet. We need a way to alert users that because of Gofer changes their
  pipeline wont work any more. Some type of notification system that pertinent to the pipelines. We could make pipeline errors their own endpoint
  which would be a good structurally.
  - What types of errors do we need to account for?
    - A trigger subscription has failed due to the trigger being not found or unable to be contacted.
    - A pipeline's config for trigger/commontask is invalid
    - A Gofer has uninstalled a trigger/commontask that a pipeline previously depended on.
- syncmap needs a get and swap method
- When we create the run specific token for a run and enter it into the user's run stuff. We also need to make sure we clean
  that token up after the run is done.
- Restore CLI config init
- Polyfmt needs to move the filter to be a builder function and the provide stringf functions.
- We need a list objects for run objects and pipeline objects.
- Replace blurry png for readme.
- For secret store and object store stuff need to manually clean those up as we can't rely on cascade on delete.
- Everything we just did for secret key probably needs to be done for object keys.
