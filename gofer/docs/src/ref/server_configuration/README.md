# Server Configuration

Gofer runs as a single static binary that you deploy onto your favorite VPS.

While Gofer will happily run in development mode without any additional configuration, this mode is **NOT**
recommended for production workloads and **not intended to be secure.**

Instead Gofer allows you to edit its startup configuration allowing you to configure it to run on your favorite
container orchestrator, object store, and/or secret backend.

## Setup

There are a few steps to setting up the Gofer service for production:

### 1) Configuration

First you will need to properly configure the Gofer service.

Gofer accepts configuration through environment variables or a configuration file. If a configuration key
is set both in an environment variable and in a configuration file, the value of the environment variable's
value will be the final value.

Gofer reads its configuration file from `/etc/gofer/gofer_web.toml`, or from the path passed to
`gofer service start --config <path>`. Extensions are configured in this file too, and `gofer extension reload`
rereads it to pick up extension changes without a restart.

Every key in the [configuration reference](./configuration_reference.md) can also be set as an environment variable.
Each one starts with a prefix of `GOFER_WEB_` and uses a double underscore between the block and the key. So the
`api.log_level` configuration can be set as:

```bash
export GOFER_WEB_API__LOG_LEVEL=debug
```

#### Configuration file

The Gofer service configuration file is written in [TOML](https://toml.io/en/).

##### Load order

Gofer starts from its built in defaults, then layers on top of them, in order:

1. The file passed to `gofer service start --config <path>`, or `/etc/gofer/gofer_web.toml` if no path is given.
2. `GOFER_WEB_` environment variables.

You only need to set the keys you want to change.

#### Bare minimum production file

These are the bare minimum values you should populate for a production ready Gofer configuration.

The values below should be changed depending on your environment. The defaults keep everything in `/tmp`, which will
lead to loss of data on server restarts, and they turn off authentication.

```toml
[server]
bind_address = "0.0.0.0:8080"
# The address extension containers use to reach Gofer. With the docker scheduler this is usually the docker
# bridge's address on the host.
extension_address = "172.17.0.1:8080"
storage_path = "/var/lib/gofer/gofer.db"
use_tls = true
tls_cert_path = "/etc/gofer/tls/cert.pem"
tls_key_path = "/etc/gofer/tls/key.pem"

# The defaults are on to make local development easy. All of them are unsafe in production.
[development]
pretty_logging = false
bypass_auth = false
use_included_certs = false

[api]
task_execution_logs_dir = "/var/lib/gofer/logs"

[external_events]
use_tls = true
tls_cert_path = "/etc/gofer/tls/cert.pem"
tls_key_path = "/etc/gofer/tls/key.pem"

[object_store.filesystem]
path = "/var/lib/gofer/objects"

[secret_store.sqlite]
path = "/var/lib/gofer/secrets.db"
# Exactly 32 characters; `openssl rand -hex 16` makes one. Never change it after secrets are stored.
encryption_key = "<your key here>"
```

If you don't use external events (for example the Github extension), set `external_events.enable = false` instead
of configuring TLS for it.

Gofer creates the object store directory itself, but not the others, so create `/var/lib/gofer` and
`/var/lib/gofer/logs` and make sure the user Gofer runs as can write to them.

### 2) Running the binary

You can find the most recent releases of Gofer on the [github releases page.](https://github.com/clintjedwards/gofer/releases).

Simply use whatever configuration management system you're most familiar with to place the binary on your chosen
VPS and manage it. You can find a quick and dirty `wget` command to pull the
latest version in the [getting started documentation.](../../guide/index.html)

### 3) First steps

You will notice upon service start that the Gofer CLI is unable to make any requests due to permissions.

You will first need to handle the problem of auth. Every request to Gofer must use an API key so Gofer can
appropriately direct requests.

More information about auth in general terms [can be found here.](./authz_n.md)

To create your root/bootstrap token use the command: `gofer token bootstrap`

<div class="box note">
  <div class="text">
  <strong>Note:</strong>

  <p>The token returned is a bootstrap token and as such has access to all routes within Gofer. It is advised that:</p>
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
