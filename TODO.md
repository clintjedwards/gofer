# Large Projects on the dockett

## Extensions should have object store access.

Extensions might need object store access due to the fact that they don't keep any state. Right now what we do is
we keep the state that we need to track subscriptions inside Gofer and on startup we have Gofer repopulate the
extension with those subscriptions. But this is short-sighted as Gofer tries to restore subscriptions as fast as possible
and has no idea what may be good or bad for the extension itself.

Instead we should give the extension access to the object store such that it can save it's record of subscriptions
and then on startup it can decide how it should handle things.

The original example for why this is needed is the interval example. If all subscriptions were restored at the same time
then the interval would start for each pipeline at the same time. Causing possible horrible thundering herd issues. Instead
it would be better for the interval extension to reach out and restore it's own subscriptions at a pace that it can
determine and in what order it wants to. This allows the interval to introduce jitter and other algorithms to better
spread the load.

## Github Extension followthrough

A great feature to bake into the Github extension would be the ability for it to act as a communicator with the
Github PR in question, if possible. This mirrors how other thingdoer tooling handles github.

It might also be possible to bake in repo management. The extension would use its extension object store permissions
(also a project that needs to be completed) and utilize that to give user's access to a repo, allowing it to have
a local cache that could possible be faster.

## Better debugging tooling.

A huge problem with thingdoers is it's hard to debug because you can't really run the code locally in most situations.
Because of this we should give the user as many tools as we can to make sure they can debug on the fly.

* Make sure attach works correctly.
* The CLI should output a debug command that dumps all the logs from a particular task.
* The CLI should output a debug command that dumps all the info from the run in general.
* Find other ways we can debug and make the user's life easier in this regard. It's possible that if we put a lot of
thought into this feature that it can become a game changer for Gofer as a whole.
* Maybe create a timeline on when each task execution happened for a particular run?

## Event loop refactor

Right now we use a midpoint of events and memory sharing to determine how to run each run/task execution. This was a
holdover from the Gofer version of the application. Instead we should focus more on an event driven design. We already
have our event processor, now we just need to make sure its the main driving force for how task executions are handled.

This would allow us to do previously complicated things like cancellations.

* The Shepard interface simply starts task executions and follows their progress.
* It emits events as executions hit certain milestones.
* It also listens for events, translates database updates, and responds to global run events.

# Small things I want to keep track of that I definitely need to do.

## API

* There needs to be a way to update extensions in place so that updating versions of extensions can be done online.
* Minify CSS when we release for frontend.
* Write a test for bootstrap tokens in hurl.
* Write a test for tokens, namespace matching, and general auth in hurl.
* Make sure to finish the implementation of Gofer run tokens. We started it but haven't quite checked all the boxes
yet.
  * Make sure to set the namespace for the user automagically.
* Allow users to query deployments from the CLI.
* Make sure is_valid_identifier is used in all the places where the user has to enter an id.
* Transition dropshot to use the new trait api. Which will eliminate the circular dependency on openapi files.
* We need to make sure that if Gofer crashes it understands how to restore the jobs that were running previously.
  * Purely for recovering lost containers, we can usually query the engine to ask it what time this container stopped
  and started. This way we can have more accurate running times instead of just leaving it as whenever the server
  restarted.
* We want to restrict the max size of the request body, but some endpoints need large bodies to upload things to us.
  We should get rid of the global restriction and instead check for what the request size should be in the preflight.
* Make the object store uploads multipart.
* Canaried deployments feature.
* There should probably be a global timeout for all runs.
* Create a setting to allow operators to turn off the ability to attach to a container.
* If the parent does not exist for a particular thing it errors incorrectly. For example if you request a correct task
execution but mistype the pipeline, you might get an error instead of a "hey that thing doesn't exist".
* Management tokens with certain namespaces should only be able to create client tokens with those same namespaces.
* Update requests that don't actually change anything return errors instead of simply telling the user nothing changed.
* By default docker doesn't allow you to do versioning tricks like pinning to a major version but freely updating the minor
version. I wonder if there is a way we can offer this feature for free for the purposes of extensions. Since extensions with
the same major version should work, but extensions might all have different minor versions, it would be useful to be able
to tell Gofer to use a major version of the extension but we always want the latest minor version.


# Small things I'll probably never get around to.

* Registry auth is largely untested and possibly unsecured, don't use it for anything serious.
* Write/Design a way to clean up expired tokens after long enough.
* When a user removes an extension we should remove the subscriptions also (check we don't already do this,
  due to cascade delete).
* Check that our websockets stuff makes sense we use joinset, make sure we're returning errors to the main thread and
bubbling them up properly.
* API needs validation for all endpoints.
* User should be able to give their builds timeouts and we need to establish a global timeout.
* When you schedule a job on a container orch, we should note where that job has run(which node).
* Pass back custom errors via the API so that consumers can understand what has happened.
* Pagination...everywhere.
* The CLI should have a feature where you can start a pipeline and follow all logs and status updates
from that pipeline in one place. Maybe this is a watch feature where each task reports the task 5 log lines until it
finishes at which time it reflects a summary about what it did.
* The CLI could have a diff command so we know exactly what is about the change from the last pipeline version.
* When using the SDK to build a pipeline, that pipeline should print to stdout the json that will be collected
* The attach prompt currently echos back user input, unsure how to fix that.


# The floor: Stuff I put things I probably should do but haven't prioritized/sorted yet.


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
