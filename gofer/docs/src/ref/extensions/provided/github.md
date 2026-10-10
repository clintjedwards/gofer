# Github <small>_Extension_</small>

The Github extension allows Gofer pipelines to be run on [Github webhook events.](https://docs.github.com/en/developers/webhooks-and-events/webhooks/webhook-events-and-payloads) This makes it possible to write event driven
workloads that depend on an action happening on Github.

See the [events section below](#events) for all supported events and the environment variables they pass to each
pipeline.

<div class="box note">
  <div class="text">
  <strong>Note:</strong>

  <p>Due to the nature of Github's API and webhooks, you'll need to first set up a new Github app to use with Gofer's Github extension.</p>
  <i>

  Steps to accomplish this can be found in the [additional steps section.](#additional-setup)

  </i>
  </div>
</div>

<div class="box danger">
  <div class="text">
  <strong>Danger:</strong>

  <p>

  The Github extension requires the [external events feature](../../server_configuration/external_events.md) of Gofer in order to accept webhooks from Github's servers. This requires your application to take traffic from external, potentially unknown sources.

  Visit the [external events page](../../server_configuration/external_events.md) for more information on how to configure Gofer's external
  events endpoint.

  If Github is your only external extension, to increase security consider [limiting the IP addresses](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/about-githubs-ip-addresses) that can access Gofer's external events endpoint.
  </p>
  </div>
</div>

## Pipeline Configuration

| Key                | Default  | Description                                                                                                                                                             |
| -------------------| -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| repository         | Required | The Github repository you would like to listen for events from. The format is in the form.                                                                              |
| event_filter       | Required | The event/action combination the pipeline will be triggered upon. It is presented in the form: `<event>/<action1>,<action2>...`. For events that do not have actions or if you simply want to trigger on any action, just putting the \<event\> will suffice. |

<div class="box note">
  <div class="text">
  <strong>Note:</strong>

  <p>If you don't include actions on an event that has multiple, Gofer will be triggered on any action. You can
  find a list of events and their actions here(Actions listed as 'activity type' in Github nomenclature.):
  https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows</p>

  </div>
</div>

### Example

```bash
gofer pipeline subscribe simple github run_tests \
    -s "repository=clintjedwards/experimental" \
    -s "event_filter=pull_request_with_check/opened,synchronize,reopened"
```
## Extension Configuration

The Github extension isn't installed by default. It requires a [Github app](https://docs.github.com/en/developers/apps/getting-started-with-apps/about-apps)
of your own; the [setup walkthrough below](#setting-it-up) covers creating one and installing the extension.

| Key                | Default  | Secret | Description                                                                                                                                         |
| ------------------ | -------- | ------ | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| app_id             | Required | no     | The Github app's ID, shown on the app's settings page.                                                                                              |
| app_installation   | Required | no     | The ID of the app's installation on your account or organization. See [finding the installation ID](#2-find-the-installation-id) below.           |
| app_key            | Required | yes    | The Github app's private key in PEM format, exactly as Github gives it to you.                                                                      |
| app_webhook_secret | Required | yes    | The webhook secret set on the Github app. Gofer uses it to verify that webhooks really came from Github. It should be a long, random string.        |

Secret settings must be [global secret references](../index.html#secrets) like `global_secret{{github-app-key}}`.

### Example

```toml
[[extensions.install]]
id = "github"
manifest = "https://raw.githubusercontent.com/clintjedwards/gofer/v<gofer version>/containers/extensions/github/manifest.toml"
[extensions.install.settings]
app_id = "112348"
app_installation = "99560091"
app_key = "global_secret{{github-app-key}}"
app_webhook_secret = "global_secret{{github-app-webhook-secret}}"
```

<div class="box note">
  <div class="text">
  <strong>Note:</strong>

  <p>Before Gofer 0.12 the private key had to be base64 encoded. It's now passed exactly as Github gives it to you;
  store the original <code>.pem</code> file, not the base64 version.</p>
  </div>
</div>

### Setting it up

Due to the nature of Github's API and webhooks, you'll need to first set up a new Github app to use with Gofer's Github extension.
Once this app has been set up, you'll have everything you need to configure the extension.

Here is a quick and dirty walkthrough on the important parts of setting up the Github application.

#### 1. Create a new Github application:

[Github's documentation](https://docs.github.com/en/developers/apps/building-github-apps/creating-a-github-app) will be the most up to date and relevant so please see their walkthrough.

On the configuration page for the new Github application the following should be noted:

- **APP ID**: Take note of the id; it will be used later for extension configuration.
- **Webhook URL**: Should be the address of your Gofer's external extension instance and pointing to the events/github endpoint:

  `ex: https://mygoferinstance.yourdomain.com/external/github`

- **Webhook Secret**: Make this a secure, long, random string of characters and note it for future extension configuration.
- **Private Keys**: Generate a private key and keep the `.pem` file Github gives you; you'll store it in Gofer's
  secret store in step 3.

<div class="box note">
  <div class="text">
  <strong>Note:</strong>

  <p>

  If you need a logo for your new Github application you're welcome to use [logo-small](../../../assets/logo-small.png)
  </p>
  </div>
</div>

#### 2. Find the installation ID

Once the Github application has been created, [install it.](https://docs.github.com/en/developers/apps/managing-github-apps/installing-github-apps)
This will give you an opportunity to configure the permissions and scope of the Github application.
It is recommended that you give read-only permissions to any permissions that might include webhooks and read-write for `code-suite` and `code-runs`.

You might also utilize this Github App to perform other actions within Github with either other extensions or your own pipeline jobs. Remember to allow the correct permissions for all use cases.

The installation ID is unfortunately hidden in an event that gets sent once the Github app has been created and installed. You can find it by navigating to the settings page for the Github application and
then viewing it in the "Recent Deliveries" page.

> 🪧 These recent deliveries only last a short amount of time, so if you take a while to check on them, they might not exist anymore. If that has happened you should be able to create another event and that will create another recent delivery.

![Recent Deliveries](../../../assets/github-apps-recent-deliveries.png)
![Installation webhook event](../../../assets/github-apps-installation-id.png)

#### 3. Store the secrets

Put the private key and webhook secret in Gofer's global secret store. Leaving out the value prompts for it, which
keeps it out of your shell history.

```bash
gofer secret global put github-app-key < ~/Downloads/myorg-gofer.2022-01-24.private-key.pem
gofer secret global put github-app-webhook-secret
```

#### 4. Add the extension to Gofer's config

Add an `[[extensions.install]]` entry like the [example above](#example), with your app ID and installation ID and
references to the two secrets. Point `manifest` at the github manifest for your version of Gofer.

`gofer extension manifest <manifest url> --id github` prints this block for you, with every setting documented.

#### 5. Reload

```bash
gofer extension reload
```

Gofer shows the extension it's about to install and its settings; confirm, and it starts. If it doesn't,
`gofer extension list` shows why and `gofer extension logs github` shows what the extension itself reported.

## Events

Gofer's extensions have the ability to pass along event specific information in the form of environment variables that
get injected into each container's run. Most of these variables are pulled from the webhook request that comes in.

Below is a breakdown of the environment variables that are passed to a run based on the event that was generated.
You can find more information about the format the variables will be in by [referencing the payloads for the event](https://docs.github.com/en/developers/webhooks-and-events/webhooks/webhook-events-and-payloads).

Events below are the only events that are supported.

| Event                    | Metadata                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| pull_request             | "GOFER_EXTENSION_GITHUB_EVENT" <br/>"GOFER_EXTENSION_GITHUB_ACTION"<br/>"GOFER_EXTENSION_GITHUB_PULLREQUEST_HEAD_REF"<br/>"GOFER_EXTENSION_GITHUB_REPOSITORY"<br/>"GOFER_EXTENSION_GITHUB_PULLREQUEST_HEAD_SHA"<br/>"GOFER_EXTENSION_GITHUB_PULLREQUEST_AUTHOR_USERNAME"<br/>"GOFER_EXTENSION_GITHUB_PULLREQUEST_AUTHOR_EMAIL"<br/>"GOFER_EXTENSION_GITHUB_PULLREQUEST_AUTHOR_NAME"<br/>                                                                       |
| pull_request_with_check  | "GOFER_EXTENSION_GITHUB_EVENT" <br/>"GOFER_EXTENSION_GITHUB_ACTION"<br/>"GOFER_EXTENSION_GITHUB_PULLREQUEST_HEAD_REF"<br/>"GOFER_EXTENSION_GITHUB_REPOSITORY"<br/>"GOFER_EXTENSION_GITHUB_PULLREQUEST_HEAD_SHA"<br/>"GOFER_EXTENSION_GITHUB_PULLREQUEST_AUTHOR_USERNAME"<br/>"GOFER_EXTENSION_GITHUB_PULLREQUEST_AUTHOR_EMAIL"<br/>"GOFER_EXTENSION_GITHUB_PULLREQUEST_AUTHOR_NAME"<br/>                                                                       |                                                                                                                                                                                                                                                                        |
| push                     | "GOFER_EXTENSION_GITHUB_EVENT":<br/>"GOFER_EXTENSION_GITHUB_ACTION"<br/>"GOFER_EXTENSION_GITHUB_REF"<br/>"GOFER_EXTENSION_GITHUB_REPOSITORY"<br/>"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_ID"<br/>"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_AUTHOR_NAME"<br/>"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_AUTHOR_EMAIL"<br/>"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_AUTHOR_USERNAME"<br />"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_COMMITTER_NAME"<br />"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_COMMITTER_EMAIL"<br />"GOFER_EXTENSION_GITHUB_HEAD_COMMIT_COMMITTER_USERNAME"<br />|
| release                  | "GOFER_EXTENSION_GITHUB_ACTION"<br/>"GOFER_EXTENSION_GITHUB_REPOSITORY"<br/>"GOFER_EXTENSION_GITHUB_RELEASE_TAG_NAME"<br/>"GOFER_EXTENSION_GITHUB_RELEASE_TARGET_COMMITISH"<br/>"GOFER_EXTENSION_GITHUB_RELEASE_AUTHOR_LOGIN"<br/>"GOFER_EXTENSION_GITHUB_RELEASE_CREATED_AT"<br/>"GOFER_EXTENSION_GITHUB_RELEASE_PUBLISHED_AT"                                                                                                                                |

<div class="box note">
  <div class="text">
  <strong>Note:</strong>

  <p>The event <code>pull_request_with_check</code> is a special event not found within the Github API. It's primarily
  to be used when the subscriber wants to report the job status back to the pull request based on the result of the job started</p>
  </div>
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

.danger {
    border-left: 5px solid #FF6961;
}
</style>
