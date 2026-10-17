# Secrets and Access

## Prefer pipeline secrets over global ones.

[Pipeline secrets](../../ref/secret_store/index.html) can only be used by the pipeline they belong to, so a mistake
stays contained. Reach for a global secret only when several pipelines really need the same value, and limit it to the
namespaces that need it:

```bash
gofer secret global put team_a_deploy_key -n "team-a-.*" < ./deploy_key
```

Either way, pass secrets in with `sdk.PipelineSecret` or `sdk.GlobalSecret` instead of writing them into the pipeline
config. Configs are stored and shown as written, so anything typed into one directly is visible to anyone who can
read the pipeline.

## Only give API tokens to tasks that need them.

`InjectAPIToken(true)` gives a task a token for Gofer's API, which it needs to store objects or use the `gofer` CLI.
Turn it on only for those tasks. A task that just runs tests doesn't need to be able to talk to Gofer, and every token
you hand out is one more thing that can leak from a log or a compromised dependency.
