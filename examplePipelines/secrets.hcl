id          = "secret_test_pipeline"
name        = "[secrets] Gofer Test Pipeline"
description = <<EOT
This pipeline shows off how secrets might be used. Gofer can read secrets from an implemented secret store and populate
the pipeline configuration with them.
EOT

task "no_dependencies" "ghcr.io/clintjedwards/experimental:log" {
  description = "This task has no dependencies so it will run immediately"

  // The env_variable mentioned here is special, for this example we're pretneding its a secret we don't want exposed.
  // As such we use the special secret syntax to convey to Gofer that the secret must be retrieved from the secret store
  // beforehand.
  env_vars = {
    "SOME_VARIABLE" : "something here"
    "LOGS_HEADER" : "{{ logs_header }}"
  }
}
