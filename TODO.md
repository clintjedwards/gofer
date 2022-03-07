### Top remaining features before v1.0.0

#### Git downloader container

- Support auth, support downloading into run or pipeline objects, support checking out specific commit.
- Support sparse checkouts?
- Support caching, don't redownload the container every run just simply git pull it.

#### Generate per run auth keys and allow people to use the binary to do stuff like download their favorite key from the object store.

#### Nomad scheduler integration

#### An "notify" function for pipeline configuration

- Notifications work in the same fashion as triggers. We spin up a bunch at launch time and then continually feed them
  events as they happen.
- These containers can use a default "if this should happen" function that we write and provide in the sdk. Things like:
  - If pipeline run fails x times
  - If % of pipeline run fails
  - If total time of run > <duration>
- The output of that function should be whether to send a notification or not and a message on what failed.
- The container can then format that message for the notifier and then send it to the respective platform.
- Why not make this a container that just runs right after the user's container?
  - Some queries based on time will be very hard to write. For example: "Please alert me when this pipeline has not run
    in a while.". With trigger-like notifers you can do that, with pipeline based notifiers you cannot.
- we can allow things like annotations due to the feature of access to the kv store. Meaning that results from the most
  recent container run can be stored in the kv store and then we can pull those and pass to the notifer before it triggers.
- First notifier should be the github notifier

#### Json output

### API

- Write validation for all endpoints
- User should also be able to give their builds a per-task timeout. Limit this to 5 min - 8 hours. If this is not set the scheduler should set it to 8hours.
- We should have a feature that allows individual tasks and pipelines the ability to turn off logging.
  and anything else that is hard to glean from other locations.
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
- Implement json output. This should involve separating the output of what should be humanized vs regular.
- Combined logs feature: Ability to attach to all task runs at the same time and dump from the run.

### Scheduler

- Implement CPU/MEMORY per task values since all non-local schedulers will need this.

### SecretStore

- Vault impl
- Check that the cli properly prevents people from requesting any secret, in any pipeline. This should be a simple
- namespace check.
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

- Before we slurp entire HCL files into memory first check that we have enough memory available.
- Metrics via openTelemetry
- Include the ability to turn off database filters and return all results(abandoned pipelines, deleted namespaces).

### Rough spots in design

- It currently runs as a singleton, not distributed.
- Because things are handled at the current abstraction layer for users who just want to throw code and have it work it can be difficult. Users who operate within Gofer will have to do at least some thought about repositories downloads, possibly caching, transferring between containers, etc. These are all things that some CI/CD systems give for free. The managing of large git repos is the biggest pain point here.
- The umbrella for this tool is large. There is a reason Jenkins still leads, the plugin ecosystem needs significant time to catch up to its large ecosystem and then to do it properly would require non-insignificant maintenance.
- It is possible for a trigger subscription to be disabled due to network error and the trigger to still send it a successful event.
  Overtime this might cause drift between what events triggers should actually be sending back.
- Events have to be managed in multiple places making them a maintenance nightmare when you have to add, remove, or update events.

### Documentation

- Trigger documentation:
  - How to test triggers
  - How to work with triggers locally
  - Explanation of the SDK on writing triggers
- Add interval as the example for new triggers in the docs
- Write a design document
- Improve documentation and examples for features.
  - For example: writing custom notifiers allows you to implement Google style static analysis

* Investigate possible bug where user changes a pipe to a different ID and then tries to update the old pipeline.
  From the error it seems like its trying to pull the incorrect pipeline.
* We need concurrent protection for `triggers map[string]*models.Trigger`
* Cancelled during run should say something more descriptive. See if we can track down by cancel function and if there
  is a space for more description.
* Printing a task run in CLI should also show parents
* Can we get FAILED_PRECONDITION to be a bit clearer on where things failed? FOr example is should say what it was depending on, what the status of that thing was and explain thats why it failed.

* Exec command
* Take what we get from the exec command input
* We strip the whitespace from the ends
* We base64 the entire thing and store it alongside the pipeline
* We can then pass this to the receiver by calling the relevant bash command and un-base64ing it in the container.
* Document how one would test exec commands locally (simply just echo into docker)
* Document new exec statement
* Last we left off we implemented the exec into tasks so now we just need to implement them on the scheduler side.
