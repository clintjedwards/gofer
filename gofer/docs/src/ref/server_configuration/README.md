# Server Configuration

Gofer runs as a single static binary that you deploy onto your favorite VPS.

While Gofer will happily run in development mode without any additional configuration, this mode is **NOT**
recommended for production workloads and **not intended to be secure.**

Instead Gofer allows you to edit it's startup configuration allowing you to configure it to run on your favorite
container orchestrator, object store, and/or secret backend.

## Setup

There are a few steps to setting up the Gofer service for production:

### 1) Configuration

First you will need to properly configure the Gofer service.

Gofer accepts configuration through environment variables or a configuration file. If a configuration key
is set both in an environment variable and in a configuration file, the value of the environment variable's
value will be the final value.

You can view a list of environment variables Gofer takes by using the `gofer service start -h` command. It's
important to note that each environment variable starts with a prefix of `GOFER_WEB_`. So setting the `api.log_level`
configuration can be set as:

```bash
export GOFER_WEB_API__LOG_LEVEL=debug
```

#### Configuration file

The Gofer service configuration file is written in [TOML](https://toml.io/en/).

##### Load order

The Gofer service looks for its configuration in one of several places (ordered by first searched):

1. /etc/gofer/gofer_web.toml

#### Bare minimum production file

These are the bare minimum values you should populate for a production ready Gofer configuration.

The values below should be changed depending on your environment; leaving them as they currently are will lead to loss
of data on server restarts.

### 2) Running the binary

You can find the most recent releases of Gofer on the [github releases page.](https://github.com/clintjedwards/gofer/releases).

Simply use whatever configuration management system you're most familiar with to place the binary on your chosen
VPS and manage it. You can find a quick and dirty `wget` command to pull the
latest version in the [getting started documentation.](../../guide/index.html)

### 3) First steps

You will notice upon service start that the Gofer CLI is unable to make any requests due to permissions.

You will first need to handle the problem of auth. Every request to Gofer must use an API key so Gofer can
appropriately direct requests.

More information about auth in general terms [can be found here.](./authentication.md)

To create your root management token use the command: `gofer token bootstrap`

<div class="box note">
  <div class="text">
  <strong>Note:</strong>

  <p>The token returned is a management token and as such has access to all routes within Gofer. It is advised that:</p>
    <ol>
      <li> You use this token only in admin situations and to generate other lesser permissioned tokens.</li>
      <li> Store this token somewhere safe. </li>
    </ol>
    </div>
</div>

From here you can use your root token to provision extra, lower permissioned tokens for everyday use.

When communicating with Gofer through the CLI you can set the token to be automatically passed per request in
[one of many ways.](../../cli/configuration.md)

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
