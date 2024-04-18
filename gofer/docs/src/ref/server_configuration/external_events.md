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

## It works like this:

1. When the Gofer service is started it starts the external events service on a separate port per the
   service configuration settings. It is also possible to just turn off this feature via the same configuration file.
2. External services can send Gofer http requests with payloads and headers specific to the extension they're
   trying to communicate with. It's possible to target specific extensions by using the `/events` endpoint.

   `ex: https://mygofer.mydomain.com/events/github <- #extension label`

3. Gofer serializes and forwards the request to the relevant extension where it is validated for authenticity of
   sender and then processed.
4. A extension may then handle this external event in any way it pleases. For example, the Github extension takes in
   external events which are expected to be Github webhooks and starts a pipeline if the event type matches one the user wanted.

[^1]:
    The reason for the alternate endpoint is due to the security concerns with sharing the same endpoint as the
    main API service of the Gofer API. Since this endpoint is different you can now specifically set up security
    groups such that it is only exposed to IP addresses that you trust without exposing those same address to
    Gofer as a whole.
