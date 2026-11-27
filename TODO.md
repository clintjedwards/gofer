# Large Projects on the docket

These come after the built images refactor is done.

* The `gofer` CLI inside every task, so together with the injected token and the run folder a task can work with
  Gofer without any setup.
  * The server mounts its own binary into each task's container when it starts it, the way Docker's `--init` does,
    instead of building it into images. That way it always matches the server, works with every image, old ones and
    extensions included, and doesn't change anyone's image.
  * It needs a static build. The current binary needs glibc, so it won't start on Alpine or distroless images.
  * Nothing can require it, so task code still doesn't need to know about Gofer.
  * This would cover the idea under General of a container that has the CLI packed in.
* Extensions that don't come from a registry. Today an extension is still an image Gofer pulls, with its own
  `registry_auth`, while tasks run images Gofer stores itself.
  * At the least, the extensions that come with Gofer (cron, interval, github) should ship with it, so starting them
    never pulls anything, works without the network, and always matches the server's version. They could be built
    into the release and loaded from Gofer's own image store, the same way tasks are.
  * Maybe every extension: one is installed by uploading its image through the same blob routes `gofer up` uses, and
    then runs like a very long task. That would make extension `registry_auth` go away too. Open questions: who
    uploads it (an admin with the CLI, from a Dockerfile or from their local Docker), and how upgrading one works.
  * If neither turns out worth it, extensions keep pulling from registries as they do now.

# Small things I want to keep track of that I definitely need to do.

* Let extension manifests declare the additional roles an extension needs, instead of operators having to know and
  list them in `additional_roles`. Could also be a way to permission extensions per pipeline or per purpose. Needs a
  design session.
* Pin extension images by digest in manifests (`image@sha256:...`) instead of by tag. Gofer pulls on every start, so
  if someone re-pushes a tag the extension's code changes without the manifest changing and without reload showing
  anything. With digests, "the manifest didn't change" really means "the code didn't change". The SDKs' manifest
  command would need the digest, so this probably happens after the image is pushed in the release flow.
* Build the provided extensions against the in-repo Go SDK with a `replace` directive, like examplePipelines already
  does. Right now a breaking SDK change means pushing main, then running `go get ...@main` in each extension and
  committing again before the release can be tagged. With `replace` it's one commit. Things to fix or accept if we do:
  * The Docker build context has to move from `containers/` to the repo root so `sdk/go` is visible. Needs a root
    `.dockerignore` (target/, .git, node_modules) and Dockerfiles that copy only `sdk/go` and their own extension dir
    instead of `ADD .`, or builds get slow and the layer cache breaks on any repo change.
  * Images can pick up uncommitted SDK edits since builds use the working tree. Add a clean tree check
    (`git status --porcelain`) to `build-containers`.
  * Nothing exercises the published SDK anymore, so something that only breaks for outside consumers (like an
    uncommitted SDK file) goes unnoticed. Could cover this with a release check that builds one extension with the
    replace dropped (`go mod edit -dropreplace` on a temp copy).
  * We stop feeling breaking SDK changes the way third party extension authors do, since our extensions never break.
  * The extensions stop working as copy and paste templates; the replace path breaks outside the repo. Docs that
    point at them as examples would need to say to drop that line.
  * The `require` version in each go.mod goes stale and is misleading.
  * `go install .../containers/extensions/<ext>@latest` stops working since Go refuses modules with replace
    directives. Probably nobody does this.
  * champagne/run_tests has the same problem on the Rust side (`gofer_sdk` from git on `branch = "main"`); a path
    dependency would fix it.
* Buffer external events (webhooks) for extensions that are down, e.g. while one restarts during a reload. Keep the
  last N requests per extension with the time they arrived and hand them over when the extension starts, letting the
  extension decide what to do with them. That covers webhook driven extensions like github. Time based extensions
  (cron, interval) are harder since there's nothing for Gofer to store; they'd need their own way to notice and
  handle what they missed while down.
* Add `gofer service drain` for admins, for safely stopping Gofer before an upgrade or restart.
  * It turns on `ignore_pipeline_run_events` through `PATCH /api/system`, which refuses new runs and holds queued
    ones, then polls until no runs are left running and it's safe to stop Gofer.
  * `--no-wait` turns on draining and returns right away, for scripts or when you just want to stop new work.
  * Ctrl-C while it's watching should only stop the watching; the server stays drained. Say so when it exits.
  * Needs a way to turn it back off; `gofer service undrain` is the obvious name. Since the setting is stored in the
    database it survives a restart, so after an upgrade Gofer comes back up still refusing runs until someone undrains
    it. Print that clearly at the end of `drain`, and probably log a warning at startup when Gofer starts drained.
  * The missing piece is a list of running runs across all pipelines. The only run list today is per pipeline and
    can't filter by state, so polling it means walking every namespace and pipeline. Add an admin-only endpoint that
    returns the running runs and how many are queued; the `list_running` storage query already exists.
    * Poll it every couple of seconds. For each running run, the existing `/runs/{run_id}/tasks` endpoint gives its
      progress (tasks done out of total, and the task currently running). Print a line when something changes
      rather than redrawing a table, so it reads the same in a terminal or a CI log.
    * Drain only has to wait on running runs, but show how many are queued, since they all start as soon as Gofer is
      undrained.
    * Watching the event stream (`/api/events`) instead would show progress the moment it happens, but it still
      needs the running-runs list to start from, has to subscribe before fetching that list or drain can wait
      forever, and has to resync if the connection drops. A couple of seconds of delay isn't worth that.
  * `gofer context` (and the web UI) should show when Gofer is drained, since a drained server looks a lot like a
    broken one from a pipeline owner's point of view. `gofer context` already shows "not accepting new runs", but
    only to admins since `GET /api/system` is admin only; non-admins need some way to see it too.
  * Document it in the server configuration docs as the way to stop new work before an upgrade.
* Allow a parallelism mode where when parallelism is at it's max the oldest run, if still running gets, cancelled.
  * Also make it so that the github extension can do this as well, if a new run for a branch gets kicked off, if there
    is already a run for that branch, cancel the ongoing one and trigger a new one.
* Transition dropshot to use the new trait api. Which will eliminate the circular dependency on openapi files.
* Canaried deployments feature.
* Make run execution survive failures. Today one failed call to Docker ends the run, and running containers anywhere
  but local Docker would make those failures more common.
  * In monitor_task_execution, calls to Docker to check on container status are expected to succeed. If one fails the
    whole thing is aborted. It should retry with backoff and only give up after a while.
  * The recover_run function needs to account for the fact that sometimes the event_ids mentioned within runs might
    not exist anymore. It should also not return any errors but instead log them and move on, trying its best
    despite any failures.

# Small things I'll probably never get around to.

* Change the poll interval for pruneing events to be based on the number of events processed rather than just a standard
  static prune time. Maybe we can just make it a big LRU.
* Events can be lost when the single sqlite write connection is busy. `EventBus::publish` only broadcasts an event after
  it's saved, and saving waits on the same write connection everything else uses. If that wait passes sqlx's 30 second
  acquire timeout the event is logged and dropped, and a dropped `CompletedTaskExecution` leaves its run stuck in
  `running` forever. Since everything depends on events we don't want to just broadcast unsaved ones; giving the event
  log its own sqlite database, so it never waits behind other writes, is probably the fix.
* Stop holding the sqlite write connection across slow work. There's only one, so anything slow done while holding it
  stalls every write in Gofer, event publishing included. For example `launch_new_run` keeps its `BEGIN IMMEDIATE`
  transaction open across `secret_store.put`. Worth going through the other `write_conn()` callers for the same thing.
* Dropshot has implemented a trait API which would speed up compliation times and overall lead to more maintainable
code. Right now it doesn't quite work due to the main api trait being too large. (We'd have to write all the handlers
in one very large file or split them up). https://github.com/oxidecomputer/dropshot/issues/1069 should fix this.
* Extension `registry_auth` hasn't been tested against a real private registry yet.
* Write/Design a way to clean up expired tokens after long enough.
* Check that our websockets stuff makes sense we use joinset, make sure we're returning errors to the main thread and
bubbling them up properly.
* If Gofer ever runs containers on more than one machine, note which machine each task ran on.
* If Gofer ever runs containers on more than one machine, a run's tasks have to run on the same one, since they share
  the run folder. Tekton does this for tasks that share a workspace, with what it calls an affinity assistant. The cost
  is that a run can only use one machine, and if that machine dies the run's files go with it.
* Pass back custom errors via the API so that consumers can understand what has happened.
* Pagination...everywhere.
* The CLI should have a feature where you can start a pipeline and follow all logs and status updates
from that pipeline in one place. Maybe this is a watch feature where each task reports the task 5 log lines until it
finishes at which time it reflects a summary about what it did.
* When using the SDK to build a pipeline, that pipeline should print to stdout the json that will be collected

# The floor: Stuff I put things I probably should do but haven't prioritized/sorted yet.

### Built images

- `gofer up` inside a Gofer task. Tasks that are only a base image already work there, since they don't need Docker.
  The rest need a BuildKit to talk to, like a remote one that `gofer up` reaches over the network instead of starting
  its own container.
- Copying files in without Docker. A layer that adds `/app/file` also decides what `/app` itself is, so doing it
  safely means reading the base's own folders first: their owners, modes, and symlinks like `/bin` to `/usr/bin`.
- ARM and mixed platforms. Images are always linux/amd64 today. A platform override per task would cover ARM servers,
  and if Gofer becomes distributed, each machine might need its own.
- Identity tokens for tasks that are only a base image. Those are downloaded with oci-client, which can only trade a
  username and password for a registry token, so a login that's an identity token (some registries hand these out in
  place of a password) only works for tasks built with BuildKit.
- Replacing Docker. The server only needs to load images and run containers, and the CLI only needs a BuildKit, so
  neither has to be Docker forever.
- Image cleanup can delete a blob that was just uploaded. `remove_unless_newer` in `images.rs` checks a file's time
  and then deletes it, and an upload can rename a fresh copy into place between the two. The upload still gets a 204,
  and registering fails until the next `gofer up` uploads the blob again. Renaming the file aside, checking its time
  and `is_used` again, and only then deleting it would close the gap.
- Loading a task's image into Docker shares `docker.timeout_seconds` (300s by default) with every other Docker call,
  and Docker only answers `/images/load` once it has read the whole image. So a big image on a slow disk can fail its
  task. Loads could get a client with a much longer timeout of their own, or none.
- Upload blobs in parallel. `gofer up` sends them one at a time, which is slow when a pipeline's first upload is a
  few large layers.

### SecretStore

- It would be cool to get at least one other secret store implementation like Vault.
  - For an extension like vault we manage the read and write in the same way we would for bolt. So vault gives us a
    prefix path and we essentially just used that prefix path to store secrets.

### ObjectStore

- We could probably make the default object store pretty good for trival to medium size deployments by implementing a
  CAS.

### Extensions

- Test that unsubscribing works with all extensions. And create a test suite that extensions can run against.
- The interval extension should create jitter of about 2-5 mins. During that time it can choose when to start counting
  to extension an event. This is so that when we restart the server all events don't perfectly line up with each other
  and cause a storm. There might be other, smarter ways to handle this queue and api calling as well.
- Extensions should be able to report details about their execution somehow. It would be nice when looking at my
  pipeline run to see exactly when the extension performed certain actions. And be able to troubleshoot an extension
  that is taking overly long.
- Github sometimes changes their payloads and this causes us to always have to be at the latest release or else casting
  payloads might break. Investigate payload casting and see if maybe we can get something even partial if not a better
  error for the user.

#### More extensions:

There are several useful things we can do with the concept of extensions:

- There should be a way to monitor another pipeline(in any namespace) and then
  run your pipeline based on that pipeline.
- There should be a way to monitor any pipeline and then notify yourself(email, slack, whatever) on a certain cadence.
  Things like:
  - If pipeline fails 3 runs in a row.
  - If pipeline failure rate ever dives below certain percentage.
  - If total time of a run exceeds a given duration.
  - When a run finishes.
  - When a run fails.
  - If a particular task run fails or succeeds.

### Frontend

- Cut down how much each UI refresh asks for. The overview page makes up to two requests per pipeline
  (`runs?since=...&limit=500` and, for quiet pipelines, `runs?limit=20`) every 5 seconds while a run is in progress and
  every 30 seconds otherwise, almost all returning the same thing. Roughly from cheapest to most work:
  - Only ask for what changed. Either the client asks for runs newer than the last one it has, or the server returns
    an ETag and the browser gets a small `304 Not Modified` when nothing changed. The `limit=500` request is the one
    worth fixing first.
  - Have the server push updates instead (SSE, websockets, or long polling) so the page only hears about real
    changes. Most work, since the server has to track listeners and the client has to handle reconnects, but the event
    stream (`/api/events`) already has `StartedRun`, `CompletedRun` and the task events, so it might be most of the way
    there.

### General

- Let tasks set CPU and memory limits.
- Limit how big a run folder can get. Docker's local volumes can't, so a run that writes too much fills the disk Docker
  keeps its data on. It only lasts as long as the run, but a few big runs at once could still do it. Filesystem quotas
  (like XFS project quotas) or checking the folder's size while the run goes are the options.
- Metrics via openTelemetry
- Create a container for custom use that has gofer-cli already packed in and possibly allows
  - Think about making a new task type that you can pass commands to that automatically uses the gofer container. So
    users can get zero to code ultra-fast.
- Improve Logging:
  - We should change extensions(and probably main?) over to use slog instead so we can get consistent logging patterns
    from extensions.
  - We need to refactor logging for some routes to build on top of each other so that they we automatically get things
    like namespace, pipeline.

### Rough spots in design

- It currently runs as a singleton, not distributed. There are a lot of things to figure out here for a full distributed
  system.
- The umbrella for this tool is large. There is a reason Jenkins still leads, the plugin ecosystem needs significant
  time to catch up to its large ecosystem and then to do it properly would require non-insignificant maintenance.

### Public Gofer ideas

- Can we give user's a timeout that is super low, like a total container runtime of a few minutes. The only way to get
  past this is to sign up from a differnet IP. That way you can try it out, but you can't just run your own crypto shit
  on it.
- Once the timeout is up we simply log the IP and prevent that user from making any more requests.
- We might be able to get this for free in some golang ratelimiting libraries, we'd have to have the user sign up in
  some way first in order to prevent people from abusing. We can ratelimit routes that need to be always public per IP.
- How do we secure the running of containers? We can do somethings like preventing root user for the container:
  https://firecracker-microvm.github.io/

### Documentation

- Server configuration reference should have one more field on whether it is required or not.
- Extension documentation:
  - Extensions now have two required functions, extension installations and extension runs
    - Run is the service, Install runs a small program meant to help with installation.
  - How to test extensions
  - How to work with extensions locally
  - Explanation of the SDK on writing extensions
- Add a section where we create a new extension using a extension that has already been created. as the example for new
  extensions in the docs
- Secrets explanation. Why is there global secrets and pipelines secrets? Whats the difference.
  - Global secrets can only be set by administrators
- Write a small RFC for Gofer. Why were the decisions made the way they were, what was the purpose of the project, etc
  etc.
  - We are forgoing having cli spit out Json due to gofer having an API, the cli is meant for humans and shouldn't be
    used by programs.
- Write copius notes on commontasks and extensions layout. The difference between user passed config and system passed
  config. And suggest a way to collect those.
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
  - Test that two tasks can pass things to each other through the run folder, including a task that runs as another
    user.
  - Test that a run's folder and task containers are removed when it ends, and by the sweep when a crash left them.
  - Test that logs are removed correctly.
  - Test that GOFER_TOKEN and inject works correctly, make sure it gets cleaned up properly.

### Security

- Extensions need a lot of thinking through.
- Extensions are meant to run in containers and we allow users to pass TLS to them. But that means that extension
  writers can take your certs and ship them somewhere else. Low priority since extensions in any form requires running
  external code.
