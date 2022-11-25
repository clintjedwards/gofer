# How does Gofer work?

Gofer works in a very simple client-server model. You deploy Gofer as a single binary to your favorite VPS and you can configure it to connect to all the tooling you currently use to run containers.

Gofer acts as a scheduling middle man between a user's intent to run a container at the behest of an event and your already established container orchestration system.

## Workflow

Interaction with Gofer is mostly done through its [command line interface](./cli/README.md) which is included in the same binary as the master service.

### General Workflow

1. Gofer is connected to a container orchestrator of some sort. This can be just your local docker service or something like K8s or Nomad.
2. It launches it's configured extensions (extensions are just docker containers) and these extensions wait for events to happen.
3. Users create pipelines (by configuration file) that define exactly in which order and what containers they would like to run.
4. These pipelines don't have to, but usually involve extensions so that pipelines can run automatically.
5. Either by extension or manual intervention a pipeline run will start and schedule the containers defined in the configuration file.
6. Gofer will collect the logs, exit code, and other essentials from each container run and provide them back to the user along with summaries of how that particular run performed.

## Extension Implementation

1. When Gofer launches the first thing it does is create the extension containers the same way it schedules any other container.
2. The extension containers are all small GRPC services that are implemented using a specific interface provided by the [SDK](https://pkg.go.dev/github.com/clintjedwards/gofer/sdk).
3. Gofer passes the extension a secret value that only it knows so that the extension doesn't respond to any requests that might come from other sources.
4. After the extension is initialized Gofer will subscribe any pipelines that have requested this extension (through their pipeline configuration file) to that extension.
5. The extension then takes note of this subscription and waits for the relevant event to happen.
6. When the event happens it figures out which pipeline should be alerted and sends an event to the main Gofer process.
7. The main gofer process then starts a pipeline run on behalf of the extension.
