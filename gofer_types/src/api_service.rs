use crate::api::{
    deployment::*, event::*, extension::*, namespace::*, object::*, permissioning::*, pipeline::*,
    pipeline_config::*, run::*, secret::*, subscription::*, system::*, task_execution::*, token::*,
};
use dropshot::{
    HttpError, HttpResponseCreated, HttpResponseDeleted, HttpResponseOk,
    HttpResponseUpdatedNoContent, Path, Query, RequestContext, TypedBody, WebsocketChannelResult,
    WebsocketConnection,
};

#[dropshot::api_description]
trait ApiService {
    type Context;

    /// List all deployments.
    #[endpoint(
      method = GET,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/deployments",
      tags = ["Deployments"],
    )]
    async fn list_deployments(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<DeploymentPathArgsRoot>,
    ) -> Result<HttpResponseOk<ListDeploymentsResponse>, HttpError>;

    /// Get api deployment by id.
    #[endpoint(
      method = GET,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/deployments/{deployment_id}",
      tags = ["Deployments"],
    )]
    async fn get_deployment(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<DeploymentPathArgs>,
    ) -> Result<HttpResponseOk<GetDeploymentResponse>, HttpError>;

    /// List all events.
    #[channel(
      protocol = WEBSOCKETS,
      path = "/api/events",
      tags = ["Events"],
    )]
    async fn stream_events(
        rqctx: RequestContext<Self::Context>,
        query_params: Query<EventQueryArgs>,
        conn: WebsocketConnection,
    ) -> WebsocketChannelResult;

    /// Get api event by id.
    #[endpoint(
      method = GET,
      path = "/api/events/{event_id}",
      tags = ["Events"],
    )]
    async fn get_event(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<EventPathArgs>,
    ) -> Result<HttpResponseOk<GetEventResponse>, HttpError>;

    // List all extensions currently registered.
    #[endpoint(
      method = GET,
      path = "/api/extensions",
      tags = ["Extensions"],
    )]
    async fn list_extensions(
        rqctx: RequestContext<Self::Context>,
    ) -> Result<HttpResponseOk<ListExtensionsResponse>, HttpError>;

    /// Returns details about a specific extension.
    #[endpoint(
      method = GET,
      path = "/api/extensions/{extension_id}",
      tags = ["Extensions"],
    )]
    async fn get_extension(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<ExtensionPathArgs>,
    ) -> Result<HttpResponseOk<GetExtensionResponse>, HttpError>;

    /// Register and start a new extension.
    ///
    /// This route is only available to admin tokens.
    #[endpoint(
      method = POST,
      path = "/api/extensions",
      tags = ["Extensions"],
    )]
    async fn install_extension(
        rqctx: RequestContext<Self::Context>,
        body: TypedBody<InstallExtensionRequest>,
    ) -> Result<HttpResponseCreated<InstallExtensionResponse>, HttpError>;

    /// Enable or disable an extension.
    ///
    /// This route is only accessible for admin tokens.
    #[endpoint(
      method = PATCH,
      path = "/api/extensions/{extension_id}",
      tags = ["Extensions"],
    )]
    async fn update_extension(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<ExtensionPathArgs>,
        body: TypedBody<UpdateExtensionRequest>,
    ) -> Result<HttpResponseUpdatedNoContent, HttpError>;

    /// Uninstall a registered extension.
    ///
    /// This route is only accessible for admin tokens.
    #[endpoint(
      method = DELETE,
      path = "/api/extensions/{extension_id}",
      tags = ["Extensions"],
    )]
    async fn uninstall_extension(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<ExtensionPathArgs>,
    ) -> Result<HttpResponseDeleted, HttpError>;

    // Retrieves logs from the extension container.
    #[channel(
      protocol = WEBSOCKETS,
      path = "/api/extensions/{extension_id}/logs",
      tags = ["Extensions"],
    )]
    async fn get_extension_logs(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<ExtensionPathArgs>,
        conn: WebsocketConnection,
    ) -> WebsocketChannelResult;

    /// List all namespaces.
    #[endpoint(
      method = GET,
      path = "/api/namespaces",
      tags = ["Namespaces"],
)]
    async fn list_namespaces(
        rqctx: RequestContext<Self::Context>,
    ) -> Result<HttpResponseOk<ListNamespacesResponse>, HttpError>;

    /// Get api namespace by id.
    #[endpoint(
      method = GET,
      path = "/api/namespaces/{namespace_id}",
      tags = ["Namespaces"],
    )]
    async fn get_namespace(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<NamespacePathArgs>,
    ) -> Result<HttpResponseOk<GetNamespaceResponse>, HttpError>;

    /// Update a namespace's details.
    ///
    /// This route is only accessible for admin tokens.
    #[endpoint(
      method = PATCH,
      path = "/api/namespaces/{namespace_id}",
      tags = ["Namespaces"],
    )]
    async fn update_namespace(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<NamespacePathArgs>,
        body: TypedBody<UpdateNamespaceRequest>,
    ) -> Result<HttpResponseOk<UpdateNamespaceResponse>, HttpError>;

    /// Delete api namespace by id.
    ///
    /// This route is only accessible for admin tokens.
    #[endpoint(
      method = DELETE,
      path = "/api/namespaces/{namespace_id}",
      tags = ["Namespaces"],
    )]
    async fn delete_namespace(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<NamespacePathArgs>,
    ) -> Result<HttpResponseDeleted, HttpError>;

    /// List all run objects.
    #[endpoint(
      method = GET,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/objects",
      tags = ["Objects"],
    )]
    async fn list_run_objects(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<RunObjectPathArgsRoot>,
    ) -> Result<HttpResponseOk<ListRunObjectsResponse>, HttpError>;

    /// Get run object by key.
    #[endpoint(
      method = GET,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/objects/{key}",
      tags = ["Objects"],
    )]
    async fn get_run_object(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<RunObjectPathArgs>,
    ) -> Result<HttpResponseOk<GetRunObjectResponse>, HttpError>;

    /// Insert a new object into the run object store.
    #[endpoint(
      method = POST,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/objects",
      tags = ["Objects"],
    )]
    async fn put_run_object(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<RunObjectPathArgsRoot>,
        body: TypedBody<PutRunObjectRequest>,
    ) -> Result<HttpResponseCreated<PutRunObjectResponse>, HttpError>;

    /// Delete run object by key.
    #[endpoint(
      method = DELETE,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/objects/{key}",
      tags = ["Objects"],
    )]
    async fn delete_run_object(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<RunObjectPathArgs>,
    ) -> Result<HttpResponseDeleted, HttpError>;

    /// List all pipeline objects.
    #[endpoint(
      method = GET,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/objects",
      tags = ["Objects"],
    )]
    async fn list_pipeline_objects(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<PipelineObjectPathArgsRoot>,
    ) -> Result<HttpResponseOk<ListPipelineObjectsResponse>, HttpError>;

    /// Get pipeline object by key.
    #[endpoint(
      method = GET,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/objects/{key}",
      tags = ["Objects"],
    )]
    async fn get_pipeline_object(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<PipelineObjectPathArgs>,
    ) -> Result<HttpResponseOk<GetPipelineObjectResponse>, HttpError>;

    /// Insert a new object into the pipeline object store.
    #[endpoint(
      method = POST,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/objects",
      tags = ["Objects"],
    )]
    async fn put_pipeline_object(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<PipelineObjectPathArgsRoot>,
        body: TypedBody<PutPipelineObjectRequest>,
    ) -> Result<HttpResponseCreated<PutPipelineObjectResponse>, HttpError>;

    /// Delete pipeline object by key.
    #[endpoint(
      method = DELETE,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/objects/{key}",
      tags = ["Objects"],
    )]
    async fn delete_pipeline_object(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<PipelineObjectPathArgs>,
    ) -> Result<HttpResponseDeleted, HttpError>;

    /// List all extension objects.
    #[endpoint(
      method = GET,
      path = "/api/extensions/{extension_id}/objects",
      tags = ["Objects"],
    )]
    async fn list_extension_objects(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<ExtensionObjectPathArgsRoot>,
    ) -> Result<HttpResponseOk<ListExtensionObjectsResponse>, HttpError>;

    /// Get extension object by key.
    #[endpoint(
      method = GET,
      path = "/api/extensions/{extension_id}/objects/{key}",
      tags = ["Objects"],
    )]
    async fn get_extension_object(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<ExtensionObjectPathArgs>,
    ) -> Result<HttpResponseOk<GetExtensionObjectResponse>, HttpError>;

    /// Insert a new object into the extension object store.
    #[endpoint(
      method = POST,
      path = "/api/extensions/{extension_id}/objects",
      tags = ["Objects"],
    )]
    async fn put_extension_object(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<ExtensionObjectPathArgsRoot>,
        body: TypedBody<PutExtensionObjectRequest>,
    ) -> Result<HttpResponseCreated<PutExtensionObjectResponse>, HttpError>;

    /// Delete extension object by key.
    #[endpoint(
      method = DELETE,
      path = "/api/extensions/{extension_id}/objects/{key}",
      tags = ["Objects"],
    )]
    async fn delete_extension_object(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<ExtensionObjectPathArgs>,
    ) -> Result<HttpResponseDeleted, HttpError>;

    /// List all roles.
    #[endpoint(
      method = GET,
      path = "/api/roles",
      tags = ["Permissions"],
    )]
    async fn list_roles(
        rqctx: RequestContext<Self::Context>,
    ) -> Result<HttpResponseOk<ListRolesResponse>, HttpError>;

    /// Get api role by id.
    #[endpoint(
      method = GET,
      path = "/api/roles/{role_id}",
      tags = ["Permissions"],
    )]
    async fn get_role(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<RolePathArgs>,
    ) -> Result<HttpResponseOk<GetRoleResponse>, HttpError>;

    /// Create a new role.
    ///
    /// This route is only accessible for admin tokens.
    #[endpoint(
      method = POST,
      path = "/api/roles",
      tags = ["Permissions"],
    )]
    async fn create_role(
        rqctx: RequestContext<Self::Context>,
        body: TypedBody<CreateRoleRequest>,
    ) -> Result<HttpResponseCreated<CreateRoleResponse>, HttpError>;

    /// Update a role's details.
    ///
    /// This route is only accessible for admin tokens.
    #[endpoint(
      method = PATCH,
      path = "/api/roles/{role_id}",
      tags = ["Permissions"],
    )]
    async fn update_role(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<RolePathArgs>,
        body: TypedBody<UpdateRoleRequest>,
    ) -> Result<HttpResponseOk<UpdateRoleResponse>, HttpError>;

    /// Delete api role by id.
    ///
    /// This route is only accessible for admin tokens.
    #[endpoint(
      method = DELETE,
      path = "/api/roles/{role_id}",
      tags = ["Permissions"],
    )]
    async fn delete_role(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<RolePathArgs>,
    ) -> Result<HttpResponseDeleted, HttpError>;

    /// List all pipeline configs.
    ///
    /// A pipeline's config is the small program you write to configure how you want your pipeline to run.
    #[endpoint(
      method = GET,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs",
      tags = ["Configs"],
    )]
    async fn list_configs(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<PipelineConfigPathArgsRoot>,
    ) -> Result<HttpResponseOk<ListPipelineConfigsResponse>, HttpError>;

    /// Get a specific version of a pipeline configuration.
    ///
    /// A version of 0 indicates to return the latest pipeline config.
    #[endpoint(
      method = GET,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs/{version}",
      tags = ["Configs"],
    )]
    async fn get_config(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<PipelineConfigPathArgs>,
    ) -> Result<HttpResponseOk<GetPipelineConfigResponse>, HttpError>;

    /// Register a new pipeline configuration.
    ///
    /// This creates both the pipeline metadata and the initial config object.
    #[endpoint(
      method = POST,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs",
      tags = ["Configs"],
    )]
    async fn register_config(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<PipelineConfigPathArgsRoot>,
        body: TypedBody<RegisterPipelineConfigRequest>,
    ) -> Result<HttpResponseCreated<RegisterPipelineConfigResponse>, HttpError>;

    /// Deploy pipeline config.
    #[endpoint(
      method = POST,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs/{version}",
      tags = ["Configs"],
    )]
    async fn deploy_config(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<PipelineConfigPathArgs>,
    ) -> Result<HttpResponseCreated<DeployPipelineConfigResponse>, HttpError>;

    /// Delete pipeline config by version.
    #[endpoint(
      method = DELETE,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs/{version}",
      tags = ["Configs"],
    )]
    async fn delete_config(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<PipelineConfigPathArgs>,
    ) -> Result<HttpResponseDeleted, HttpError>;

    /// List all pipelines.
    ///
    /// Returns the metadata for all pipelines. If you want a more complete picture of the pipeline details
    /// combine this endpoint with the configs endpoint to grab the metadata AND the user's pipeline configuration.
    #[endpoint(
      method = GET,
      path = "/api/namespaces/{namespace_id}/pipelines",
      tags = ["Pipelines"],
    )]
    async fn list_pipelines(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<PipelinePathArgsRoot>,
    ) -> Result<HttpResponseOk<ListPipelinesResponse>, HttpError>;

    /// Get pipeline by id.
    #[endpoint(
      method = GET,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}",
      tags = ["Pipelines"],
    )]
    async fn get_pipeline(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<PipelinePathArgs>,
    ) -> Result<HttpResponseOk<GetPipelineResponse>, HttpError>;

    /// Update a pipeline's state.
    #[endpoint(
      method = PATCH,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}",
      tags = ["Pipelines"],
    )]
    async fn update_pipeline(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<PipelinePathArgs>,
        body: TypedBody<UpdatePipelineRequest>,
    ) -> Result<HttpResponseUpdatedNoContent, HttpError>;

    /// Delete pipeline by id.
    ///
    /// IMPORTANT: Deleting a pipeline is set to cascade. All downstream objects to the pipeline (configs, secrets, runs, tasks)
    /// will be removed as well.
    #[endpoint(
      method = DELETE,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}",
      tags = ["Pipelines"],
    )]
    async fn delete_pipeline(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<PipelinePathArgs>,
    ) -> Result<HttpResponseDeleted, HttpError>;

    /// List all runs.
    ///
    /// Returns a list of all runs by pipeline id.
    #[endpoint(
      method = GET,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs",
      tags = ["Runs"],
    )]
    async fn list_runs(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<RunPathArgsRoot>,
        query_params: Query<ListRunsQueryArgs>,
    ) -> Result<HttpResponseOk<ListRunsResponse>, HttpError>;

    /// Get run by id.
    #[endpoint(
      method = GET,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}",
      tags = ["Runs"],
    )]
    async fn get_run(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<RunPathArgs>,
    ) -> Result<HttpResponseOk<GetRunResponse>, HttpError>;

    /// Start a run of a particular pipeline.
    #[endpoint(
      method = POST,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs",
      tags = ["Runs"],
    )]
    async fn start_run(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<RunPathArgsRoot>,
        body: TypedBody<StartRunRequest>,
    ) -> Result<HttpResponseCreated<StartRunResponse>, HttpError>;

    /// Cancel a run by id.
    #[endpoint(
      method = DELETE,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}",
      tags = ["Runs"],
    )]
    async fn cancel_run(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<RunPathArgs>,
    ) -> Result<HttpResponseDeleted, HttpError>;

    /// List all global secrets.
    ///
    /// Admin tokens required.
    #[endpoint(
      method = GET,
      path = "/api/secrets/global",
      tags = ["Secrets"],
    )]
    async fn list_global_secrets(
        rqctx: RequestContext<Self::Context>,
    ) -> Result<HttpResponseOk<ListGlobalSecretsResponse>, HttpError>;

    /// Get global secret by key.
    ///
    /// Admin token required.
    #[endpoint(
      method = GET,
      path = "/api/secrets/global/{key}",
      tags = ["Secrets"],
    )]
    async fn get_global_secret(
        rqctx: RequestContext<Self::Context>,
        query_params: Query<GetGlobalSecretQueryArgs>,
        path_params: Path<GlobalSecretPathArgs>,
    ) -> Result<HttpResponseOk<GetGlobalSecretResponse>, HttpError>;

    /// Insert a new secret into the global secret store.
    ///
    /// This route is only accessible for admin tokens.
    #[endpoint(
      method = POST,
      path = "/api/secrets/global",
      tags = ["Secrets"],
    )]
    async fn put_global_secret(
        rqctx: RequestContext<Self::Context>,
        body: TypedBody<PutGlobalSecretRequest>,
    ) -> Result<HttpResponseCreated<PutGlobalSecretResponse>, HttpError>;

    /// Delete global secret by key.
    ///
    /// This route is only accessible for admin tokens.
    #[endpoint(
      method = DELETE,
      path = "/api/secrets/global/{key}",
      tags = ["Secrets"],
    )]
    async fn delete_global_secret(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<GlobalSecretPathArgs>,
    ) -> Result<HttpResponseDeleted, HttpError>;

    /// List all pipeline secrets.
    #[endpoint(
      method = GET,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/secrets",
      tags = ["Secrets"],
    )]
    async fn list_pipeline_secrets(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<PipelineSecretPathArgsRoot>,
    ) -> Result<HttpResponseOk<ListPipelineSecretsResponse>, HttpError>;

    /// Get pipeline secret by key.
    #[endpoint(
      method = GET,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/secrets/{key}",
      tags = ["Secrets"],
    )]
    async fn get_pipeline_secret(
        rqctx: RequestContext<Self::Context>,
        query_params: Query<GetPipelineSecretQueryArgs>,
        path_params: Path<PipelineSecretPathArgs>,
    ) -> Result<HttpResponseOk<GetPipelineSecretResponse>, HttpError>;

    /// Insert a new secret into the pipeline secret store.
    #[endpoint(
      method = POST,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/secrets",
      tags = ["Secrets"],
    )]
    async fn put_pipeline_secret(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<PipelineSecretPathArgsRoot>,
        body: TypedBody<PutPipelineSecretRequest>,
    ) -> Result<HttpResponseCreated<PutPipelineSecretResponse>, HttpError>;

    /// Delete pipeline secret by key.
    #[endpoint(
      method = DELETE,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/secrets/{key}",
      tags = ["Secrets"],
    )]
    async fn delete_pipeline_secret(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<PipelineSecretPathArgs>,
    ) -> Result<HttpResponseDeleted, HttpError>;

    /// List all subscriptions.
    #[endpoint(
      method = GET,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/subscriptions",
      tags = ["Subscriptions"],
    )]
    async fn list_subscriptions(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<SubscriptionPathArgsRoot>,
    ) -> Result<HttpResponseOk<ListSubscriptionsResponse>, HttpError>;

    /// Get subscription by id.
    #[endpoint(
      method = GET,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/subscriptions/{extension_id}/{subscription_id}",
      tags = ["Subscriptions"],
    )]
    async fn get_subscription(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<SubscriptionPathArgs>,
    ) -> Result<HttpResponseOk<GetSubscriptionResponse>, HttpError>;

    /// Update a subscription's state.
    #[endpoint(
      method = PATCH,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/subscriptions/{extension_id}/{subscription_id}",
      tags = ["Subscriptions"],
    )]
    async fn update_subscription(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<SubscriptionPathArgs>,
        body: TypedBody<UpdateSubscriptionRequest>,
    ) -> Result<HttpResponseUpdatedNoContent, HttpError>;

    /// Create a new subscription.
    #[endpoint(
      method = POST,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/subscriptions",
      tags = ["Subscriptions"],
    )]
    async fn create_subscription(
        rqctx: RequestContext<Self::Context>,
        path: Path<SubscriptionPathArgsRoot>,
        body: TypedBody<CreateSubscriptionRequest>,
    ) -> Result<HttpResponseCreated<CreateSubscriptionResponse>, HttpError>;

    /// Delete subscription by id.
    #[endpoint(
      method = DELETE,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/subscriptions/{extension_id}/{subscription_id}",
      tags = ["Subscriptions"],
    )]
    async fn delete_subscription(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<SubscriptionPathArgs>,
    ) -> Result<HttpResponseDeleted, HttpError>;

    /// Describe current system meta-information.
    ///
    /// Return a number of internal metadata about the Gofer service itself.
    #[endpoint(
      method = GET,
      path = "/api/system/metadata",
      tags = ["System"],
    )]
    async fn get_metadata(
        rqctx: RequestContext<Self::Context>,
    ) -> Result<HttpResponseOk<GetSystemMetadataResponse>, HttpError>;

    /// Get system parameters.
    #[endpoint(
      method = GET,
      path = "/api/system",
      tags = ["System"],
    )]
    async fn get_system_preferences(
        rqctx: RequestContext<Self::Context>,
    ) -> Result<HttpResponseOk<GetSystemPreferencesResponse>, HttpError>;

    /// Update system parameters.
    #[endpoint(
      method = PATCH,
      path = "/api/system",
      tags = ["System"],
    )]
    async fn update_system_preferences(
        rqctx: RequestContext<Self::Context>,
        body: TypedBody<UpdateSystemPreferencesRequest>,
    ) -> Result<HttpResponseOk<UpdateSystemPreferencesResponse>, HttpError>;

    /// List all task executions.
    ///
    /// Returns a list of all task executions by run.
    #[endpoint(
      method = GET,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks",
      tags = ["Tasks"],
    )]
    async fn list_task_executions(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<TaskExecutionPathArgsRoot>,
    ) -> Result<HttpResponseOk<ListTaskExecutionsResponse>, HttpError>;

    /// Get task execution by id.
    #[endpoint(
      method = GET,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}",
      tags = ["Tasks"],
    )]
    async fn get_task_execution(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<TaskExecutionPathArgs>,
    ) -> Result<HttpResponseOk<GetTaskExecutionResponse>, HttpError>;

    /// Cancel a task execution by id.
    #[endpoint(
      method = DELETE,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}",
      tags = ["Tasks"],
    )]
    async fn cancel_task_execution(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<TaskExecutionPathArgs>,
        query_params: Query<CancelTaskExecutionQueryArgs>,
    ) -> Result<HttpResponseDeleted, HttpError>;

    /// Retrieves logs from a task execution.
    #[channel(
      protocol = WEBSOCKETS,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}/logs",
      tags = ["Tasks"],
    )]
    async fn get_logs(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<TaskExecutionPathArgs>,
        conn: WebsocketConnection,
    ) -> WebsocketChannelResult;

    /// Removes a task execution's associated log object.
    ///
    /// This is useful for if logs mistakenly contain sensitive data.
    #[endpoint(
      method = DELETE,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}/logs",
      tags = ["Tasks"],
    )]
    async fn delete_logs(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<TaskExecutionPathArgs>,
    ) -> Result<HttpResponseDeleted, HttpError>;

    /// Run command on a running task execution container.
    ///
    /// This allows you to run a command on a task execution container and connect to the stdin and stdout/err for said
    /// container.
    ///
    /// Useful for debugging.
    #[channel(
      protocol = WEBSOCKETS,
      path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}/attach",
      tags = ["Tasks"],
    )]
    async fn attach_task_execution(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<TaskExecutionPathArgs>,
        query_params: Query<AttachTaskExecutionQueryParams>,
        socket_conn: WebsocketConnection,
    ) -> WebsocketChannelResult;

    /// List all Gofer API tokens.
    ///
    /// This endpoint is restricted to admin tokens only.
    #[endpoint(
      method = GET,
      path = "/api/tokens",
      tags = ["Tokens"],
    )]
    async fn list_tokens(
        rqctx: RequestContext<Self::Context>,
    ) -> Result<HttpResponseOk<ListTokensResponse>, HttpError>;

    /// Get api token by id.
    #[endpoint(
      method = GET,
      path = "/api/tokens/{id}",
      tags = ["Tokens"]
    )]
    async fn get_token_by_id(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<TokenPathArgs>,
    ) -> Result<HttpResponseOk<GetTokenByIDResponse>, HttpError>;

    /// Get api token who made the request.
    #[endpoint(
      method = GET,
      path = "/api/tokens/whoami",
      tags = ["Tokens"]
    )]
    async fn whoami(
        rqctx: RequestContext<Self::Context>,
    ) -> Result<HttpResponseOk<WhoAmIResponse>, HttpError>;

    /// Create a new token.
    ///
    /// This endpoint is restricted to admin tokens only.
    #[endpoint(
      method = POST,
      path = "/api/tokens",
      tags = ["Tokens"]
    )]
    async fn create_token(
        rqctx: RequestContext<Self::Context>,
        body: TypedBody<CreateTokenRequest>,
    ) -> Result<HttpResponseCreated<CreateTokenResponse>, HttpError>;

    /// Delete api token by id.
    ///
    /// This endpoint is restricted to admin tokens only.
    #[endpoint(
      method = DELETE,
      path = "/api/tokens/{id}",
      tags = ["Tokens"],
    )]
    async fn delete_token(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<TokenPathArgs>,
    ) -> Result<HttpResponseDeleted, HttpError>;

    /// Create root admin token.
    ///
    /// This endpoint can only be hit once and will create the root admin token,
    /// from which all other tokens can be created.
    #[endpoint(
      method = POST,
      path = "/api/tokens/bootstrap",
      tags = ["Tokens"]
    )]
    async fn create_bootstrap_token(
        rqctx: RequestContext<Self::Context>,
    ) -> Result<HttpResponseCreated<CreateTokenResponse>, HttpError>;

    /// Update a token's state.
    #[endpoint(
      method = PATCH,
      path = "/api/tokens/{id}",
      tags = ["Tokens"],
    )]
    async fn update_token(
        rqctx: RequestContext<Self::Context>,
        path_params: Path<TokenPathArgs>,
        body: TypedBody<UpdateTokenRequest>,
    ) -> Result<HttpResponseUpdatedNoContent, HttpError>;
}
