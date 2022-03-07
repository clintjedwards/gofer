id          = "simple_test_pipeline"
name        = "[simple] Gofer Test Pipeline"
description = <<EOT
This pipeline shows off the various features of a simple Gofer pipeline. Triggers, Tasks, and
dependency graphs are all tools that can be wielded to create as complicated pipelines as need be.
EOT

// Tasks are the building blocks of a pipeline. They represent individual containers and can be
// configured to depend on one or multiple other tasks.
task "no_dependencies" "ghcr.io/clintjedwards/experimental:wait" {
  description = "This task has no dependencies so it will run immediately"

  // Environment variables are the way in which your container is configured.
  // These are passed into your container at runtime.
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
    "no_dependencies" : "successful",
  }
  env_vars = {
    "LOGS_HEADER" : "This string can be anything you want it to be",
  }
}

// Task two is the last in line to be run since it's dependency tree looks like:
// "no_dependencies" -> "depends_on_one" -> "depends_on_task_two".
// It's only difference is that regardless of the state that "depends_on_one" ends with it will run.
// This task also shows the exec command which you can use to run shell commands on any container.
task "depends_on_task_two" "docker.io/library/hello-world" {
  description = "This task depends on the second task, but will run after its finished regardless of the result."
  depends_on = {
    "depends_on_one" : "any",
  }
  exec "/bin/bash" {
    script = <<EOT
echo Hello from Gofer!
EOT
  }
}
