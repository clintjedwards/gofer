# Running the Server Locally

Gofer is deployed as a single static binary allowing you to run the full service locally so you can play with the
internals before committing resources to it. Spinning Gofer up locally is also a great way to
debug "what would happen if?" questions that might come up during the creation of pipeline config files.

## [Install Gofer](./installing_gofer.md)

## Install Docker

Gofer uses your local instance of [docker](../ref/docker.md) to run containers.

But before we use your local docker service... you have to have one in the first place. If you don't have docker
installed, the installation is quick. Rather than covering the specifics here you can instead find a guide on how to
install docker for your operating system [on its documentation site.](https://docs.docker.com/get-docker/)

Gofer talks to Docker as the user you run it as, so make sure that user can run containers. `docker ps` working
without `sudo` is a good sign; on Linux that usually means being in the `docker` group.

`gofer up` uses Docker too, on whichever machine you run it from, to build the images of tasks that copy files in or
run commands. Here that's the same machine.

## Start the server

By default the Gofer binary is able to run the server in development mode. Simply start the service by:

```bash
gofer service start
```

The server runs in the foreground and prints its logs, so leave it running and use a second terminal for the rest of
this guide.

To check that it's up, open [http://localhost:8080](http://localhost:8080) in your browser to see Gofer's web UI. The
CLI is already set up to talk to a local server, so `gofer pipeline list` should work too (it'll be empty for now).

### What development mode means

With no configuration, Gofer starts with settings meant for trying it out on your own machine:

- **Authentication is off.** Anyone who can reach port 8080 can do anything, so don't run it like this on a shared
  machine or network.
- **Everything is stored in `/tmp`.** Your pipelines, runs, secrets, and objects are lost when your machine restarts.
- **Secrets use a placeholder encryption key** that's the same for every development install.

That's all fine for following this guide. When you're ready to run Gofer for real, the
[server configuration](../ref/server_configuration/index.html) page covers turning these off.

<div class="box note">
    <div class="text"><strong>Note:</strong> The Gofer CLI has many useful commands, try running `gofer -h` to see a full listing.</div>
</div>

<style>
.box {
    padding: 10px 15px;
    margin: 10px 0;
    align-items: center;
}

.note {
    border-left: 5px solid #0074d9;
}
</style>
