---
sidebar_position: 1
---

# 1) Running the server locally

Gofer is deployed as a single static binary allowing you to run the full service locally so you can play with the
internals before committing resources to it. Spinning Gofer up locally is also a great way to
debug "what would happen if?" questions that might come up during the creation of pipeline config files.

## [Install Gofer](../installing-gofer)

## Install Docker

The way in which Gofer runs containers is called a [Scheduler](../../schedulers/overview). When deploying Gofer at scale we can deploy it with a more serious container scheduler ([Nomad](https://www.nomadproject.io/), [Kubernetes](https://kubernetes.io/)) but for now we're just going to use the default local docker scheduler included. This simply uses your local instance of [docker](../../schedulers/docker/overview) to run containers.

But before we use your local docker instance we have to have one in the first place. If you don't have docker installed, the installation is quick. Rather than covering the specifics here you can instead find a guide on how to install docker for your operating system [on its documentation site.](https://docs.docker.com/get-docker/)

## Start the server

By default the Gofer binary is able to run the server in development mode. Simply start the service by:

```shell
gofer service start
```

:::tip

The Gofer CLI has many useful commands, try running `gofer -h` to see a full listing.

:::
