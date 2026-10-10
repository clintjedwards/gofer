# Writing Extensions

Just like tasks, extensions are simply containers, which makes them easy to test and move around. An extension is a
small HTTP service that Gofer starts, sends pipeline subscriptions to, and that calls Gofer's API when something
should happen (usually starting a run).

The easiest way to write one is with one of the SDKs. The Go SDK is `github.com/clintjedwards/gofer/sdk/go/extensions`. It handles the
HTTP service, authentication with Gofer, and printing your extension's [manifest](./manifest.md). The
[interval extension](https://github.com/clintjedwards/gofer/tree/main/containers/extensions/interval) is heavily
commented and makes a good starting point.

## The shape of an extension

You give the SDK two things:

1. **Your extension's documentation**, as plain data: the settings it takes (`ConfigParams`), the parameters pipelines
   pass when they subscribe (`PipelineSubscriptionParams`), and a description.
2. **A function that builds your extension.** It reads its settings, restores its subscriptions, and returns a value
   that implements `extsdk.ExtensionServiceInterface` (`Health`, `Debug`, `Subscribe`, `Unsubscribe`, `Shutdown`,
   and `ExternalEvent`).

```go
var documentation = extsdk.Documentation{
	Body: "Runs pipelines when the example API says so.",
	ConfigParams: []extsdk.Parameter{
		{
			Key:           "api_token",
			Documentation: "The token used to call the example API.",
			Required:      true,
			Secret:        true,
		},
		{
			Key:           "poll_interval",
			Documentation: "How often to check the example API.",
			Default:       "1m",
		},
	},
	PipelineSubscriptionParams: []extsdk.Parameter{
		{
			Key:           "topic",
			Documentation: "Which topic to watch.",
			Required:      true,
		},
	},
}

func newExtension() *extension {
	token := extsdk.GetConfigFromEnv("api_token")
	// ...set up the extension, restore subscriptions with sdk.ListExtensionSubscriptions, and so on.
	return &extension{token: token}
}

func main() {
	extsdk.Run(documentation, func() extsdk.ExtensionServiceInterface { return newExtension() })
}
```

Keeping the documentation separate from the extension itself is what makes manifests work. When the binary is run as
`./extension manifest --image <image>`, `Run` prints the manifest and exits without ever calling `newExtension`, so
it doesn't need any settings, a Gofer server, or credentials.

The manifest is the only way Gofer sees your documentation; it never asks the running extension. Changing a setting
or parameter means regenerating and republishing the manifest along with the new image.

## Settings

- Mark any setting that holds a credential with `Secret: true`. Gofer will only accept a global secret reference for
  it, which keeps the real value out of Gofer's config and database.
- Give optional settings a `Default` when there's a sensible one. Gofer fills it in when the operator doesn't set the
  key, and shows it in `gofer extension manifest` output.
- Read settings with `extsdk.GetConfigFromEnv("key")`. Gofer passes them as `GOFER_EXTENSION_CONFIG_<KEY>`
  environment variables.
- Settings are read once when the extension starts. Changing one means a restart, which `gofer extension reload`
  handles.

## Publishing

Build and push your image, then generate a manifest that points at that exact image tag and publish it somewhere
operators can download it over https:

```bash
docker build -t ghcr.io/me/my_extension:1.2.0 .
docker push ghcr.io/me/my_extension:1.2.0
go run . manifest --image ghcr.io/me/my_extension:1.2.0 > manifest.toml
```

Keeping `manifest.toml` in your repository and regenerating it on each release works well; that's how
[Gofer's own extensions](./provided/index.html) do it (`make generate-manifests` writes them and `make check-manifests`
fails if one is out of date).

Operators then install your extension by pointing an `[[extensions.install]]` entry at the manifest. See
[Installing and configuring extensions](./index.html#installing-and-configuring-extensions).

## Compatibility

Operators upgrade extensions in place, so new versions have to keep working with the data and settings older
versions left behind. See [Upgrading Extensions](./upgrading.md#compatibility).

## Rust

The Rust SDK (`gofer_sdk::extension`) works the same way. You implement the `Extension` trait and hand `run` your
documentation and an async function that builds the extension. Running the binary as
`<binary> manifest --image <image>` prints the manifest without calling that function, and the output is identical to
what the Go SDK would print for the same documentation.

```rust
use gofer_sdk::extension::{Documentation, Extension, Parameter, get_config_from_env, run};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let documentation = Documentation {
        body: "Runs pipelines when the example API says so.".into(),
        config_params: vec![Parameter {
            key: "api_token".into(),
            required: true,
            documentation: "The token used to call the example API.".into(),
            secret: true,
            default: String::new(),
        }],
        pipeline_subscription_params: vec![],
    };

    run(documentation, || async {
        let token = get_config_from_env("api_token").ok_or("api_token is required")?;
        // ...set up the extension and restore its subscriptions.
        Ok(Box::new(MyExtension { token }) as Box<dyn Extension>)
    })
    .await
}
```

The function can be async and return an error, so it can call Gofer's API to restore subscriptions and fail cleanly
if a setting is wrong.
