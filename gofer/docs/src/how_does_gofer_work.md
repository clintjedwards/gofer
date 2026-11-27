# How does Gofer work?

Gofer works in a very simple client-server model. You deploy Gofer as a single binary to your favorite VPS, and it runs
containers on that machine's Docker.

Gofer sits between the events that should start your work, like a schedule or a push to Github, and the containers that
do it.

## Workflow

Interaction with Gofer is mostly done through its [command line interface](./cli/index.html) which is included in the
same binary as the master service.

### General Workflow

1. Gofer runs every container on the [Docker](./ref/docker.md) of the machine it's on.
2. It launches its configured extensions (extensions are just containers) and these extensions wait for events to happen
   or perform some service on behalf of your pipeline.
3. Users create pipelines (by configuration file) that define exactly in which order and what containers they would like
   to run. `gofer up` builds each task's image on the user's machine and uploads it with the config, so each version of
   a pipeline keeps the images it was registered with.
4. Pipelines can be [subscribed to extensions](./ref/pipeline_configuration/subscriptions.md) so they run automatically.
5. Either by extension or manual intervention a pipeline run will start and run the containers defined in the
   configuration file, loading each task's image into Docker from Gofer's own store when Docker doesn't have it yet.
6. Gofer will collect the logs, exit code, and other essentials from each container run and provide them back to the
   user along with summaries of how that particular run performed.

## Extension Implementation

1. When Gofer launches the first thing it does is create the extension containers the same way it starts any other
   container.
2. The extension containers are all small REST API web services that are implemented using a specific interface provided
   by the sdk.
3. Gofer passes the extension a secret value that only it knows so that the extension doesn't respond to any requests
   that might come from other sources.
4. After the extension is initialized Gofer sends it every pipeline subscription it has (created with
   [`gofer pipeline subscribe`](./ref/pipeline_configuration/subscriptions.md)).
5. The extension then takes note of this subscription and waits for the relevant event to happen.
6. When the event happens it figures out which pipeline should be alerted and sends an event to the main Gofer process.
7. The main gofer process then starts a pipeline run on behalf of the extension.
