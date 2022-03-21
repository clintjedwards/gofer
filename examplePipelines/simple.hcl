id          = "simple_test_pipeline"
name        = "[simple] Gofer Test Pipeline"
description = <<EOT
This pipeline shows off a very simple pipeline that simply pulls in a container and runs a command.
Veterans of CI/CD tooling should be familar with this pattern.

Tasks are the building blocks of a pipeline. They represent individual containers and can be configured to depend on
one or multiple other tasks.

In the task here, we simply call the very familar Ubuntu container and run some commands of our own.

While this is the simplest example of Gofer, the vision is to move away from writing our logic code in long bash
scripts within these task definitions.

Ideally, these tasks are custom containers built with the purpose of being run within Gofer for a
particular workflow. Allowing you to keep the logic code closer to the actual object that uses it and keeping
the Gofer pipeline configurations from becoming a mess.
EOT

task "simple_task" "ubuntu:latest" {
  description = "This task simply prints our hello-world message and exits!"
  exec "/bin/bash" {
    script = <<EOT
    echo Hello from Gofer!
    EOT
  }
}
