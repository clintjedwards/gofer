# Schemas

Object definitions used by request and response bodies. Field types link to other schemas on this page.

## Action

A string, one of:

| Value | Description |
| ----- | ----------- |
| `read` |  |
| `write` |  |
| `delete` |  |

## ApplyExtensionRequest

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `plan_hash` | string | yes | The `plan_hash` from the plan the operator reviewed. |

## ApplyExtensionResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `can_revert` | boolean | yes | Whether the version that was running before can be brought back with revert. |
| `extension` | [Extension](#extension) | yes | The extension as it ended up. Check `state` and `state_reason` to see whether it started. |

## Build

How a task's image is made. `gofer up` builds it on the user's machine and uploads it to Gofer, so a version of a pipeline always runs exactly the image it was registered with.

One of the variants below. Variants without a payload are sent as a plain string; the rest are an object with the variant name as its only key.

| Variant | Payload | Description |
| ------- | ------- | ----------- |
| `steps` | [Steps](#steps) | Start from a base image, then copy files in and run commands, in the order they were written. |
| `dockerfile` | `context`: string<br>`path`: string | Build from a Dockerfile, for anything the steps can't do. Both paths are relative to the folder of the pipeline's config. |

## BuiltImage

The image `gofer up` built for a task.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `digest` | string | yes | The digest of the image's manifest, like `sha256:4a5c...`. |
| `source_hash` | string | yes | A hash of the files that went into the image. It's how `gofer up` knows whether they changed since. |

## Config

A representation of the user's configuration settings for a particular pipeline.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `allow_runs_from` | array of string | yes | Other pipelines whose runs may start runs of this one, as `namespace/pipeline` patterns. Each half is a regex that has to match the whole id. |
| `deprecated` | integer | yes | Time in epoch milliseconds when this pipeline config was not longer used. |
| `description` | string | yes | Description of pipeline's purpose and other details. |
| `name` | string | yes | Human readable name for pipeline. |
| `namespace_id` | string | yes | Unique identifier of the target namespace. |
| `parallelism` | integer | yes | The amount of runs allowed to happen at any given time. |
| `pipeline_id` | string | yes | Unique identifier of the target pipeline. |
| `registered` | integer | yes | Time in epoch milliseconds when this pipeline config was registered. |
| `skip_dates` | array of string | yes | Days the pipeline doesn't run, as `MM/DD` or `MM/DD-MM/DD` with an optional weekday like `:thu`. Dates repeat every year and are in UTC. |
| `state` | [ConfigState](#configstate) | yes | The deployment state of the config. This is used to determine the state of this particular config and if it is currently being used or not. |
| `tasks` | map of string to [Task](#task) | yes | Tasks associated with this pipeline. |
| `version` | integer | yes | The iteration number for this pipeline's configs. |

## ConfigState

A string, one of:

| Value | Description |
| ----- | ----------- |
| `unknown` |  |
| `unreleased` | Has never been deployed. |
| `live` | Currently deployed. |
| `deprecated` | Has previously been deployed and is now defunct. |

## CreateNamespaceRequest

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `description` | string | yes | Short description about what the namespace is used for. |
| `id` | string | yes | The unique identifier for the namespace. Only accepts alphanumeric chars with hyphens. No spaces. |
| `name` | string | yes | Humanized name for the namespace. |

## CreateNamespaceResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `namespace` | [Namespace](#namespace) | yes | Information about the namespace created. |

## CreateRoleRequest

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `description` | string | yes | Short description about what the role is used for. |
| `grants` | [Grants](#grants) | yes | What the role allows. |
| `id` | string | yes | The unique identifier for the role. Only accepts alphanumeric chars with hyphens. No spaces. |

## CreateRoleResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `role` | [Role](#role) | yes | Information about the role created. |

## CreateSubscriptionRequest

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `extension_id` | string | yes |  |
| `settings` | map of string to string | yes |  |
| `subscription_id` | string | yes |  |

## CreateSubscriptionResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `subscription` | [Subscription](#subscription) | yes | Information about the subscription created. |

## CreateTokenRequest

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `expires` | integer | yes | The amount of time the token is valid for in seconds. An expiry of 0 means that token does not expire. |
| `metadata` | map of string to string (nullable) | no | Various other bits of data you can attach to tokens. This is used by Gofer to track some details about tokens, but can also be used by users to attach bits of information that would make the token easier to programmatically manage. |
| `roles` | array of string | yes | The list of roles to apply to the token. |
| `user` | string | yes | The plaintext username of the token user. |

## CreateTokenResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `secret` | string | yes | The actual token created. API Tokens should be protected in the same fashion as passwords. |
| `token_details` | [Token](#token) | yes | Information about the token created. |

## CreateWebLoginResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `code` | string | yes | Single use code the browser trades for the token. Pass it in a URL fragment so it never reaches server logs. |
| `expires` | integer | yes | When the code stops working, in epoch milliseconds. |

## DebugResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `info` | string | yes |  |

## DeployPipelineConfigResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `deployment` | [Deployment](#deployment) | yes | Information about the pipeline created. |

## Deployment

A deployment represents a transition between two pipeline versions.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `deployment_id` | integer | yes | Unique identifier for the deployment. |
| `end_version` | integer | yes | Version of the pipeline being promoted. |
| `ended` | integer | yes | Time of deployment end in epoch milliseconds. |
| `logs` | array of [Event](#event) | yes | The event logs from the deployment. |
| `namespace_id` | string | yes | Unique identifier for the target namespace. |
| `pipeline_id` | string | yes | Unique identifier for the target pipeline. |
| `start_version` | integer | yes | Version of the pipeline is being deprecated. |
| `started` | integer | yes | Time of deployment start in epoch milliseconds. |
| `state` | [deployment_state](#deployment_state) | yes | The current state of the deployment as it exists within Gofer's operating model. |
| `status` | [deployment_status](#deployment_status) | yes | The final status of the deployment. |
| `status_reason` | [deployment_status_reason](#deployment_status_reason) (nullable) | no | Details about a deployment's specific status |

## Documentation

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `body` | string | yes | Anything the extension wants to explain to the user. This text is inserted into the documentation a user can look up about the extension. Supports AsciiDoc. |
| `config_params` | array of [Parameter](#parameter) | yes | Each extension has configuration parameters that can be passed in at extension startup. These parameters should control extension behavior for it's entire lifetime. |
| `pipeline_subscription_params` | array of [Parameter](#parameter) | yes | Each extension has pipeline subscription parameters that are passed in by a pipeline when it attempts to subscribe to an extension. This controls how the extension treats that specific pipeline subscription. |

## Error

Error information from a response.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `error_code` | string | no |  |
| `message` | string | yes |  |
| `request_id` | string | yes |  |

## Event

A single event

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `emitted` | integer | yes | Time event was performed in epoch milliseconds. |
| `id` | string | yes | Unique identifier for event. |
| `kind` | [Kind](#kind) | yes | The type of event it is. |

## ExchangeWebLoginRequest

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `code` | string | yes | The code from `create_web_login`. |

## ExchangeWebLoginResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `secret` | string | yes | The token to sign in with. Protect it like a password. |

## Extension

An Extension is the way that pipelines add extra functionality to themselves. Pipelines can "subscribe" to extensions and extensions then act on behalf of that pipeline.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `documentation` | [Documentation](#documentation) | yes | What the extension's manifest says about it. Comes from the manifest alone, so it's there whether or not the extension is running; empty only for extensions that are no longer in Gofer's config. |
| `manifest` | string | yes | Where the extension's manifest came from. Empty for extensions that are no longer in Gofer's config. |
| `registration` | [Registration](#registration) | yes | Metadata about the extension as it is registered within Gofer. |
| `started` | integer | yes | The start time of the extension in epoch milliseconds. |
| `state` | [extension_state](#extension_state) | yes | The current state of the extension as it exists within Gofer's operating model. |
| `state_reason` | string | yes | Why the extension is in its current state; set when it failed to start or was stopped. |
| `url` | string | yes | The network address used to communicate with the extension by the main process. |

## ExtensionGrant

Grants access to resources belonging to matching extensions.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `actions` | array of [Action](#action) | yes |  |
| `extension` | string | yes | Regex matched against the entire extension id. Use '.*' to match every extension. |
| `resources` | array of [ExtensionResource](#extensionresource) | yes |  |

## ExtensionResource

Things that belong to a specific extension.

A string, one of:

| Value | Description |
| ----- | ----------- |
| `objects` |  |
| `subscriptions` |  |
| `logs` |  |

## FieldChange

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `field` | string | yes |  |
| `new` | string | yes |  |
| `old` | string | yes |  |

## GetDeploymentResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `deployment` | [Deployment](#deployment) | yes | The target deployment. |

## GetEventResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `event` | [Event](#event) | yes | The target event. |

## GetExtensionResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `extension` | [Extension](#extension) | yes | The extension requested. |

## GetGlobalSecretResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `metadata` | [Secret](#secret) | yes | The target secret metadata. |
| `secret` | string (nullable) | no | The actual secret, only included if "include_secret" param is true. |

## GetNamespaceResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `namespace` | [Namespace](#namespace) | yes | The target namespace. |

## GetPipelineConfigResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `config` | [Config](#config) | yes | The target pipeline config. |

## GetPipelineResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `pipeline` | [Metadata](#metadata) | yes | The metadata for the pipeline. |

## GetPipelineSecretResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `metadata` | [Secret](#secret) | yes | The target secret metadata. |
| `secret` | string (nullable) | no | The actual secret, only included if "include_secret" param is true. |

## GetRoleResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `role` | [Role](#role) | yes | The target role. |

## GetRunResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `run` | [Run](#run) | yes | The run requested. |

## GetSubscriptionResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `subscription` | [Subscription](#subscription) | yes | The metadata for the subscription. |

## GetSystemMetadataResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `commit` | string | yes | The commit of the current build. |
| `semver` | string | yes | The semver version of the current build. |

## GetSystemPreferencesResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `bootstrap_token_created` | boolean | yes |  |
| `ignore_pipeline_run_events` | boolean | yes |  |

## GetTaskExecutionResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `task_execution` | [TaskExecution](#taskexecution) | yes | The task execution requested. |

## GetTokenByIDResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `token` | [Token](#token) | yes | The target token. |

## GlobalGrant

Grants access to resources that aren't scoped to a namespace or extension. Only 'read' on 'events', 'tokens', or 'roles' is accepted; the rest is admin only.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `actions` | array of [Action](#action) | yes |  |
| `resources` | array of [GlobalResource](#globalresource) | yes |  |

## GlobalResource

Things that aren't scoped to a namespace or extension.

A string, one of:

| Value | Description |
| ----- | ----------- |
| `events` |  |
| `tokens` |  |
| `roles` |  |
| `secrets` |  |
| `system` |  |

## Grants

Everything a role allows. Each grant stands on its own; a request is allowed only if a single grant matches its target and includes both the resource and the action.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `extensions` | array of [ExtensionGrant](#extensiongrant) | no |  |
| `global` | array of [GlobalGrant](#globalgrant) | no |  |
| `namespaces` | array of [NamespaceGrant](#namespacegrant) | no |  |

## Initiator

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `id` | string | yes | The unique identifier for the token that initiated the request. |
| `kind` | [token_kind](#token_kind) | yes | What kind of token started the run; a person, an extension, another run, or `anonymous` when no token was used (only possible with `bypass_auth` on). |
| `user` | string | yes | The plaintext username for of the token. |

## Keep

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `base` | string | yes | The image to move them onto, like `debian:13-slim`. It needs whatever the kept files need to run, like the system libraries a program uses. |
| `paths` | array of string | yes | Full paths in the image. Each lands at the same path on the new base. |

## Kind

One of the variants below. Variants without a payload are sent as a plain string; the rest are an object with the variant name as its only key.

| Variant | Payload | Description |
| ------- | ------- | ----------- |
| `any` | none | The Any kind is a special event kind that denotes the caller wants to listen for any event. It should not be used as a normal event type(for example do not publish anything with it). It is internal only and not passed back on event streaming. |
| `created_namespace` | `namespace_id`: string |  |
| `deleted_namespace` | `namespace_id`: string |  |
| `disabled_pipeline` | `namespace_id`: string<br>`pipeline_id`: string |  |
| `enabled_pipeline` | `namespace_id`: string<br>`pipeline_id`: string |  |
| `created_pipeline` | `namespace_id`: string<br>`pipeline_id`: string |  |
| `deleted_pipeline` | `namespace_id`: string<br>`pipeline_id`: string |  |
| `started_deployment` | `end_version`: integer<br>`namespace_id`: string<br>`pipeline_id`: string<br>`start_version`: integer |  |
| `completed_deployment` | `end_version`: integer<br>`namespace_id`: string<br>`pipeline_id`: string<br>`start_version`: integer |  |
| `queued_run` | `namespace_id`: string<br>`pipeline_id`: string<br>`run_id`: integer |  |
| `started_run` | `namespace_id`: string<br>`pipeline_id`: string<br>`run_id`: integer |  |
| `completed_run` | `namespace_id`: string<br>`pipeline_id`: string<br>`run_id`: integer<br>`status`: [run_status](#run_status) |  |
| `started_run_cancellation` | `namespace_id`: string<br>`pipeline_id`: string<br>`run_id`: integer |  |
| `created_task_execution` | `namespace_id`: string<br>`pipeline_id`: string<br>`run_id`: integer<br>`task_execution_id`: string |  |
| `started_task_execution` | `namespace_id`: string<br>`pipeline_id`: string<br>`run_id`: integer<br>`task_execution_id`: string |  |
| `completed_task_execution` | `namespace_id`: string<br>`pipeline_id`: string<br>`run_id`: integer<br>`status`: [task_execution_status](#task_execution_status)<br>`task_execution_id`: string |  |
| `started_task_execution_cancellation` | `namespace_id`: string<br>`pipeline_id`: string<br>`run_id`: integer<br>`task_execution_id`: string<br>`timeout`: integer |  |
| `extension_state_changed` | `extension_id`: string<br>`reason`: string<br>`state`: [extension_state](#extension_state) | An extension started, stopped, failed, or stopped answering health checks. Reason says why, when there is one. |
| `pipeline_extension_subscription_registered` | `extension_id`: string<br>`namespace_id`: string<br>`pipeline_id`: string<br>`subscription_id`: string |  |
| `pipeline_extension_subscription_unregistered` | `extension_id`: string<br>`namespace_id`: string<br>`pipeline_id`: string<br>`subscription_id`: string |  |
| `created_role` | `role_id`: string |  |
| `deleted_role` | `role_id`: string |  |

## ListDeploymentsResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `deployments` | array of [Deployment](#deployment) | yes | A list of all deployments. |

## ListExtensionObjectsResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `objects` | array of [Object](#object) | yes | A list of all extension objects. |

## ListExtensionSubscriptionsResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `subscriptions` | array of [Subscription](#subscription) | yes | A list of all pipeline subscriptions for the given extension. |

## ListExtensionsResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `extensions` | array of [Extension](#extension) | yes | A list of all extensions. |

## ListGlobalSecretsResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `secrets` | array of [Secret](#secret) | yes | A list of all global secrets. |

## ListMissingImageBlobsRequest

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `digests` | array of string | yes | The digests of every blob the images are made of. |

## ListMissingImageBlobsResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `digests` | array of string | yes | The digests of the blobs that still need to be uploaded. |

## ListNamespacesResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `namespaces` | array of [Namespace](#namespace) | yes | A list of all namespaces. |

## ListPipelineConfigsResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `configs` | array of [Config](#config) | yes | A list of all pipelines configs. |

## ListPipelineObjectsResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `objects` | array of [Object](#object) | yes | A list of all pipeline objects. |

## ListPipelineSecretsResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `secrets` | array of [Secret](#secret) | yes | A list of all pipeline secrets. |

## ListPipelinesResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `pipelines` | array of [PipelineSummary](#pipelinesummary) | yes | A list of all pipelines. |

## ListRolesResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `roles` | array of [Role](#role) | yes | A list of all roles. |

## ListRunObjectsResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `objects` | array of [Object](#object) | yes | A list of all run objects. |

## ListRunsResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `runs` | array of [Run](#run) | yes | A list of all runs. |

## ListSubscriptionsResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `subscriptions` | array of [Subscription](#subscription) | yes | A list of all pipeline subscriptions. |

## ListTaskExecutionsResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `task_executions` | array of [TaskExecution](#taskexecution) | yes | A list of all task executions. |

## ListTokensResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `tokens` | array of [Token](#token) | yes | A list of all tokens. |

## Metadata

Details about the pipeline itself, not including the configuration that the user can change. All these values are changed by the system or never changed at all. This sits in contrast to the config which the user can change freely.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `created` | integer | yes | Time of pipeline creation in epoch milliseconds. |
| `disabled_until` | integer | yes | When a disabled pipeline becomes active again, in epoch milliseconds. 0 if it isn't disabled or if it stays that way until someone enables it. |
| `modified` | integer | yes | Time pipeline was updated to a new version in epoch milliseconds. |
| `namespace_id` | string | yes | Unique identifier of the target namespace. |
| `pipeline_id` | string | yes | Unique identifier of the target pipeline. |
| `state` | [PipelineState](#pipelinestate) | yes | The current running state of the pipeline. This is used to determine if the pipeline should run or not. |

## Namespace

A namespace represents a grouping of pipelines. Normally it is used to divide teams or logically different sections of workloads. It is the highest level unit as it sits above pipelines in the hierarchy of Gofer.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `created` | integer | yes | Time in epoch milliseconds when namespace was created. |
| `description` | string | yes | Short description about what the namespace is used for. |
| `id` | string | yes | Unique identifier for the namespace. |
| `modified` | integer | yes | Time in epoch milliseconds when namespace would expire. |
| `name` | string | yes | Humanized name for the namespace. |

## NamespaceGrant

Grants access to resources within matching namespaces and pipelines.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `actions` | array of [Action](#action) | yes |  |
| `namespace` | string | yes | Regex matched against the entire namespace id. Use '.*' to match every namespace. |
| `pipeline` | string (nullable) | no | Regex matched against the entire pipeline id. Leaving it out matches every pipeline. |
| `resources` | array of [NamespaceResource](#namespaceresource) | yes |  |

## NamespaceResource

Things that live under a namespace and pipeline.

A string, one of:

| Value | Description |
| ----- | ----------- |
| `pipelines` |  |
| `configs` |  |
| `deployments` |  |
| `runs` |  |
| `task_executions` |  |
| `objects` |  |
| `secrets` |  |
| `subscriptions` |  |

## Object

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `created` | integer | yes | Time in epoch milliseconds that this object was registered. |
| `key` | string | yes | The identifier for the object value. |

## Parameter

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `default` | string | no | The value used when the operator doesn't set one. Empty means there is no default. |
| `documentation` | string | yes |  |
| `key` | string | yes |  |
| `required` | boolean | yes |  |
| `secret` | boolean | no | Secret params only accept a global secret reference (`global_secret{{key}}`). |

## Pipeline

`Pipeline` represents a sequence of tasks, where each task is a discrete unit of work encapsulated within a container. This structure allows you to organize and define the workflow for the tasks you want to execute. - The ID must be between 3 and 32 characters long and only alphanumeric, hyphens are the only allowed alphanumeric character. Ex. `simple-pipeline` - The name is a human friendly name to represent the pipeline. Ex. `Simple Pipeline`

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `allow_runs_from` | array of string | no | Other pipelines whose runs may start runs of this one, as `namespace/pipeline` patterns. Each half is a regex that has to match the whole id, so `team-a-.*/.*` allows every pipeline in namespaces starting with `team-a-`. Defaults to empty so configs built with older SDKs still parse. |
| `description` | string (nullable) | no | Short description of what the pipeline is used for. |
| `id` | string | yes | Unique user defined identifier. |
| `name` | string | yes | Humanized name, meant for display. |
| `parallelism` | integer | yes | Controls how many runs can be active at any single time. 0 defaults to whatever the global Gofer setting is. |
| `skip_dates` | array of string | no | Days the pipeline doesn't run, as `MM/DD` or `MM/DD-MM/DD`. Either can be limited to a weekday, so `11/22-11/28:thu` is Thanksgiving. Dates repeat every year and are in UTC. Defaults to empty so configs built with older SDKs still parse. |
| `tasks` | array of [Task2](#task2) | yes | A mapping of pipeline owned tasks. |

## Pipeline2

A collection of logically grouped tasks. A task is a unit of work wrapped in a docker container. Pipeline is a secondary level unit being contained within namespaces and containing runs.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `config` | [Config](#config) | yes | User controlled data for the targeted pipeline. |
| `metadata` | [Metadata](#metadata) | yes | Macro level details on the targeted pipeline. |

## PipelineState

A string, one of:

| Value | Description |
| ----- | ----------- |
| `unknown` |  |
| `active` |  |
| `disabled` |  |

## PipelineSummary

A pipeline as it shows up in a listing: its metadata plus the name and description from its newest config, so callers can show something friendlier than the id without fetching every config.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `created` | integer | yes | Time of pipeline creation in epoch milliseconds. |
| `description` | string | yes | Description from the newest registered config. |
| `disabled_until` | integer | yes | When a disabled pipeline becomes active again, in epoch milliseconds. 0 if it isn't disabled or if it stays that way until someone enables it. |
| `modified` | integer | yes | Time pipeline was updated to a new version in epoch milliseconds. |
| `name` | string | yes | Humanized name from the newest registered config. |
| `namespace_id` | string | yes | Unique identifier of the target namespace. |
| `pipeline_id` | string | yes | Unique identifier of the target pipeline. |
| `state` | [PipelineState](#pipelinestate) | yes | The current running state of the pipeline. This is used to determine if the pipeline should run or not. |

## PlanAction

A string, one of:

| Value | Description |
| ----- | ----------- |
| `install` | New to Gofer; it'll be registered and started. |
| `update` | Its config changed; the old container is stopped and a new one started. |
| `start` | Its config is unchanged but it isn't running, usually because it failed to start earlier. |
| `unchanged` | Already running as the config describes. |
| `disable` | Turned off with `enabled = false`; it'll be stopped. |
| `unconfigure` | No longer in the config; it'll be stopped but its subscriptions and data are kept. |
| `invalid` | The config entry has a problem. Nothing will be changed until it's fixed. |

## PlanExtensionsResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `extensions` | array of [PlannedExtension](#plannedextension) | yes | Every extension Gofer knows about or the config mentions, and what applying the config would do to it. |

## PlannedExtension

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `action` | [PlanAction](#planaction) | yes |  |
| `changes` | array of [FieldChange](#fieldchange) | yes | What's different between what's running and what the config describes. |
| `error` | string | yes | Why the config entry is invalid. Empty unless the action is `invalid`. |
| `extension_id` | string | yes |  |
| `manifest` | string | yes | Where the extension's manifest comes from. |
| `plan_hash` | string | yes | Pass this back when applying so Gofer can refuse if anything changed after the plan was made. |

## PutExtensionObjectResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `object` | [Object](#object) | yes | Information about the object created. |

## PutGlobalSecretRequest

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `content` | string | yes | The actual plaintext secret. |
| `force` | boolean | yes | Overwrite a value of a secret if it already exists. |
| `key` | string | yes | The name for the secret you would like to store. |
| `namespaces` | array of string | yes | The namespaces you want this secret to be accessible by. Regexes matched against the entire namespace id. |

## PutGlobalSecretResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `secret` | [Secret](#secret) | yes | Information about the secret created. |

## PutPipelineObjectResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `object` | [Object](#object) | yes | Information about the object created. |

## PutPipelineSecretRequest

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `content` | string | yes | The actual plaintext secret. |
| `force` | boolean | yes | Overwrite a value of a secret if it already exists. |
| `key` | string | yes | The name for the secret you would like to store. |

## PutPipelineSecretResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `secret` | [Secret](#secret) | yes | Information about the secret created. |

## PutRunObjectResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `object` | [Object](#object) | yes | Information about the object created. |

## RegisterPipelineConfigRequest

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `config` | [Pipeline](#pipeline) | yes | The pipeline configuration. This is usually supplied by the CLI which translates written code into this format. |

## RegisterPipelineConfigResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `pipeline` | [Pipeline2](#pipeline2) | yes | The current pipeline. |

## Registration

When installing a new extension, we allow the extension installer to pass a bunch of settings that allow us to go get that extension on future startups.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `additional_roles` | array of string | yes | Additional roles allow the operator to add additional roles to the extension token. This allow extensions to have greater ranges of permissioning than the default. |
| `created` | integer | yes | Time of registration creation in epoch milliseconds. |
| `extension_id` | string | yes | Unique identifier for the extension. |
| `image` | string | yes | Which container image this extension should run. |
| `modified` | integer | yes | Time of last modification in epoch milliseconds. |
| `registry_auth` | [RegistryAuth](#registryauth) (nullable) | no | Auth credentials for the image's registry. The password is a global secret reference. |
| `settings` | array of [Variable](#variable) | yes | The extension's settings from Gofer's config, passed to the extension as env vars. Secrets are stored as global secret references, never the values themselves, so settings are safe to show. |
| `status` | [extension_status](#extension_status) | yes | Whether the extension is enabled or not; extensions can be disabled to prevent use by admins. |

## RegistryAuth

Authentication information for container registries. Extensions use it to pull their images.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `pass` | string | yes |  |
| `user` | string | yes |  |

## RequiredParentStatus

A string, one of:

| Value | Description |
| ----- | ----------- |
| `unknown` |  |
| `any` |  |
| `success` |  |
| `failure` |  |

## RequiredParentStatus2

A string, one of:

| Value | Description |
| ----- | ----------- |
| `Unknown` |  |
| `Any` |  |
| `Success` |  |
| `Failure` |  |

## Role

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `description` | string | yes |  |
| `grants` | [Grants](#grants) | yes |  |
| `id` | string | yes | Alphanumeric with dashes only |
| `system_role` | boolean | yes | If this role was created by Gofer itself. System roles cannot be modified. |

## Run

A run is one or more tasks being executed on behalf of some extension. Run is a third level unit containing tasks and being contained in a pipeline.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `created` | integer | yes | Time the run was created in epoch milliseconds. A run can sit queued behind the pipeline's parallelism limit for a while, so this can be well before `started`. |
| `ended` | integer | yes | Time of run end in epoch milliseconds. |
| `event_id` | string (nullable) | no | The UUID of the QueuedRun event for this run. Essentially pointing to the start of the run in the event stream. This is used internally to help with run recovery. |
| `initiator` | [Initiator](#initiator) | yes | Information about what started the run. |
| `namespace_id` | string | yes | Unique identifier of the target namespace. |
| `pipeline_config_version` | integer | yes | Which version of the pipeline did this run execute. |
| `pipeline_id` | string | yes | Unique identifier of the target pipeline. |
| `run_id` | integer | yes | Unique identifier of the target run. |
| `started` | integer | yes | Time the run left the queue and started launching tasks, in epoch milliseconds. 0 while still queued. |
| `state` | [run_state](#run_state) | yes | The current state of the run within the Gofer execution model. Describes if the run is in progress or not. |
| `status` | [run_status](#run_status) | yes | The final result of the run. |
| `status_reason` | [run_status_reason](#run_status_reason) (nullable) | no | More information on the circumstances around a particular run's status. |
| `store_objects_expired` | boolean | yes | Whether run level objects are deleted. |
| `token_id` | string (nullable) | no | The unique identifier for Gofer's auto-inject token. This feature is so that users can easily use Gofer's API with a ready injected token into the run just-in-time. If this is None this run had no tasks with the `inject_api_token` setting enabled. These tokens automatically expire after a pre-determined time. |
| `variables` | array of [Variable](#variable) | yes | Run level environment variables to be passed to each task execution. |

## Secret

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `created` | integer | yes | Time in epoch milliseconds that this secret was registered. |
| `key` | string | yes | The identifier for the secret value. |
| `namespaces` | array of string | yes | The namespaces this secret is allowed to be accessed from. Regexes matched against the entire namespace id. |

## StartRunRequest

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `force` | boolean | no | Start the run even if today is one of the pipeline's skip dates. It doesn't start runs of a disabled pipeline. |
| `variables` | map of string to string | yes | Environment variables for the run's tasks. They override variables of the same name from the pipeline config and from Gofer itself, except `GOFER_API_BASE_URL` and `GOFER_TOKEN`, which a run can't set. |

## StartRunResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `run` | [Run](#run) | yes | Information about the run started. |

## Step

One of the variants below. Variants without a payload are sent as a plain string; the rest are an object with the variant name as its only key.

| Variant | Payload | Description |
| ------- | ------- | ----------- |
| `copy` | `from`: string<br>`to`: string | Copies a file, or what's inside a folder, into the image. `from` is relative to the folder of the pipeline's config, and leaving it is fine, like `../shared`. |
| `run` | `command`: string | Runs a command with `/bin/sh -c`. |

## Steps

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `base` | string | yes | The image to start from, like `rust:1-trixie`. |
| `keep` | [Keep](#keep) (nullable) | no | Moves what it lists onto another base image once every step has run, leaving everything else behind. |
| `steps` | array of [Step](#step) | yes |  |

## Subscription

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `extension_id` | string | yes | Unique identifier of the target extension. |
| `namespace_id` | string | yes | Unique identifier of the target namespace. |
| `pipeline_id` | string | yes | Unique identifier of the target pipeline. |
| `settings` | map of string to string | yes | The extension's pipeline configuration settings. |
| `status` | [subscription_status](#subscription_status) | yes | Whether the subscription is working. `error` means the extension behind it isn't installed or isn't running, so the subscription does nothing until it's back. |
| `status_reason` | [subscription_status_reason](#subscription_status_reason) (nullable) | no | Why the subscription is in `error`, including what's wrong with the extension. |
| `subscription_id` | string | yes | A unique label differentiating this subscription from other subscriptions. |

## Task

A task represents a particular workload within a pipeline. Tasks are composable within a larger pipeline, meaning they can be run before, after, or alongside other tasks. Tasks represent the lowest level of the Gofer hierarchy and is what Gofer references to see how a user might want their workload handled.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `build` | [Build](#build) | yes | How the task's image is made. `gofer up` builds it on the user's machine before it registers the config. |
| `command` | array of string (nullable) | no | Replaces the image's command; follows normal docker convention of command: https://docs.docker.com/reference/dockerfile/#cmd. It's already built into the image. |
| `depends_on` | map of string to [RequiredParentStatus](#requiredparentstatus) | yes | Which other tasks (by id) this task depends on. |
| `description` | string | yes | Short description about the workload. |
| `entrypoint` | array of string (nullable) | no | Replaces the image's entrypoint; follows normal docker convention for entrypoint: https://docs.docker.com/reference/dockerfile/#entrypoint. It's already built into the image. |
| `id` | string | yes | Unique identifier for the task. |
| `image` | [BuiltImage](#builtimage) | yes | The image that was built for this version of the task, which is what its runs use. |
| `inject_api_token` | boolean | yes | Whether to inject a run specific Gofer API key. Useful for using Gofer API within the container. |
| `timeout_seconds` | integer | yes | How long, in seconds, the task's container can run before Gofer stops it and fails the task. 0 means the server's `api.task_execution_max_timeout_seconds` applies, which also caps anything set here. |
| `variables` | array of [Variable](#variable) | yes | Variables which will be passed in as env vars to the task. |

## Task2

Represents a single task within a `Pipeline`. A task is a unit of work that operates within its own container. Each task defines the operations to be performed and the container environment in which these operations will run.

- The ID must be between 3 and 32 characters long and only alphanumeric, hyphens are the only allowed alphanumeric character. Ex. `simple-pipeline` - The name is a human friendly name to represent the pipeline. Ex. `Simple Pipeline`

Gofer builds every task's image itself, when you run `gofer up`. A task starts from a base image, copies in your files and runs commands, or it points at a Dockerfile.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `build` | [Build](#build) | yes | How the task's image is made. |
| `command` | array of string (nullable) | no | Replaces the image's command. It's built into the image. |
| `depends_on` | map of string to [RequiredParentStatus2](#requiredparentstatus2) | yes |  |
| `description` | string (nullable) | no |  |
| `entrypoint` | array of string (nullable) | no | Replaces the image's entrypoint. It's built into the image. |
| `id` | string | yes |  |
| `image` | [BuiltImage](#builtimage) (nullable) | no | The image `gofer up` built. A config straight from the SDK doesn't have one; `gofer up` fills it in before it registers the config. |
| `inject_api_token` | boolean | yes |  |
| `timeout_seconds` | integer | no | How long the task's container can run before Gofer stops it and fails the task. 0 means the server's max task execution timeout applies. Defaults to 0 so configs built with older SDKs still parse. |
| `variables` | map of string to string | yes |  |

## TaskExecution

a task execution is a specific execution of a task/container. It represents a 4th level unit in the hierarchy. namespace -> pipeline -> run -> task execution. It is the last and most specific object in Gofer's execution model.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `created` | integer | yes | Time of task execution creation in epoch milliseconds. |
| `ended` | integer | yes | Time of task execution end in epoch milliseconds. |
| `exit_code` | integer (nullable) | no | The exit code of the task execution completion if it is finished. |
| `logs_expired` | boolean | yes | Whether the logs have past their retention time. |
| `logs_removed` | boolean | yes | If the logs for this execution have been removed. This can be due to user request or automatic action based on expiry time. |
| `namespace_id` | string | yes | Unique identifier of the target namespace. |
| `pipeline_id` | string | yes | Unique identifier of the target pipeline. |
| `run_id` | integer | yes | Unique identifier of the target run. |
| `started` | integer | yes | Time of task execution start in epoch milliseconds. |
| `state` | [task_execution_state](#task_execution_state) | yes | The current state of the task execution within the Gofer execution model. Describes if the execution is in progress or not. |
| `status` | [task_execution_status](#task_execution_status) | yes | The final result of the task execution. |
| `status_reason` | [task_execution_status_reason](#task_execution_status_reason) (nullable) | no | More information on the circumstances around a particular task execution's status. |
| `task` | [Task](#task) | yes | Information about the underlying task this task execution ran. |
| `task_id` | string | yes | Unique identifier of the current task being executed. |
| `timeline` | array of [task_execution_timeline_entry](#task_execution_timeline_entry) | yes | The phases the task execution went through in order, so you can tell where its time went. |
| `variables` | array of [Variable](#variable) | yes | The environment variables injected during this particular task execution. |

## Token

Gofer API Token.

The hash field is skipped during serialization to prevent it from being exposed to the user. This isn't a foolproof practice, but it'll work for now.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `created` | integer | yes | Time in epoch milliseconds when token was created. |
| `disabled` | boolean | yes | If the token is inactive or not; disabled tokens cannot be used for requests. |
| `expires` | integer | yes | Time in epoch milliseconds when token would expire. An expiry of 0 means that token does not expire. |
| `id` | string | yes | Unique identifier for token. |
| `kind` | [token_kind](#token_kind) | yes | What the token was created for. |
| `metadata` | map of string to string | yes | Extra information about this token in label form |
| `roles` | array of string | yes | The role ids for the current token. |
| `user` | string | yes | The user of the token in plaintext. |

## UpdateNamespaceRequest

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `description` | string (nullable) | no | Short description about what the namespace is used for. |
| `name` | string (nullable) | no | Humanized name for the namespace. |

## UpdateNamespaceResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `namespace` | [Namespace](#namespace) | yes | Information about the namespace updated. |

## UpdatePipelineRequest

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `disabled_until` | integer (nullable) | no | When the pipeline becomes active again by itself, in epoch milliseconds. Can only be set along with the state `disabled`, and has to be in the future. Leave it out to keep the pipeline disabled until it's enabled again. |
| `state` | [PipelineState](#pipelinestate) (nullable) | no |  |

## UpdateRoleRequest

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `description` | string (nullable) | no | Short description about what the role is used for. |
| `grants` | [Grants](#grants) (nullable) | no | Replaces everything the role allows. |

## UpdateRoleResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `role` | [Role](#role) | yes | Information about the role updated. |

## UpdateSubscriptionRequest

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `status` | [UpdateSubscriptionStatus](#updatesubscriptionstatus) (nullable) | no |  |

## UpdateSubscriptionStatus

A string, one of:

| Value | Description |
| ----- | ----------- |
| `active` |  |
| `disabled` |  |

## UpdateSystemPreferencesRequest

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `ignore_pipeline_run_events` | boolean (nullable) | no |  |

## UpdateSystemPreferencesResponse

An object with no defined fields.

## UpdateTokenRequest

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `disabled` | boolean (nullable) | no |  |

## Variable

A variable is a key value pair that is used either at a run or task level. The variable is inserted as an environment variable to an eventual task execution. It can be owned by different parts of the system which control where the potentially sensitive variables might show up.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `key` | string | yes |  |
| `source` | [VariableSource](#variablesource) | yes |  |
| `value` | string | yes |  |

## VariableSource

A string, one of:

| Value | Description |
| ----- | ----------- |
| `unknown` |  |
| `pipeline_config` | From the user's own pipeline configuration. |
| `system` | From the Gofer API executor itself. |
| `run_options` | Injected at the beginning of a particular run. |
| `extension` | Injected by a subscribed extension. |

## WhoAmIResponse

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `token` | [Token](#token) | yes | The target token. |

## deployment_state

A string, one of:

| Value | Description |
| ----- | ----------- |
| `Running` |  |
| `Complete` |  |
| `Unknown` | Should never be in this state. |

## deployment_status

A string, one of:

| Value | Description |
| ----- | ----------- |
| `Unknown` | Should only be in this state if the deployment is not yet complete. |
| `Failed` | Has encountered an issue, either container issue or scheduling issue. |
| `Successful` | Finished with a proper exit code. |

## deployment_status_reason

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `description` | string | yes | A description of why the deployment might have failed and what was going on at the time. |
| `reason` | [deployment_status_reason_type](#deployment_status_reason_type) | yes | The specific type of deployment failure. |

## deployment_status_reason_type

A string, one of:

| Value | Description |
| ----- | ----------- |
| `Unknown` |  |

## extension_state

A string, one of:

| Value | Description |
| ----- | ----------- |
| `unknown` | Should never be in this state. |
| `processing` | Being checked and prepared before its container starts. |
| `running` | Its container is running, as Docker reports it. |
| `degraded` | Running, but it stopped answering health checks. Gofer keeps sending it work and restarts it if it doesn't recover. The extension's `state_reason` has the last error. |
| `exited` | Extension has exited; usually because of an error. |
| `stopped` | Not running on purpose, because it's disabled or no longer in the config. |
| `failed` | Gofer couldn't start the extension. The extension's `state_reason` says why. |

## extension_status

A string, one of:

| Value | Description |
| ----- | ----------- |
| `unknown` | Cannot determine status of Extension; should never be in this status. |
| `enabled` | Installed and able to be used by pipelines. |
| `disabled` | Turned off in Gofer's config with `enabled = false`. |
| `unconfigured` | Installed at some point but no longer listed in Gofer's config. Its subscriptions and data are kept so adding it back to the config picks up where it left off; `gofer extension purge` deletes them for good. |

## run_state

The current state of the run. The state is described as the progress of the run towards completion.

A string, one of:

| Value | Description |
| ----- | ----------- |
| `unknown` |  |
| `pending` | Queued and waiting to start. A run stays pending until its pipeline is under its parallelism limit and Gofer is under its global limit, and while run events are paused. |
| `running` | Currently running. |
| `complete` | All tasks have been resolved and the run is no longer being executed. |

## run_status

The current status of the run. Status is described as if the run succeeded or not.

A string, one of:

| Value | Description |
| ----- | ----------- |
| `unknown` | Could not determine the current state of the status. Should only be in this state if the run has not yet completed. |
| `failed` | One or more tasks in run have failed. |
| `successful` | All tasks in a run have completed with a non-failure state. |
| `cancelled` | One or more tasks in a run have been cancelled. |
| `skipped` | The run was never started since it was asked for on one of the pipeline's skip dates. |

## run_status_reason

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `description` | string | yes | A description of why the run might have failed and what was going on at the time. |
| `reason` | [run_status_reason_type](#run_status_reason_type) | yes | The specific type of run failure. |

## run_status_reason_type

A string, one of:

| Value | Description |
| ----- | ----------- |
| `unknown` | Gofer has no fucking clue how the run got into this state. |
| `abnormal_exit` | While executing the run, one or more tasks exited with an abnormal exit code. |
| `container_error` | While executing the run, one or more tasks' images couldn't be loaded, or their containers couldn't be started. |
| `failed_precondition` | The run could not be executed as requested due to user defined attributes given. |
| `user_cancelled` | One or more tasks could not be completed due to a user cancelling the run. |
| `admin_cancelled` | One or more tasks could not be completed due to the system or admin cancelling the run. |
| `timeout` | One or more tasks ran longer than their timeout and were stopped. |
| `skip_date` | The run was asked for on one of the pipeline's skip dates. |

## subscription_status

A string, one of:

| Value | Description |
| ----- | ----------- |
| `unknown` |  |
| `active` |  |
| `error` |  |
| `disabled` |  |

## subscription_status_reason

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `description` | string | yes | A description of why the subscription might have failed and what was going on at the time. |
| `reason` | [subscription_status_reason_type](#subscription_status_reason_type) | yes | The specific type of subscription failure. |

## subscription_status_reason_type

A string, one of:

| Value | Description |
| ----- | ----------- |
| `unknown` |  |
| `not_found` |  |
| `failed` |  |

## task_execution_phase

A step a task execution goes through on its way to finishing. Used to break down where a task execution spent its time, especially the time before its container actually started running.

A string, one of:

| Value | Description |
| ----- | ----------- |
| `waiting_on_dependencies` | Waiting for the tasks it depends on to finish. |
| `preparing` | Resolving variables, which includes reading from the secret and object stores. |
| `loading_image` | Docker is loading the task's image from Gofer's image store. Skipped when Docker already has it. |
| `starting_container` | Docker is creating and starting the container. |
| `running` | The container is running. |

## task_execution_state

A string, one of:

| Value | Description |
| ----- | ----------- |
| `complete` |  |
| `unknown` | Should never be in this state. |
| `processing` | Being checked and prepared before its container starts. |
| `waiting` | Waiting for its container to start. |
| `running` | Its container is running, as Docker reports it. |

## task_execution_status

A string, one of:

| Value | Description |
| ----- | ----------- |
| `unknown` |  |
| `failed` | Has encountered an issue, either container issue or scheduling issue. |
| `successful` | Finished with a proper exit code. |
| `cancelled` | Cancelled mid run due to user requested cancellation. |
| `skipped` | Not run due to dependencies not being met. |

## task_execution_status_reason

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `description` | string | yes | A description of why the task execution might have failed and what was going on at the time. |
| `reason` | [task_execution_status_reason_type](#task_execution_status_reason_type) | yes | The specific type of task execution failure. |

## task_execution_status_reason_type

A string, one of:

| Value | Description |
| ----- | ----------- |
| `unknown` | Gofer has no fucking clue how the run got into this state. |
| `abnormal_exit` | A non-zero exit code has been received. |
| `container_error` | Docker couldn't load the task's image, or couldn't start or watch its container. |
| `failed_precondition` | User error in task execution parameters. |
| `cancelled` | User invoked cancellation.k |
| `orphaned` | Task execution was lost due to extreme internal error. |
| `timeout` | The container ran longer than its timeout and was stopped. The description says which timeout it hit. |

## task_execution_timeline_entry

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `phase` | [task_execution_phase](#task_execution_phase) | yes | The phase the task execution entered. |
| `started` | integer | yes | When the phase began in epoch milliseconds. A phase lasts until the next entry begins, or until the task execution ended if it's the last one. |

## token_kind

What a token was created for. Gofer sets this itself and it can't be chosen through the API, so it can be trusted.

A string, one of:

| Value | Description |
| ----- | ----------- |
| `user` | A token created through the API, usually for a person. |
| `extension` | A token Gofer creates for a running extension. |
| `run` | A token Gofer creates for a run so its tasks can talk to the API (the `inject_api_token` feature). |
| `bootstrap` | The one-time token used to set up a new Gofer instance. |
| `anonymous` | Not a real token. Gofer uses it for requests that came in without one, either because `bypass_auth` is on or because the route allows anonymous reads. No stored token ever has this kind. |

