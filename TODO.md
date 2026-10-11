# Large Projects on the docket

None

# Small things I want to keep track of that I definitely need to do.

* Let extension manifests declare the additional roles an extension needs, instead of operators having to know and
  list them in `additional_roles`. Could also be a way to permission extensions per pipeline or per purpose. Needs a
  design session.
* Pin extension images by digest in manifests (`image@sha256:...`) instead of by tag. Gofer pulls on every start, so
  if someone re-pushes a tag the extension's code changes without the manifest changing and without reload showing
  anything. With digests, "the manifest didn't change" really means "the code didn't change". The SDKs' manifest
  command would need the digest, so this probably happens after the image is pushed in the release flow.
* Follow up: pipelines aren't told when an extension they're subscribed to stops (disabled, unconfigured, or failed to
  start). The subscriptions stay but nothing triggers them, so a pipeline owner just sees runs that never happen.
  Showing the extension's state next to a pipeline's subscriptions might be a quick fix; worth checking whether
  there's more to it (events, notifying the pipeline, what the web UI shows).
* Buffer external events (webhooks) for extensions that are down, e.g. while one restarts during a reload. Keep the
  last N requests per extension with the time they arrived and hand them over when the extension starts, letting the
  extension decide what to do with them. That covers webhook driven extensions like github. Time based extensions
  (cron, interval) are harder since there's nothing for Gofer to store; they'd need their own way to notice and
  handle what they missed while down.
* Allow a parallelism mode where when parallelism is at it's max the oldest run, if still running gets, cancelled.
  * Also make it so that the github extension can do this as well, if a new run for a branch gets kicked off, if there
    is already a run for that branch, cancel the ongoing one and trigger a new one.
* We should allow the ops side to somehow set the GOFER_API_BASE_URL for the containers. This can change based on
where the container is running.
* When you insert a new pipeline it should show you a diff on what you're changing.
* We need to productionalize and offer the container that builds repo containers. The test for this is build_repo_container
  directory. Think more about how the UX should be handled here.
* Pipeline configs when they are registered need to be hashed, so that we can make sure the user didn't mistakenly
try to register the same thing twice.
* Transition dropshot to use the new trait api. Which will eliminate the circular dependency on openapi files.
* Canaried deployments feature.
* There should probably be a global timeout for all runs.
* Update requests that don't actually change anything return errors instead of simply telling the user nothing changed.
* The final piece of the run shepard needs to implement a run queue to fully transition over to event driven.
  It should use task leasing to avoid any stuck processors.

# Small things I'll probably never get around to.

* Change the poll interval for pruneing events to be based on the number of events processed rather than just a standard
  static prune time. Maybe we can just make it a big LRU.
* The recover_run function needs to account for the fact that sometimes the event_id that are mentioned within runs
  might not exist anymore. This function should also not return any errors but instead just log them and move on. It
  should try its best despite any failures.
* Change the sqlite write_pool to be guarded by a mutex. This would avoid very obvious errors in code that might lead
  to deadlocks during runtime.
* Dropshot has implemented a trait API which would speed up compliation times and overall lead to more maintainable
code. Right now it doesn't quite work due to the main api trait being too large. (We'd have to write all the handlers
in one very large file or split them up). https://github.com/oxidecomputer/dropshot/issues/1069 should fix this.
* Registry auth is largely untested and possibly unsecured, don't use it for anything serious.
* Write/Design a way to clean up expired tokens after long enough.
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
* In monitor_task_execution calls to the scheduler to check on container status are expected to succeed. If they fail
the whole thing is aborted, which is obviously bad because when we implement networked schedulers network calls will fail
sometimes.

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

### Extensions

- Test that unsubscribing works with all extensions. And create a test suite that extensions can run against.
- The interval extension should create jitter of about 2-5 mins. During that time it can choose when to start counting to extension an event. This is so that when we restart the server all events don't perfectly line up with each other and cause a storm. There might be other, smarter ways to handle this queue and api calling as well.
- Extensions should be able to report details about their execution somehow. It would be nice when looking at my pipeline run to see exactly when the extension performed certain actions. And be able to troubleshoot an extension that is taking overly long.
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

### General

- Metrics via openTelemetry
- Create a container for custom use that has gofer-cli already packed in and possibly allows
  - Think about making a new task type that you can pass commands to that automatically uses the gofer container. So users can get zero to code ultra-fast.
- Improve Logging:
  - We should change extensions(and probably main?) over to use slog instead so we can get consistent logging patterns from extensions.
  - We need to refactor logging for some routes to build on top of each other so that they we automatically get things
    like namespace, pipeline.

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
