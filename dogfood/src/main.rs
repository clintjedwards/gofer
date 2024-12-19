use gofer_sdk::config::{Pipeline, Task};

fn main() {
    Pipeline::new("repo_container_builder", "Repo Container Builder")
        .description("Since Gofer has no concept of persistent volumes, to speed up runs and conserve bandwidth \
this pipeline is set up to build a new container with the Gofer codebase inside at the latest commit.\
        
This allows downstream Gofer jobs that need access to the repository to simply use the 'latest' of the
built container and not have to download the entire repository on every run.
        
This job in it's current form is very basic. If we truly wanted to cut down on build times and bandwidth \
we could use the previous repository from the last build and then fetch the new commits from that. Due to how \
docker caching works that would allow us to get this pipeline's build time down so far as to update the \
containermultiple times per day. 

    ")
        .tasks(vec![Task::new("build_repo_container", "docker:latest")]).finish().unwrap();
}
