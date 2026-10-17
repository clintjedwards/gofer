# Working with Git Repositories

Gofer doesn't know about your repository, so your tasks clone it themselves. That's two problems: getting access to
the repo, and getting the code into each task that needs it.

## Getting access

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

**To share a key across pipelines**, store it as a global secret limited to the namespaces that need it, as in
[prefer pipeline secrets over global ones](./secrets_and_access.md#prefer-pipeline-secrets-over-global-ones), and use
`sdk.GlobalSecret("team_a_deploy_key")` in place of `sdk.PipelineSecret`. Keep shared keys read-only, since every
pipeline in those namespaces can read them.

**HTTPS with a token works too**, and skips the SSH setup:
`git clone https://x-access-token:$GITHUB_TOKEN@github.com/my-org/my-repo.git`. The tradeoff is that personal access
tokens are usually tied to someone's account, so they break when that person leaves and often reach more repos than
the pipeline needs. Prefer fine-grained tokens limited to one repo.

## Getting the code

Start with the first option and only move down the list when it's actually too slow.

**1. Fetch just the commit you need.** This is fast enough for most repositories and it's the most correct option:
you build exactly the commit that triggered the run, not whatever the branch points to by the time the task starts.
The [Github extension](../../ref/extensions/provided/github.md#events) passes that commit in as
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
run object, then have the others depend on that task and download it, as shown in
[files and directories](./passing_data_between_tasks.md#files-and-directories). Every task sees the same tree and your
git host only gets hit once per run.

**3. Cache the repository across runs.** Keep a tarball of the repo as a pipeline object. Each run downloads it,
runs `git fetch` to pick up only what changed, then uploads it again. A few things to know:

- Upload over the old one by adding `?force=true` to the request URL, or `--force` with the CLI.
- If two runs update the cache at the same time, the last one wins. That's fine for a cache, but don't keep anything
  there you can't rebuild.
  [`Parallelism(1)`](./designing_pipelines.md#limit-pipelines-that-cant-overlap) avoids it entirely.
- Pipeline objects past `pipeline_object_limit` get deleted, oldest first, so write the clone task to fall back to a
  fresh clone when the cache is missing.

**4. Bake it into the image.** For repos that rarely change, like shared scripts or tooling, copying them into the
task's image at build time skips the download entirely.

**5. Run a git caching proxy.** For very large repos, or many runs close together, a caching proxy near your
container hosts (like [git-cache-http-server](https://github.com/jonasmalacofilho/git-cache-http-server)) serves
clones locally and only goes back to the real host for new commits.
