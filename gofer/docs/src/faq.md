# FAQ

Most of these come from the same design choice: every task is its own container, possibly on a different machine,
and Gofer doesn't tie pipelines to a git repository. That keeps Gofer simple and lets it run anything, but some things
other CI/CD tools do for you implicitly you'll do explicitly here. The answers below are the patterns that work well.

### > My tasks don't share a filesystem. How do I pass files between them?

Use the [object store](./ref/object_store/index.html). It's Gofer's replacement for a shared workspace.

- **Run objects** live for a single run (plus a few after it, see `run_object_expiry`). Use them for handing a value
  or a file from one task to the tasks after it.
- **Pipeline objects** last across runs until the pipeline hits `pipeline_object_limit`. Use them for caches.

A task stores objects through Gofer's API, so give it `InjectAPIToken(true)`. Gofer sets `GOFER_API_BASE_URL`,
`GOFER_NAMESPACE`, `GOFER_PIPELINE_ID` and `GOFER_RUN_ID` on every task, so the request needs no other setup:

```bash
# Upload a directory as a run object.
tar -czf - ./dist | curl -sf -X POST \
  -H "Authorization: Bearer $GOFER_TOKEN" -H "gofer-api-version: v0" --data-binary @- \
  "$GOFER_API_BASE_URL/api/namespaces/$GOFER_NAMESPACE/pipelines/$GOFER_PIPELINE_ID/runs/$GOFER_RUN_ID/objects/dist"

# Download it in a later task.
curl -sf -H "Authorization: Bearer $GOFER_TOKEN" -H "gofer-api-version: v0" \
  "$GOFER_API_BASE_URL/api/namespaces/$GOFER_NAMESPACE/pipelines/$GOFER_PIPELINE_ID/runs/$GOFER_RUN_ID/objects/dist" \
  | tar -xzf -
```

If your image has the `gofer` binary, `gofer run object put $GOFER_PIPELINE_ID $GOFER_RUN_ID dist @` and
`gofer run object get ...` do the same thing.

Two things trip people up:

- **The reading task has to depend on the writing one** with `DependsOn`. Tasks run in parallel by default, so
  without it the object might not exist yet.
- **Download files through the API, not as variables.** `sdk.RunObject("key")` works well for small text values, but
  it converts the object to text, so it'll mangle a tarball or any other binary file.

For small values, see [passing data between tasks](./ref/object_store/index.html#passing-data-between-tasks).

### > How do I work with a git repository? Other CI/CD tools clone it for me.

Gofer doesn't know about your repository, so your tasks clone it themselves. That's two problems: getting access to
the repo, and getting the code into each task that needs it.

#### Getting access

**Use a read-only deploy key per repository, stored as a pipeline secret.** Deploy keys are scoped to a single repo,
so a leaked key only exposes that repo.

```bash
ssh-keygen -t ed25519 -N "" -f ./deploy_key       # Add deploy_key.pub to the repo as a read-only deploy key.
gofer secret pipeline put my-pipeline deploy_key < ./deploy_key
rm ./deploy_key
```

Then hand it to the task as a variable and point git at it:

```go
const setupGit = `
apk add --no-cache git openssh-client
mkdir -p ~/.ssh
printf '%s\n' "$DEPLOY_KEY" > ~/.ssh/deploy_key   # Secrets are stored without the trailing newline ssh requires.
chmod 600 ~/.ssh/deploy_key
printf '%s\n' "$KNOWN_HOSTS" > ~/.ssh/known_hosts
export GIT_SSH_COMMAND="ssh -i ~/.ssh/deploy_key -o UserKnownHostsFile=$HOME/.ssh/known_hosts"
`

sdk.NewTask("build", "alpine:latest").
    Variables(map[string]string{
        "DEPLOY_KEY":  sdk.PipelineSecret("deploy_key"),
        "KNOWN_HOSTS": sdk.PipelineObject("known_hosts"),
    }).
    Command("sh", "-c", setupGit+"git clone git@github.com:my-org/my-repo.git")
```

**Pin the host's SSH keys instead of turning off host checking.** It's tempting to add
`-o StrictHostKeyChecking=no`, but then anyone who can intercept the connection can feed your task whatever code
they like. Copy the host's published keys (Github lists theirs in its docs under "Github's SSH key fingerprints")
into a pipeline object once, as above, or bake them into your image:

```bash
gofer pipeline object put my-pipeline known_hosts ./github_known_hosts
```

**To share a key across pipelines**, store it as a global secret and limit which namespaces can use it. This is the
way to give one team's pipelines access without giving it to everyone:

```bash
gofer secret global put team_a_deploy_key -n "team-a-.*" < ./deploy_key
```

Then use `sdk.GlobalSecret("team_a_deploy_key")` in place of `sdk.PipelineSecret`. Keep shared keys read-only, since
every pipeline in those namespaces can read them.

**HTTPS with a token works too**, and skips the SSH setup:
`git clone https://x-access-token:$GITHUB_TOKEN@github.com/my-org/my-repo.git`. The tradeoff is that personal access
tokens are usually tied to someone's account, so they break when that person leaves and often reach more repos than
the pipeline needs. Prefer fine-grained tokens limited to one repo.

#### Getting the code

Start with the first option and only move down the list when it's actually too slow.

**1. Fetch just the commit you need.** This is fast enough for most repositories and it's the most correct option:
you build exactly the commit that triggered the run, not whatever the branch points to by the time the task starts.
The [Github extension](./ref/extensions/provided/github.md#events) passes that commit in as
`GOFER_EXTENSION_GITHUB_PULLREQUEST_HEAD_SHA` for pull requests and `GOFER_EXTENSION_GITHUB_HEAD_COMMIT_ID` for pushes.

```bash
git init -q repo && cd repo
git remote add origin git@github.com:my-org/my-repo.git
git fetch -q --depth 1 origin "$GOFER_EXTENSION_GITHUB_HEAD_COMMIT_ID"
git checkout -q FETCH_HEAD
```

For large repos, add `--filter=blob:none` to download file contents only as they're needed, or use
`git sparse-checkout` to check out only the directories the task uses. Fetching a commit by its hash works on Github
and Gitlab; some self-hosted git servers need `uploadpack.allowReachableSHA1InWant` turned on first.

**2. Clone once per run and share it.** When several tasks need the code, have one task fetch it and upload it as a
run object, then have the others depend on that task and download it, using the commands from the
[file sharing answer](#my-tasks-dont-share-a-filesystem-how-do-i-pass-files-between-them) above. Every task sees
the same tree and your git host only gets hit once per run.

**3. Cache the repository across runs.** Keep a tarball of the repo as a pipeline object. Each run downloads it,
runs `git fetch` to pick up only what changed, then uploads it again. A few things to know:

- Upload over the old one by adding `?force=true` to the request URL, or `--force` with the CLI.
- If two runs update the cache at the same time, the last one wins. That's fine for a cache, but don't keep anything
  there you can't rebuild.
- Pipeline objects past `pipeline_object_limit` get deleted, oldest first, so write the clone task to fall back to a
  fresh clone when the cache is missing.

**4. Bake it into the image.** For repos that rarely change, like shared scripts or tooling, copying them into the
task's image at build time skips the download entirely.

**5. Run a git caching proxy.** For very large repos, or many runs close together, a caching proxy near your
container hosts (like [git-cache-http-server](https://github.com/jonasmalacofilho/git-cache-http-server)) serves
clones locally and only goes back to the real host for new commits.
