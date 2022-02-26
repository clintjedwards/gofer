id          = "object_test_pipeline"
name        = "[objects] Gofer Test Pipeline"
description = <<EOT
This pipeline hows how one might use Gofer's object/kv store to pass container results to other containers.
EOT


task "no_dependencies" "ghcr.io/clintjedwards/experimental:log" {
  description = "This task has no dependencies so it will run immediately"

  // The env_variable mentioned here is special, for this example we're pretending its a value we've stored in our
  // pipeline store.
  // As such we use the special secret syntax to convey to Gofer that the valuhe must be retrieved from the object store
  // beforehand.
  env_vars = {
    "SOME_VARIABLE" : "something here",
    "LOGS_HEADER" : "pipeline{{ logs_header }}",
  }
}
