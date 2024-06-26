### On the floor

- A dope feature to bake into github extensions is the ability for it to act also as a communicator with the github
  PR. When you start a pipeline not only does it handle the webhooks of starting the pipeline, but it will also
  mark the PR in question as pending and query the Gofer API to figure out when it's done and mark the pipeline
  as completed. Could also include some other goodies just like any other CI/CD platform.
  - Github Extension:
  - Restore ability to register a pipeline to be triggered every time some event happens.
  - Update Github extension documentation in external docs
  - Github extension also needs to do check-run stuff
  - Mention the 'all' action in documentation
  - Update Github documentation, it needs a lot of work.
- Minify CSS when we release for frontend.
- Registry auth is largely untested and possibly unsecured, don't use it for anything serious.
- Make sure to test and document namespace glob matching for tokens
- Include in dev docs how to build openapi stuff: (oapi-codegen)
- Write a test for bootstrap tokens.
- The CLI should allow you to dump the debug for an extension.
- We should create a background job that all it does is attempt to clean out tokens which have been expired for a certain amount of time.
- Write a test for force on objects and secrets
- We need to make sure that the secret key name is always URL friendly since we use it as a key in the new REST API.
- Implement run gofer_token in runs.rs
- Document how to fix things with #[schemars(rename = "run_status_reason")] and how they will manifest.
- Create sdkutils for rust sdk also.
- Finish Deployments
- still need to do Subscriptions (including the startup/restore function)
- run cargo bloat and matklad's tool to see what crate is making compilation really slow.
- Make sure is_valid_identifier is used in all the places where the user has to enter an id.
- Remove the circular dependency on openapi and the code, it often ends in sadness since in order to generate things
  we need the code to work and the code doesn't work if the sdk doesn't provide some functions. But the functions aren't provided
  because it can't be generated and round we go.
- When a user removes an extension we should remove the subscriptions also (check we don't already do this, due to cascade delete).
- Restore the restore jobs functionality.
- We want to restrict the max size of the request body, but some endpoints need large bodies to upload things to us. We should
  get rid of the global restriction and instead check for what the request size should be in the preflight.
- Make the object store uploads multipart.
- The way we use joinset right now it kinda suboptimal. We should return errors with our async functions instead of breaking.
  Then we can just return the error to the main thread and bubble it up properly.
- When we shutdown the server we need a way to try to clean up all the webhook connections that might still be being used by
  people.
- For the run token we should make sure we set namespace properly on behalf of the user.
- There is a bug where restarts will cause extensions to use brand new tokens. We should at least clean up the old ones.

### Canaried pipelines

- With versioned pipelines we now need the ability for Gofer to roll out your pipeline for you and watch telemetry on it.
- We need to add "update methods" to pipeline settings which will control the manner in which we roll out updates. Runs will need to include which version of the pipeline has run

### API

- The API needs proper validation for all endpoints and probably some fuzz testing.
- User should also be able to give their builds a per-task timeout. Limit this to 5 min - 8 hours. If this is not set the scheduler should set it to 8 hours.
- We should have a feature that allows individual tasks and pipelines the ability to turn off logging.
- Implement a Global timeout for all runs.
- Implement the feature allowing people to attach into their containers and allow maintainers to turn that off.
- Offer the ability to run two different versions of a container at the same time so that people can slowly roll out new jobs in a safe fashion. (green blue/canary)
- Purely for recovering lost containers, we can usually query the engine to ask it what time this container stopped and started. This way we can have more accurate running times instead of just leaving it as whenever
  the server restarted.
- The CLI does some checking to make sure that the config pushed will work. Should the API do further checking when someone registers a new pipeline config? Or should we just error?
- When you ask to list a pipeline or run or task run, if that thing or it's parent does not exist we should tell the user it doesn't.(This is most likely just simply passing up the not found error from the db)
- Reconstructing a timeline of any given pipeline run would be a really cool feature.
- Check that management token only with the same namespaces are able to create client tokens with those same namespaces.
- We need to address all the minor bugs around namespaces and pipelines and their existance when calling upon the api
- Github actions just downloads it's cache, we should do the same thing or make it easy for users to do so.
- Change description for models.go to reflect new thinking on shared objects
- We need to make a distinct between bot api keys and human api keys so we can know who used which, this might be useful for the metadata type.
- Possible redesign for the shepard interface is needed, right now it doesn't support cancellations and other stuff very well.
  - A possible option is to leverage the internal event system that we have and have the shepard as a "supervisor".
  - It's job will be to start threads that follow each task_execution as it attempts to run. Those task_executions will
    also produce events.
  - As events are read the supervisor will be in charge of database updates and such from those events.It will also keep
    track of the overall run state.
  - For example when all task_executions have sent a "complete" event, then the supervisor will mark the overall run as
    finished and update the database to say as much.
  - Another example would be that when we execute a "cancel" run. Each task_execution will hear that cancellation request
    and perform the container cancellation and emit a "cancelled event". The supervisor will hear the original cancel request
    and the cancelled status update from the task_executions and then mark the run as appropriately cancelled.
  - This structure allows a bit more flexbility for the tons of green threads we'll be running to track the task_executions
    without binding them all to some context object and then doing cancellations from there.
- Attach is a bit broken, it doesn't properly close the connection when the client drops(might be other too)
- Fix the "no fields updated", return better errors.
- The websockets situation can be cleaned up and better errors could be returned.

### SDK

- Rust sdk needs documentation.
- Rust documentation possibly needs to be on cargo.io.
- Rust needs more love in general, the stack rank has golang first.

### CLI

- When you schedule a job the job should tell you where your task is running. Maybe it does this through the contianer name?
- Provide custom errors at the api level so we can pass back intelligent errors to users.
- Improve CLI errors overall.
- Add command line options for controlling pagination
- Combined logs feature: Ability to attach to all task runs at the same time and dump from the run.
- We should have a watch function for run that nicely displays the task runs as they start and finish.
  (We could even have the active task_executions display the last 5 log lines as it runs each in their own personal terminal print load bar thing)
- CLI now just compiles from language. This means that we can also just straight up read from things like json/toml since it all compiles back to proto anyway.
- https://github.com/clintjedwards/gofer/commit/955e1b7da76fdfa5aa26bcb5dd0b138af605aa45
  - Reimplement this and make it so it shows the parent status.
- Create an init function for both rust and golang(simply just prompts you for your language) and then creates a new default vanilla pipeline. (similar to the old "config init" command)
- Inspiration for CLI design: https://github.com/bensadeh/circumflex
  - Look into bubble tea for some interactions.
- A diff command might be awesome.
- Expand the CLI up command to actually walk the user through the deployment using watch. Right now it just starts the deployment and walks away.
- When a user runs up, should we compare their config to known configs and reject
  registration if it's the same?
- We should allow when people build Gofer manifests to print it out in pretty json. This will allow people to write tests against Gofer manifests very easily.
- In the CLI as the user a question with a prompt like ?
- The attach command echo's back the user's input. I'm currently unsure of how to fix that.

### Scheduler

- Implement CPU/MEMORY per task values since all non-local schedulers will need this.
- It would be cool to have at least one other scheduler. Nomad is a great scheduler for this.

### SecretStore

- It would be cool to get at least one other secret store implementation like Vault.
  - For an extension like vault we manage the read and write in the same way we would for bolt. So vault gives us a prefix
    path and we essentially just used that prefix path to store secrets.

### ObjectStore

- We could probably make the default object store pretty good for trival to medium size deployments by implementing a CAS.
- Maybe there is a library we can use to do this for us? Does BoltDB do some form of deduplicaton?

### Extensions

- Extensions aren't particularly durable. If a container orchestrator moves them (to potentially make room for other things) they lose all state. Maybe we can allow extensions to use Gofer's object store such that they can persist state.
  It's possible that on Extension startup we can have it grab objects and then just return an error on the health endpoint until it's ready.
- Test that unsubscribing works with all extensions. And create a test suite that extensions can run against.
- The interval extension should create jitter of about 2-5 mins. During that time it can choose when to start counting to extension an event. This is so that when we restart the server all events don't perfectly line up with each other and cause a storm. There might be other, smarter ways to handle this queue and api calling as well.
- Extensions should follow semver. Extensions that use the same major version of Gofer should be compatible.
- If a extension by the same name is already installed, we should refuse to install another but instead allow the user to update it.
- Extensions should be able to report details about their execution somehow. It would be nice when looking at my pipeline run to see exactly when the extension performed certain actions. And be able to troubleshoot an extension that is taking overly long.
- Make sure to put in the extension documentation which versions are compatible with gofer. The current scheme is that all major versions are compatible with all same major versions. So if Gofer releases a 1.0, then all extensions will have to release a 1.0. This means that extensions can update their minor and patch versioning, but major versions will also be compatible with the same Gofer major version. Make sure this is documented.
- Github sometimes changes their payloads and this causes us to always have to be at the latest release or else casting payloads might break. Investigate payload casting and see if maybe we can get something even partial if not a better error for the user.
- Extensions probably need a healthcheck endpoint, so we can try to self heal and if not we can at least inform the user. We
  can also report things like latency and metrics from each extension via this endpoint.

#### More extensions:

There are several useful things we can do with the concept of extensions:

- There should be a way to monitor another pipeline(in any namespace) and then
  run your pipeline based on that pipeline.
- There should be a way to monitor any pipeline and then notify yourself(email, slack, whatever) on a certain cadence. Things like:
  - If pipeline fails 3 runs in a row.
  - If pipeline failure rate ever dives below certain percentage.
  - If total time of a run exceeds a given duration.
  - When a run finishes.
  - When a run fails.
  - If a particular task run fails or succeeds.

### Frontend

- SuccessRate should be tracked, we also probably can run a background job that will sleep the majority of the time and
  then run once every day or so to calculate metrics.
- On the first page a constantly updating event log would be really cool for the default namespace.

### Things I need to do but probably will never get around to

- Test registry auth.
- Write tests for all "TO" and "FROM" functions.

### General

- For FromProto methods where the reference might be nil; auto-create the reference before attempting to fill it. Look at registry auth for an example.
- Metrics via openTelemetry
- Check that when we create the run specific token for a run and enter it into the user's run stuff. We also need to make sure we clean that token up after the run is done.
- Create a container for custom use that has gofer-cli already packed in and possibly allows
  - Think about making a new task type that you can pass commands to that automatically uses the gofer container. So users can get zero to code ultra-fast.
- Improve Logging:
  - We should change extensions(and probably main?) over to use slog instead so we can get consistent logging patterns from extensions.
  - We need to refactor logging for some routes to build on top of each other so that they we automatically get things
    like namespace, pipeline.
- When first spinning up Gofer we attempt to check for a bootstrap token. To do this we must filter out any extension tokens that get automagically created now. Instead of checking if there are any tokens at all, we should instead just have a gofer metadata table and bootstrap_token_created: true.

### Rough spots in design

- It currently runs as a singleton, not distributed. There are a lot of things to figure out here for a full distributed system.
- The umbrella for this tool is large. There is a reason Jenkins still leads, the plugin ecosystem needs significant time to catch up to its large ecosystem and then to do it properly would require non-insignificant maintenance.
- Write some documentation on the Domain model design. Sometimes it can be hard to wrap your head around going from Config -> SDK -> Proto -> Models and they are all named fairly similarly.

### Public Gofer ideas

- Can we give user's a timeout that is super low, like a total container runtime of a few minutes. The only way to get past this is to sign up from a differnet IP. That way you can try it out, but you can't just run your own crypto shit on it.
- Once the timeout is up we simply log the IP and prevent that user from making any more requests.
- We might be able to get this for free in some golang ratelimiting libraries, we'd have to have the user sign up in some way first in order to prevent people from abusing. We can ratelimit routes that need to be always public per IP.
- How do we secure the running of containers? We can do somethings like preventing root user for the container: https://firecracker-microvm.github.io/

### Documentation

- Server configuration reference should have one more field on whether it is required or not.
- Extension documentation:
  - Extensions now have two required functions, extension installations and extension runs
    - Run is the service, Install runs a small program meant to help with installation.
  - How to test extensions
  - How to work with extensions locally
  - Explanation of the SDK on writing extensions
- Add a section where we create a new extension using a extension that has already been created. as the example for new extensions in the docs
- Secrets explanation. Why is there global secrets and pipelines secrets? Whats the difference.
  - Global secrets can only be set by administrators
- Write a small RFC for Gofer. Why were the decisions made the way they were, what was the purpose of the project, etc etc.
  - We are forgoing having cli spit out Json due to gofer having an API, the cli is meant for humans and shouldn't be used by programs.
- Write copius notes on commontasks and extensions layout. The difference between user passed config and system passed config. And suggest a way to collect those.
  - Gofer passes them one set of env vars from the gofer system itself
    These are prefixed with `gofer_extension_system_{var}`
  - Gofer then passes them another set of env vars from the admin that was set up through registration.
    These are prefixed with `gofer_extension_config_{var}`
  - Gofer then passes them another set of env vars from the user's own config.
    These are prefixed with `gofer_extension_param_{var}`
- Write better documentation on how to spin Gofer up locally so you can test out your pipeline.
- Add documentation for new token namespaces
- Document extension system env vars

### Testing

- integration tests
  - Test that we can retrieve a binary from the object store
  - Test that users in one namespace cannot access global secrets meant for another namespace.
  - Test that two tasks can pass things to each other via objects.
  - Test that run objects expire correctly and that they get properly marked as expired
  - Test that logs are removed correctly.
  - Test that GOFER_API_TOKEN and inject works correctly, make sure it gets cleaned up properly.

### Security

- Extensions need a lot of thinking through.
- Extensions are meant to run in containers and we allow users to pass TLS to them. But that means that extension
  writers can take your certs and ship them somewhere else. Low priority since extensions in any form requires running
  external code.
