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

- ExecuteTaskTree has a bunch of ways to fail. When it does we should allow those failures to be shown in the taskrun
  instead of just being printed out in the logs. So we'll probably need to add all the taskruns as cancelled and the
  run as failed due to preconditions.

* Think about re-writing and packaging the Config package
* Make sure the Notifier config and the Trigger config saved into the database don't save sensitive values in plaintext.
* Investigate writing a small script to help with updating events. We can run it with make.
* Triggers when asked should return a list of registered pipelines
* Fill out the registered response in all triggers
* All triggers/notifiers now need installer scripts
* The trigger/notifier/installer code needs some massive cleanup
* When we lose a check connection with a trigger go ahead and mark that trigger as inactive. This removes the need for monitorTrigger routine. #healthcheck
  - We can do even do auto-trigger attempted heal here, but respawning the trigger.
* When we uninstall a trigger we need to then kill the goroutinue for the check routine.
* Remove all replace directives in gofer-containers.

### Rewrite

- Cleanup by replacing \* imports and using local imports for long packages
- Possibly make the comfy-tables crate respect NO_COLOR
- Take a look at all the places we unwrap and clean up if needed.
- Reflection doesn't work. TLS doesn't work in dev.
- Create a namespace set command that allows the user to switch between namespaces and save it in their configuration file (CLI).
- Document/Comment all sdk libraries.
- We can potentially auto detect languages by looking for auto language structure.
- We can also just straight up read from things like json/toml since it all compiles back to json anyway.
- Fix this regression: {{- if not (len $trigger.Events) 0 }} recent events:{{- end }} in pipeline get
- Fix events for all cli stuff.
- We should be more detailed on some of the parameters in proto, instead of 'id' use 'pipeline_id'
- In our integration testing for storage, test that cascading deletes works
- Separate store_keys into it's own table
- Reevaluate if we need docker-cancellations
- We can make an env package that doesn't suck.
- Map will preserve a None type if it is still there, go back and look over the database package and fix what might be.
- Purely for recovering lost containers, we can usually query the engine to ask it what time this container
  stopped and started. This way we can have more accurate running times instead of just leaving it as whenever
  the server restarted.
- Rust trigger SDK
- Install trigger should be able to be called again to update the trigger's configuration.
- When we check the name for created identifiers make sure we use the same check as the one for the sdk config
- Pipeline validate - must have at least one task - limit parallesim to something like 20
- Remove replace directive use normal go get
- Write the compiler logic for golang now that the sdk is finished.
- Config is near completion we just have to fix: https://github.com/YushiOMOTE/econf/issues/11
- Subscribe in the event system should ideally take an enum without the caller having to specify what is inside
  the enum. (what is inside gets thrown away anyway). Is there an easy way to do this?
- TLS needs more time spent.

  - We need to figure out how to pass a file path,
  - It would be great if we could get untrusted certs to work, so we don't have to be in two different modes for
    dev and prod.

- If a trigger by the same name is already installed, we should refuse to install another.
  - Maybe have a force function in the CLI to say "hey, if you want we'll uninstall this for you".
- Implement collect logs for triggers.
- Make sure to go back and make sure that all secret values persisted to the database are encrypted.
- We should have a watch function for run that nicely displays the task runs as they start and finish.
  (We could even have the active task_runs display the last 5 log lines as it runs each in their own personal terminal print load bar thing)
- Change triggers so that the CLI collects the configuration needed for install and uninstall.
- Our validator code for API works, but it's kinda ugly

-- When we instll/uninstall triggers the first line should be which trigger we're setting up. and query lines should be
prefixed with >2

- The gofer sdk installationInstructions doesn't modify self. So you have to remember to assign it to yourself.
- Switch gofer-containers to point to the main branch and not rust rewrite
- trigger-install cli cmd should force the user to chose either use -i or -v. Even if that means -v must be empty
- Add an example of entrypoint running a script
