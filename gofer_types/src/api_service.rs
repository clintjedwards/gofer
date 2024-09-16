use crate::api::{deployment::*, event::*, extension::*};
use dropshot::{
    HttpError, HttpResponseCreated, HttpResponseDeleted, HttpResponseOk,
    HttpResponseUpdatedNoContent, Path, Query, RequestContext, TypedBody, WebsocketChannelResult,
    WebsocketConnection,
};

#[dropshot::api_description]
pub trait ApiService {
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
}
