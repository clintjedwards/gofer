# External Events

Gofer has an alternate endpoint specifically for external events streams[^1]. This endpoint takes in http requests
from the outside and passes them to the relevant extension.

You can find more about external event configuration in the
[configuration-values](../server_configuration/configuration_reference.md) reference.

```toml
enable = true
bind_address = "0.0.0.0:8081"
use_tls = false
```

## Authentication

This endpoint does not use Gofer API tokens. The services that send external events, like Github webhooks, have no way
to include one. Instead, Gofer passes every request through and the extension decides whether to trust it. For
example, the Github extension checks the webhook signature Github includes in the headers.

This means anyone who can reach this port can send requests to any installed extension. Any extension that accepts
external events should verify them, and you should limit who can reach the port where possible.

## It works like this:

1. When the Gofer service is started it starts the external events service on a separate port per the
   service configuration settings. It is also possible to just turn off this feature via the same configuration file.
2. External services can send Gofer http `POST` requests with payloads and headers specific to the extension they're
   trying to communicate with. You target a specific extension by putting its id at the end of the path.

   `ex: https://mygofer.mydomain.com/api/external/github <- extension id`

3. Gofer serializes and forwards the request, including all of its headers and body, to the relevant extension where
   it is validated for authenticity of sender and then processed.
4. An extension may then handle this external event in any way it pleases. For example, the Github extension takes in
   external events which are expected to be Github webhooks and starts a pipeline if the event type matches one the user wanted.

[^1]:
    The reason for the alternate endpoint is due to the security concerns with sharing the same endpoint as the
    main API service of the Gofer API. Since this endpoint is different you can now specifically set up security
    groups such that it is only exposed to IP addresses that you trust without exposing those same address to
    Gofer as a whole.
