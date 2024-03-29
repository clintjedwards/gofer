# FAQ

### > I have a job that works with a remote git repository, other CI/CD tools make this trivial, how do I mimic that?

The drawback of this model and architecture is does not specifically cater to GitOps. So certain workflows
that come out of the box from other CI/CD tooling will need to be recreated, due to its inherently distributed nature.

Gofer has provided several tooling options to help with this.

There are two problems that need to be solved around the managing of git repositories for a pipeline:

#### 1) How do I authenticate to my source control repository?

Good security practice suggests that you should be managing repository deploy keys, per repository, per team. You can
potentially forgo the "per team" suggestion using a "read-only" key and the scope of things using the key
isn't too big.

Gofer's suggestion here is to make deploy keys self service and then simply enter them into Gofer's secret store to be used by your pipeline's tasks. Once there you can then use it in each job to pull the required repository.

#### 2) How do I download the repository?

Three strategies:

1. Just download it when you need it. Depending on the size of your repository and the frequency of the pull, this can work absolutely fine.
2. Use the object store as a cache. Gofer provides an object store to act as a permanent (pipeline-level) or short-lived
   (run-level) cache for your workloads. Simply store the repository inside the object store and pull down per job
   as needed.
3. Download it as you need it using a local caching git server. Once your repository starts becoming large or you do many
   pulls quickly it might make more sense to use a cache[^1],[^2]. It also makes sense to only download what you
   need using git tools like `sparse checkout`

[^1]: https://github.com/google/goblet
[^2]: https://github.com/jonasmalacofilho/git-cache-http-server
