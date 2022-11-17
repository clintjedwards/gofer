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

- Rust sdk tasks methods needs a better UX. Maybe a macro that will wrap the user's items in a box for them?
- Rust sdk needs documentation.
- Rust documentation possibly needs to be on cargo.io.

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

- Provide custom errors downstream via grpc metadata so the CLI can pass back intelligent errors to users.
- Improve CLI errors overall.
- Add command line options for controlling pagination
- Combined logs feature: Ability to attach to all task runs at the same time and dump from the run.
- We should have a watch function for run that nicely displays the task runs as they start and finish.
  (We could even have the active task_runs display the last 5 log lines as it runs each in their own personal terminal print load bar thing)
- Create a namespace set command that allows the user to switch between namespaces and save it in their configuration file (CLI).
- CLI now just compiles from language. This means that we can also just straight up read from things like json/toml since it all compiles back to proto anyway.
- https://github.com/clintjedwards/gofer/commit/955e1b7da76fdfa5aa26bcb5dd0b138af605aa45
- Pipeline get needs to put more detail (list tasks, triggers, commontasks)
- Create an init function for both rust and golang(simply just prompts you for your language) and then creates a new default vanilla pipeline. (similar to the old "config init" command)
- Inspiration for CLI design: https://github.com/bensadeh/circumflex
  - Look into bubble tea for some interactions.

### Scheduler

- Implement CPU/MEMORY per task values since all non-local schedulers will need this.
- It would be cool to have at least one other scheduler. Nomad is a great scheduler for this.
- Docker scheduler should check for the docker process before attempting to connect to it.

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

### Public Gofer ideas

- Is it feasible to expose Gofer to the public?
- Can we give user's a timeout that is super low, like a total container runtime of 5 mins. That way you can try it out, but you can't just run your own crypto shit on it.
- Once the timeout is up we simply log the IP and prevent that user from making any more requests.
- We might be able to get this for free in some golang ratelimiting libraries, we'd have to have the user sign up in some way first in order to prevent people from abusing. We can ratelimit routes that need to be always public per IP.
- How do we secure the running of containers? We can do somethings like preventing root user for the container: https://firecracker-microvm.github.io/

### Documentation

- Server configuration reference should have one more field on whether it is required or not.
- Trigger documentation:
  - Triggers now have two required functions, trigger installations and trigger runs
    - Run is the service, Install runs a small program meant to help with installation.
  - How to test triggers
  - How to work with triggers locally
  - Explanation of the SDK on writing triggers
- Add a section where we create a new trigger using a trigger that has already been created. as the example for new triggers in the docs
- Secrets explanation. Why is there global secrets and pipelines secrets? Whats the difference.
  - We needed a way to store secrets for common tasks which might be used for any pipeline
    and a way to store secrets for user's individual pipelines.
  - Global secrets can only be set by administrators
- Write a small RFC for Gofer. Why were the decisions made the way they were, what was the purpose of the project, etc etc.
  - We are forgoing having cli spit out Json due to gofer having an API, the cli is meant for humans and shouldn't be used by programs.
- Write copius notes on commontasks and triggers layout. The difference between user passed config and system passed config. And suggest a way to collect those.
  - Gofer passes them one set of env vars from the gofer system itself
    These are prefixed with `gofer_plugin_system_{var}`
  - Gofer then passes them another set of env vars from the admin that was set up through registration.
    These are prefixed with `gofer_plugin_config_{var}`
  - Gofer then passes them another set of env vars from the user's own config.
    These are prefixed with `gofer_plugin_param_{var}`

### On the floor

- Create a container for custom use that has gofer-cli already packed in and possibly allows
- Fixing Pipeline updates and rolling out versioned pipelines.
  - Gofer needs versioned pipelines as a first step into supporting the possibility of canarying pipelines.
    - We need to make a user settable limit for pipeline versions. Delete older versions.
  - Several things need to get done
    1. We need to figure out how to support versioned pipelines in the context of the database and data models. We'll probably need to change schema quite a bit.
    2. Clean up how trigger sub/un-subs work. Hitting the upgrade endpoint for your pipeline should return immediately and update the pipeline's status to updating.
       - (We'll probably need to add statues to pipeline [Ready, Updating])
       - During this "updating" time Gofer will remove triggers and subscribe triggers as necessary.
       - If this process fails Gofer will rollback to old trigger state.
         - If this fails Gofer will mark the pipeline as paused(or better) for a specific reason.
       - Clients of the API will kick off the update by passing the proto as usual and then listen for pipeline updates which can then be relayed to the user.
       - Once the pipeline finishes updating the pipeline will switch back to Ready state but the API will not
         autoswitch back to active.
    3. We need to add "update methods" to pipeline settings which will control the manner in which we roll out updates. Runs will need to include which version of the pipeline has run
- Orphaned run recovery is currently broken.
- Instead of injecting Gofer API tokens by default, allow the user to turn it on per pipeline and possibly even better allow the user to opt out certain tasks from receiving the key.
- Make sure that common tasks also get the same injected vars that other tasks get. This should be a baseline injection that all tasks can expect. Those tasks can then choose to ignore those specific vars.
- Clean up both github triggers and add a github common task.
  - common task we can throw in there as a parallel task a the start of each pipeline. It will consume github commit, inform github of the pipeline pending and then query gofer to see when the run has ended. When the run ends the task will then inform github that the run has finished with a particular state.
- Think about making a new task type that you can pass commands to that automatically uses the gofer container. So users can get zero to code ultra-fast.
- When we attempt to create a pipeline we should test that the namespace exists before attempting to create it.
- Get TaskRun is broken.
