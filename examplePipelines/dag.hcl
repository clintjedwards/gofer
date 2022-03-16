id          = "dag_test_pipeline"
name        = "[dag] Gofer Test Pipeline"
description = <<EOT
This pipeline shows off how you might use Gofer's DAG(Directed Acyclic Graph) system to chain together containers
that depend on other container's end states. This is obviously very useful if you want to perform certain trees
of actions depending on what happens in earlier containers.
EOT

// Tasks are the building blocks of a pipeline. They represent individual containers and can be
// configured to depend on one or multiple other tasks.
task "task_one" "ghcr.io/clintjedwards/experimental:wait" {
  description = "This task has no dependencies so it will run immediately"

  // Environment variables are the way in which your container is configured.
  // These are passed into your container at the time it is ran.
  env_vars = {
    "WAIT_DURATION" : "20s",
  }
}

task "depends_on_one" "ghcr.io/clintjedwards/experimental:log" {
  description = <<EOT
This task depends on the first task to finish with a successfull result. This means
that if the first task fails this task will not run.
EOT
  depends_on = {
    "task_one" : "successful",
  }
  env_vars = {
    "LOGS_HEADER" : "This string can be anything you want it to be",
  }
}

// depends_on_two is the last in line to be run since it's dependency tree looks like:
// "task_one" -> "depends_on_one" -> "depends_on_two".
// It's only difference is that regardless of the state that "depends_on_one" ends with it will run.
task "depends_on_two" "docker.io/library/hello-world" {
  description = "This task depends on the second task, but will run after its finished regardless of the result."
  depends_on = {
    "depends_on_one" : "any",
  }
}
