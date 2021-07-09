id          = "secret_test_pipeline"
name        = "[secrets] Gofer Test Pipeline"
description = <<EOT
This pipeline shows off the various features of a simple Gofer pipeline. Triggers, Tasks, and
dependency graphs are all tools that can be wielded to create as complicated pipelines as need be.
EOT

task "no_dependencies" {
  image_name  = "ghcr.io/clintjedwards/experimental:log"
  description = "This task has no dependencies so it will run immediately"

  // Secrets are specified here to be pulled in from the scheduler
  // scheduler configuration determines how the scheduler actually retrieves these secrets
  // the key is what env var the secret should be injected as, the value is any extra configuration
  // the scheduler might need to retrieve the secert(for vault it might be the path to the secret)
  secrets = {
    "LOGS_HEADER" : "example/config/for/secrets"
  }
}
