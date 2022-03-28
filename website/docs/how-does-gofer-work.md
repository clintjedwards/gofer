---
sidebar_position: 3
---

# How does Gofer work?

Gofer works in a very simple client-server model. You deploy Gofer as a single binary to your favorite VPS and you can configure it to connect to all the tooling you currently use to run containers.

Gofer acts as a scheduling middle man between a user's intent to run a container at the behest of an event and your already established container orchestration system.

## Workflow

Interaction with Gofer is mostly done through its [command line interface](cli/gofer) which is included in the same binary as the master service.

### General Workflow

1. Gofer is connected to a container orchestrator of some sort. This can be just your local docker instance or something like K8s or Nomad.
2. It launches it's configured triggers (triggers are just docker containers) and these triggers wait for events to happen.
3. Users create pipelines(by configuration file) that define exactly in which order and what containers they would like to run.
4. These pipelines don't have to, but usually involve triggers so that pipelines can run automatically.
5. Either by trigger or manual intervention a pipeline run will start and schedule the containers defined in the configuration file.
6. Gofer will collect the logs, exit code, and other essentials from each container run and provide them back to the user along with summaries of how that particular run performed.

## Concepts

### DAG(Directed Acyclic Graph) Support

Gofer provides the ability to run your containers in reference to your other containers.

With DAG support you can run containers:

- In parallel.
- After other containers.
- When particular containers fail.
- When particular containers succeed.

### Triggers

Triggers are the way users can automate their pipelines by waiting on bespoke events (like the passage of time).

Gofer supports any trigger you can imagine by making triggers pluggable and portable! Triggers are nothing more than docker containers themselves that talk to the main process when its time for a pipeline to be triggered.

Gofer out of the box provides some default triggers like [cron](triggers/cron/overview), [interval](triggers/interval/overview), and [github](triggers/github/overview). But even more powerful than that, it accepts any type of trigger you can think up and code using the included [SDK](https://pkg.go.dev/github.com/clintjedwards/gofer/sdk).

Triggers are brought up alongside Gofer as docker containers that it launches and manages.

#### Implementation

1. When Gofer launches the first thing it does is create the trigger containers the same way it schedules any other container.
2. The trigger containers are all small GRPC services that are implemented using a specific interface provided by the [SDK](https://pkg.go.dev/github.com/clintjedwards/gofer/sdk).
3. Gofer passes the trigger a secret value that only it knows so that the trigger doesn't respond to any requests that might come from other sources.
4. After the trigger is initialized Gofer will subscribe any pipelines that have requested this trigger (through their pipeline configuration file) to that trigger.
5. The trigger then takes note of this subscription and waits for the relevant event to happen.
6. When the event happens it figures out which pipeline should be alerted and sends an event to the main Gofer process.
7. The main gofer process then starts a pipeline run on behalf of the trigger.

### Notifiers

Notifiers work in a similar way to triggers. They're just docker containers that run alongside your pipeline. Notifiers are first registered
with Gofer and then once added to a pipeline will wait until the pipeline has finished all task-runs before running themselves.

### Pluggable Everything

Gofer plugs into all your favorite backends your team is already using. This means that you never have to maintain things outside of your wheelhouse.

Whether you want to schedule your containers on [K8s](https://kubernetes.io/) or [AWS Lambda](https://aws.amazon.com/lambda/), or maybe you'd like to use an object store that you're more familiar with in [minio](https://min.io/) or [AWS S3](https://aws.amazon.com/s3/), Gofer provides either an already created plugin or an interface to write your own.

### Object Store

Gofer provides a built in object store [you can access with the Gofer CLI](cli/gofer_pipeline_store). This object store provides a caching and data transfer mechanism so you can pass values from one container to the next, but also store objects that you might need for all containers.

### Secret Store

Gofer provides a built in secret store [you can access with the Gofer CLI](cli/gofer_pipeline_secret). This secret store provides a way to pass secret values needed by your pipeline configuration into Gofer.
