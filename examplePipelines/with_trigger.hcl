id          = "trigger_test_pipeline"
name        = "[with_trigger] Gofer Test Pipeline"
description = <<EOT
This pipeline shows off the various features of a simple Gofer pipeline. Triggers, Tasks, and
dependency graphs are all tools that can be wielded to create as complicated pipelines as need be.
EOT

// Triggers are plugins that control the automatic execution of pipeline.
// They typically take some kind of configuration which controls the behavior of the trigger.
// The name here "interval" denotes the "kind" of trigger. Gofer supports multiple trigger kinds.
// A list of trigger kinds can be found in the documentation or via the command line:
// `gofer trigger list`
//
// You can tie triggers to a label; useful for remembering why they were created and
// referencing multiple instances of a single trigger kind quickly. The label "every_one_minute"
// here is a short and quick summary of our interval trigger's purpose.
trigger "interval" "every_one_minute" {
  every = "1m"
}

// Tasks are the building blocks of a pipeline. They represent individual containers and can be
// configured to depend on one or multiple other tasks.
task "simple_task" "ubuntu:latest" {
  description = "This task simply prints our hello-world message and exits!"
  exec "/bin/bash" {
    script = <<EOT
    echo Hello from Gofer!
    EOT
  }
}
