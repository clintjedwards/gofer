id          = "notifier_test_pipeline"
name        = "[notifier] Gofer Test Pipeline"
description = <<EOT
This pipeline shows off the notify function of Gofer. You can have Gofer automatically report the outcome of your
pipeline and do various things with that information.
EOT

task "simple_task" "ubuntu:latest" {
  description = "This task simply prints our hello-world message and exits!"
  exec "/bin/bash" {
    script = <<EOT
    echo Hello from Gofer!
    EOT
  }
}

// We invoke the "log" notifier. Which simply runs as a container after all our regular tasks have been run.
notify "log" "logger" {
    include_timestamp = "true"
}
