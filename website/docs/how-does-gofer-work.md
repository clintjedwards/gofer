---
sidebar_position: 3
---

# How does Gofer work?

Gofer works in a very simple client-server model. You deploy Gofer as a single binary to your favorite VPS and you can configure it to connect to all the tooling you currently use to run containers.

That is all Gofer does by the way, run containers. It acts as a scheduling middle man between a user's intent to run a container at the behest of another event and your already established container orchestration system.

## Workflow

Interaction with Gofer is mostly done through its [command line interface](cli/gofer) which is included in the same binary as the master service.

1. Gofer is connected to a container orchestrator of some sort. It launches it's configured triggers(triggers are just docker containers) and waits for events to happen.
2. Users create pipelines that define exactly in which order and what containers they would like to run.
3. These pipelines don't have to, but usually involve triggers so that users can run their pipeline when something else happens.
4. When a pipeline is first created it "subscribes" to the triggers mentioned in its configuration.
5. When that trigger deems that the user's event has happened it sends and event to the main Gofer process that tells Gofer to trigger a new pipeline run.
6. Gofer contacts the configured scheduler to run the containers with the settings and order that the user requested in the pipeline config file.
7. Containers are run!
8. The logs, exit code, and other essentials are collected from each container run and provided to the user, along with a summary of how that particular run performed.
9. Finished!

## Concepts

### DAG Support

Gofer provides the ability to run your containers in reference which is extremely powerful:

With DAG support you can run containers:

- In parallel.
- After other containers.
- When particular containers fail.
- When particular containers succeed.

### Pluggable Triggers

Triggers are the way users can automate their pipelines by waiting on bespoke events (like the passage of time). Gofer supports any trigger you can imagine by making triggers pluggable and portable! Triggers are nothing more than docker containers themselves that talk to the main process when its time for a Pipeline to be triggered.

Gofer provides some default triggers like [cron](triggers/cron/overview) and [interval](triggers/interval/overview) but, even more powerful than that, it accepts any type of trigger you can think up and code using the included [SDK](https://pkg.go.dev/github.com/clintjedwards/gofer/sdk).

#### Implementation

1. When Gofer launches the first thing it does is create the trigger containers the same way it schedules any other container.
2. The trigger containers are all small GRPC services that are implemented using a specific interface provided by the [SDK](https://pkg.go.dev/github.com/clintjedwards/gofer/sdk).
3. Gofer passes the trigger a specific value that only it knows so that the trigger doesn't response to any requests that might come from other sources.
4. After the trigger is initialized Gofer will subscribe any pipelines that have requested this trigger (through their pipeline configuration file) to that trigger.
5. The trigger then takes note of this subscription and waits for the relevant event to happen.
6. When the event happens it figures out which pipeline should be alerted and sends an event to the main Gofer process.
7. The main gofer process then starts a pipeline run on behalf of the trigger.

### Pluggable Everything

Gofer plugs into all your favorite backends your team is already using. This means that you never have to maintain things outside of your wheelhouse.

Whether you want to schedule your containers on [K8s](https://kubernetes.io/) or [AWS Lambda](https://aws.amazon.com/lambda/), or maybe you'd like to use an object store that you're more familiar with in [minio](https://min.io/) or [AWS S3](https://aws.amazon.com/s3/), Gofer provides either an already created plugin or an interface to write your own.

### Object Store

Gofer provides a built in object store [you can access with the Gofer CLI](cli/gofer_pipeline_store). This object store provides a caching and data transfer mechanism so you can pass values from one container to the next, but also store objects that you might need for all containers.
