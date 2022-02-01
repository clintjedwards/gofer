### Top remaining features before v1.0.0

#### Repo downloader container

- Support auth, support downloading into run or pipeline objects, support checking out specific commit.
- Support sparse checkouts?
- Support caching, don't redownload the container every run just simply git pull it.

#### Generate run level auth keys and allow people to use the binary to do stuff like download their favorite key from the object store.

#### Possibly an event system so people and things can work off events

- We could have async subscriptions so that

#### Nomad scheduler integration

#### An "notify" function for pipeline configuration

- we can allow things like annotations due to the feature of access to the kv store.
- this works exactly like triggers
- where Gofer will run a particular export container along with the user's pipeline. This container can ship
  container focused results back to something like email, slack, github, whatever.

#### Access to the kv store via pipeline configuration

-- we can give user's the ability to request store specific variables with the same syntax we do with secrets.

#### Json output

### API

- Write validation for all endpoints
- User should also be able to give their builds a per-task timeout. Limit this to 5 min - 8 hours. If this is not set the scheduler should set it to 8hours.
- We should have a feature that allows individual tasks and pipelines the ability to turn off logging.
  and anything else that is hard to glean from other locations.
- Make sure event results actually are processes correctly, we should correctly log events that are skipped without
  triggering a new run.
- Global timeout for all runs
- Implement the feature allowing people to ssh into their containers and allow maintainers to turn that off.
- Offer canary type deployments for these. Allow the user to easily rollback pipeline configuration. Allow measuring failure rate and auto-rollback.
- Offer the ability to run two different versions of a container at the same time so that people can slowly roll out new jobs in a safe fashion. (green blue/canary)
- This can possibly be done in the run step by specifying a different image name than is currently set to run.
- DeleteNamespace should abandon all pipelines. Technically if you know the namespace name you're still allowed to run jobs.
- For security reasons we probably need to narrow the amount of places you can import remote files from. Because we need to hide any auth information if included.

### CLI

- Provide custom errors downstream via grpc metadata so the CLI can pass back intelligent errors to users.
- Add command line options for controlling pagination
- When presenting errors back to the user make sure we're presenting them in an understandable manner. This might mean catching errors and providing the users with tips on how to fix them.
- Implement json output makes sense. This should involve separating the output of what should be humanized vs regular.

### Scheduler

- Implement CPU/MEMORY per task values since all non-local schedulers will need this.

### SecretStore

- Vault impl
- Check that the cli properly prevents people from requesting any secret, in any pipeline. This should be a simple
- namespace check.
- For an extension like vault we manage the read and write in the same way we would for bolt. So vault gives us a prefix
  path and we essentially just used that prefix path to store secrets.
- Document all changes that arise from the new revolution in secrets
- It might be possible to just insert secrets anywhere you want if you set it correctly. We need to check that
  the user has permissions before put or getting secrets once more.
- You should be able to list secret keys for your pipeline.

### Triggers

- Test that unsubscribing works with all triggers. And create a test suite that triggers can run against.
- The interval trigger should create jitter of about 2-5 mins. During that time it can choose when to start counting to trigger an event. This is so that when we restart the server all events don't perfectly line up with each other.
- triggers should follow semver

### General

- Before we slurp entire HCL files into memory first check that we have enough memory available.
- Combined logs feature: Ability to attach to all task runs at the same time and dump from the run.
- Metrics via openTelemetry
- Include the ability to turn off database filters and return all results(abandoned pipelines, deleted namespaces).

### Rough spots in design

- It currently runs as a singleton, not distributed.
- Because things are handled at the current abstraction layer for users who just want to throw code and have it work it can be difficult. Users who operate within Gofer will have to do at least some thought about repositories downloads, possibly caching, transferring between containers, etc. These are all things that some CI/CD systems give for free. The managing of git repos is the biggest pain point here.
- The umbrella for this tool is large. There is a reason Jenkins still leads, the plugin ecosystem needs significant time to catch up to its large ecosystem and then to do it properly would require non-insignificant maintenance.
- It is possible for a trigger subscription to be disabled due to network error and the trigger to still send it a successful event.

### Documentation

- Trigger documentation:
  - How to test triggers
  - How to work with triggers locally
  - Explanation of the SDK on writing triggers
- Add interval as the example for new triggers in the docs

* when we can't connect to a trigger we should print the logs output if it exists.
