use crate::{
    api::{
        deployments, epoch_milli, event_utils, is_valid_identifier, pipelines, tasks, ApiState,
        PreflightOptions,
    },
    http_error,
    storage::{self, StorageError},
};
use anyhow::{Context, Result};
use dropshot::{
    endpoint, HttpError, HttpResponseCreated, HttpResponseDeleted, HttpResponseOk, Path,
    RequestContext, TypedBody,
};
use gofer_sdk::config;
use http::StatusCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::Acquire;
use std::sync::Arc;
use std::{collections::HashMap, str::FromStr};
use strum::{Display, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PipelineConfigPathArgsRoot {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PipelineConfigPathArgs {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,

    /// The version of the configuration you want to target. 0 means return the latest.
    pub version: i64,
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum ConfigState {
    #[default]
    Unknown,

    /// Has never been deployed.
    Unreleased,

    /// Currently deployed.
    Live,

    /// Has previously been deployed and is now defunct.
    Deprecated,
}

/// A representation of the user's configuration settings for a particular pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Config {
    /// Unique identifier of the target namespace.
    pub namespace_id: String,

    /// Unique identifier of the target pipeline.
    pub pipeline_id: String,

    /// The iteration number for this pipeline's configs.
    pub version: u64,

    /// The amount of runs allowed to happen at any given time.
    pub parallelism: u64,

    /// Human readable name for pipeline.
    pub name: String,

    /// Description of pipeline's purpose and other details.
    pub description: String,

    /// Tasks associated with this pipeline.
    pub tasks: HashMap<String, tasks::Task>,

    /// The deployment state of the config. This is used to determine the state of this particular config and if it
    /// is currently being used or not.
    pub state: ConfigState,

    /// Time in epoch milliseconds when this pipeline config was registered.
    pub registered: u64,

    /// Time in epoch milliseconds when this pipeline config was not longer used.
    pub deprecated: u64,
}

impl Config {
    pub fn new(
        namespace_id: &str,
        pipeline_id: &str,
        version: u64,
        config: gofer_sdk::config::Pipeline,
    ) -> Result<Self> {
        Ok(Config {
            namespace_id: namespace_id.into(),
            pipeline_id: pipeline_id.into(),
            version,
            parallelism: config.parallelism.try_into()?,
            name: config.name,
            description: config.description.unwrap_or_default(),
            tasks: config
                .tasks
                .into_iter()
                .map(|task| (task.id.clone(), tasks::Task::from(task)))
                .collect(),
            state: ConfigState::Unreleased,
            registered: epoch_milli(),
            deprecated: 0,
        })
    }
}

impl Config {
    pub fn to_storage(
        &self,
    ) -> Result<(
        storage::pipeline_configs::PipelineConfig,
        Vec<storage::tasks::Task>,
    )> {
        let config = storage::pipeline_configs::PipelineConfig {
            namespace_id: self.namespace_id.clone(),
            pipeline_id: self.pipeline_id.clone(),
            version: self.version.try_into()?,
            parallelism: self.parallelism.try_into()?,
            name: self.name.clone(),
            description: self.description.clone(),
            registered: self.registered.to_string(),
            deprecated: self.deprecated.to_string(),
            state: self.state.to_string(),
        };

        let mut tasks: Vec<storage::tasks::Task> = vec![];
        for task in self.tasks.values() {
            let storage_task = task
                .to_storage(
                    self.namespace_id.clone(),
                    self.pipeline_id.clone(),
                    self.version.try_into()?,
                )
                .context("Could not properly serialize task to DB")?;

            tasks.push(storage_task);
        }

        Ok((config, tasks))
    }

    pub fn from_storage(
        config: storage::pipeline_configs::PipelineConfig,
        tasks: Vec<storage::tasks::Task>,
    ) -> Result<Self> {
        let registered = config.registered.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'registered' from storage value '{}'",
                config.registered
            )
        })?;

        let deprecated = config.deprecated.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'deprecated' from storage value '{}'",
                config.deprecated
            )
        })?;

        let state = ConfigState::from_str(&config.state).with_context(|| {
            format!(
                "Could not parse field 'state' from storage value '{}'",
                config.state
            )
        })?;

        Ok(Config {
            namespace_id: config.namespace_id,
            pipeline_id: config.pipeline_id,
            version: config.version.try_into()?,
            parallelism: config.parallelism.try_into()?,
            name: config.name,
            description: config.description,
            tasks: tasks
                .into_iter()
                .map(|task| {
                    (
                        task.task_id.clone(),
                        tasks::Task::from_storage(task).unwrap(),
                    )
                })
                .collect(),
            state,
            registered,
            deprecated,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListPipelineConfigsResponse {
    /// A list of all pipelines configs.
    pub configs: Vec<Config>,
}

/// List all pipeline configs.
///
/// A pipeline's config is the small program you write to configure how you want your pipeline to run.
#[endpoint(
    method = GET,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs",
    tags = ["Configs"],
)]
pub async fn list_configs(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<PipelineConfigPathArgsRoot>,
) -> Result<HttpResponseOk<ListPipelineConfigsResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                check_namespace: Some(path.namespace_id.clone()),
                management_only: false,
            },
        )
        .await?;

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let mut tx = match conn.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return Err(http_error!(
                "Could not open database transaction",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let storage_configs =
        match storage::pipeline_configs::list(&mut tx, &path.namespace_id, &path.pipeline_id).await
        {
            Ok(pipelines) => pipelines,
            Err(e) => {
                return Err(http_error!(
                    "Could not get config objects from database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        };

    let mut configs: Vec<Config> = vec![];

    for storage_config in storage_configs {
        let tasks = match storage::tasks::list(
            &mut tx,
            &path.namespace_id,
            &path.pipeline_id,
            storage_config.version,
        )
        .await
        {
            Ok(tasks) => tasks,
            Err(e) => {
                return Err(http_error!(
                    "Could not get task objects from database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        };

        let config = Config::from_storage(storage_config, tasks).map_err(|e| {
            http_error!(
                "Could not parse object from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        configs.push(config);
    }

    if let Err(e) = tx.commit().await {
        return Err(http_error!(
            "Could not close database transaction",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    let resp = ListPipelineConfigsResponse { configs };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetPipelineConfigResponse {
    /// The target pipeline config.
    pub config: Config,
}

/// Get a specific version of a pipeline configuration.
///
/// A version of 0 indicates to return the latest pipeline config.
#[endpoint(
    method = GET,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs/{version}",
    tags = ["Configs"],
)]
pub async fn get_config(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<PipelineConfigPathArgs>,
) -> Result<HttpResponseOk<GetPipelineConfigResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                check_namespace: Some(path.namespace_id.clone()),
                management_only: false,
            },
        )
        .await?;

    let mut version = path.version;

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let mut tx = match conn.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return Err(http_error!(
                "Could not open database transaction",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    if version == 0 {
        let latest_config = match storage::pipeline_configs::get_latest(
            &mut tx,
            &path.namespace_id,
            &path.pipeline_id,
        )
        .await
        {
            Ok(pipeline) => pipeline,
            Err(e) => match e {
                storage::StorageError::NotFound => {
                    return Err(HttpError::for_not_found(None, String::new()));
                }
                _ => {
                    return Err(http_error!(
                        "Could not get latest object from database",
                        http::StatusCode::INTERNAL_SERVER_ERROR,
                        rqctx.request_id.clone(),
                        Some(e.into())
                    ));
                }
            },
        };

        version = latest_config.version
    }

    let storage_pipeline_config = match storage::pipeline_configs::get(
        &mut tx,
        &path.namespace_id,
        &path.pipeline_id,
        version,
    )
    .await
    {
        Ok(pipeline) => pipeline,
        Err(e) => match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(None, String::new()));
            }
            _ => {
                return Err(http_error!(
                    "Could not get config objects from database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        },
    };

    let storage_tasks =
        match storage::tasks::list(&mut tx, &path.namespace_id, &path.pipeline_id, version).await {
            Ok(tasks) => tasks,
            Err(e) => {
                return Err(http_error!(
                    "Could not get task objects from database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        };

    let config = Config::from_storage(storage_pipeline_config, storage_tasks).map_err(|err| {
        http_error!(
            "Could not parse object from database",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(err.into())
        )
    })?;

    if let Err(e) = tx.commit().await {
        return Err(http_error!(
            "Could not close database transaction",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    let resp = GetPipelineConfigResponse { config };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RegisterPipelineConfigRequest {
    /// The pipeline configuration. This is usually supplied by the CLI which translates written code into
    /// this format.
    pub config: config::Pipeline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RegisterPipelineConfigResponse {
    /// The current pipeline.
    pub pipeline: pipelines::Pipeline,
}

/// Register a new pipeline configuration.
///
/// This creates both the pipeline metadata and the initial config object.
#[endpoint(
    method = POST,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs",
    tags = ["Configs"],
)]
pub async fn register_config(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<PipelineConfigPathArgsRoot>,
    body: TypedBody<RegisterPipelineConfigRequest>,
) -> Result<HttpResponseCreated<RegisterPipelineConfigResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let body = body.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                check_namespace: Some(path.namespace_id.clone()),
                management_only: false,
            },
        )
        .await?;

    if let Err(e) = is_valid_identifier(&body.config.id) {
        return Err(HttpError::for_bad_request(
            None,
            format!(
                "'{}' is not a valid identifier for pipeline id; {}",
                &body.config.id,
                &e.to_string()
            ),
        ));
    };

    if path.pipeline_id != body.config.id {
        return Err(HttpError::for_bad_request(
            None,
            "pipeline_id in URL path does not match pipeline id in configuration".into(),
        ));
    };

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let mut tx = match conn.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return Err(http_error!(
                "Could not open database transaction",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let new_pipeline_metadata = pipelines::Metadata::new(&path.namespace_id, &path.pipeline_id);

    if let Err(e) =
        storage::pipeline_metadata::insert(&mut tx, &new_pipeline_metadata.clone().into()).await
    {
        match e {
            storage::StorageError::Exists => {
                // If the pipeline already exists that just means we shouldn't create a new one from scratch.
            }
            _ => {
                return Err(http_error!(
                    "Could not insert object into database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id,
                    Some(e.into())
                ));
            }
        }
    };

    let latest_config: Option<storage::pipeline_configs::PipelineConfig> =
        match storage::pipeline_configs::get_latest(&mut tx, &path.namespace_id, &path.pipeline_id)
            .await
        {
            Ok(pipeline) => Some(pipeline),
            Err(e) => match e {
                storage::StorageError::NotFound => None,
                _ => {
                    return Err(http_error!(
                        "Could not get latest config object from database",
                        http::StatusCode::INTERNAL_SERVER_ERROR,
                        rqctx.request_id,
                        Some(e.into())
                    ));
                }
            },
        };

    let last_version = match latest_config {
        Some(latest_config) => latest_config.version,
        None => 0,
    };
    let last_version: u64 = last_version
        .try_into()
        .map_err(|err: std::num::TryFromIntError| {
            http_error!(
                "Could not serialize last_version while attempting to determine next version",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(err.into())
            )
        })?;

    let new_pipeline_config = Config::new(
        &path.namespace_id,
        &path.pipeline_id,
        last_version + 1,
        body.config,
    )
    .map_err(|err| {
        http_error!(
            "Could not create new pipeline config object",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(err.into())
        )
    })?;

    let (storage_config, storage_task_configs) =
        new_pipeline_config.to_storage().map_err(|err| {
            http_error!(
                "Could not parse object from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(err.into())
            )
        })?;

    if let Err(e) = storage::pipeline_configs::insert(&mut tx, &storage_config).await {
        match e {
            storage::StorageError::Exists => {
                return Err(HttpError::for_client_error(
                    None,
                    StatusCode::CONFLICT,
                    "pipeline config entry already exists".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not insert object into database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id,
                    Some(e.into())
                ));
            }
        }
    };

    for storage_task_config in storage_task_configs {
        if let Err(e) = storage::tasks::insert(&mut tx, &storage_task_config).await {
            match e {
                storage::StorageError::Exists => {
                    return Err(HttpError::for_client_error(
                        None,
                        StatusCode::CONFLICT,
                        "pipeline task entry already exists".into(),
                    ));
                }
                _ => {
                    return Err(http_error!(
                        "Could not insert task object into database",
                        http::StatusCode::INTERNAL_SERVER_ERROR,
                        rqctx.request_id,
                        Some(e.into())
                    ));
                }
            }
        };
    }

    if let Err(e) = tx.commit().await {
        return Err(http_error!(
            "Could not close database transaction",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id,
            Some(e.into())
        ));
    };

    api_state
        .event_bus
        .clone()
        .publish(event_utils::Kind::CreatedPipeline {
            namespace_id: new_pipeline_metadata.namespace_id.clone(),
            pipeline_id: new_pipeline_metadata.pipeline_id.clone(),
        });

    let resp = RegisterPipelineConfigResponse {
        pipeline: pipelines::Pipeline {
            metadata: new_pipeline_metadata,
            config: new_pipeline_config,
        },
    };

    Ok(HttpResponseCreated(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DeployPipelineConfigResponse {
    /// Information about the pipeline created.
    pub deployment: deployments::Deployment,
}

/// Deploy pipeline config.
#[endpoint(
    method = POST,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs/{version}",
    tags = ["Configs"],
)]
pub async fn deploy_config(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<PipelineConfigPathArgs>,
) -> Result<HttpResponseCreated<DeployPipelineConfigResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                check_namespace: Some(path.namespace_id.clone()),
                management_only: true,
            },
        )
        .await?;

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let mut tx = match conn.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return Err(http_error!(
                "Could not open database transaction",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let end_version = path.version;

    // Step 1: Insert the new deployment

    // First we check that there are no currently running deployments
    match storage::deployments::list_running(&mut tx, &path.namespace_id, &path.pipeline_id).await {
        Ok(running) => {
            if !running.is_empty() {
                return Err(HttpError::for_client_error(
                    None,
                    http::StatusCode::CONFLICT,
                    format!(
                        "Deployment '{}' is already in progress",
                        running.first().unwrap().deployment_id
                    ),
                ));
            }
        }
        Err(err) => {
            return Err(http_error!(
                "Could not get objects from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(err.into())
            ));
        }
    };

    // Get the latest live config so we can deprecate it.
    let latest_live_config = match storage::pipeline_configs::get_latest_w_state(
        &mut tx,
        &path.namespace_id,
        &path.pipeline_id,
        &ConfigState::Live.to_string(),
    )
    .await
    {
        Ok(config) => Some(config),
        Err(err) => {
            if err == StorageError::NotFound {
                None
            } else {
                return Err(http_error!(
                    "Could not get latest live config object from database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(err.into())
                ));
            }
        }
    };

    // Set start version; if there are no live pipeline configurations set the one being deployed to be the starting
    // version.
    let start_version = match latest_live_config {
        Some(config) => config.version,
        None => path.version,
    };

    // Finally get the latest deployment so we can increment the id by one.
    let mut latest_deployment_id = 0;

    let latest_deployment = match storage::deployments::get_latest(
        &mut tx,
        &path.namespace_id,
        &path.pipeline_id,
    )
    .await
    {
        Ok(deployment) => Some(deployment),
        Err(err) => {
            if err == StorageError::NotFound {
                None
            } else {
                return Err(http_error!(
                    "Could not retrieve latest config while attempting to deploy new config",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(err.into())
                ));
            }
        }
    };

    if let Some(deployment) = latest_deployment {
        latest_deployment_id = deployment.deployment_id;
    };
    let latest_deployment_id = latest_deployment_id; // Return immutability for latest_deployment_id variable.
    let new_deployment_id = latest_deployment_id + 1;

    // I'm sorry about the fallible casting, I'm lazy.
    let new_deployment = deployments::Deployment::new(
        &path.namespace_id,
        &path.pipeline_id,
        new_deployment_id as u64,
        start_version as u64,
        end_version as u64,
    );

    let storage_deployment = new_deployment
        .clone()
        .try_into()
        .map_err(|err: anyhow::Error| {
            http_error!(
                "Could not parse object from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(err.into())
            )
        })?;

    storage::deployments::insert(&mut tx, &storage_deployment)
        .await
        .map_err(|err| {
            http_error!(
                "Could not insert object from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(err.into())
            )
        })?;

    if let Err(e) = tx.commit().await {
        return Err(http_error!(
            "Could not database transaction",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    // Step 2: Officially start the deployment.
    api_state
        .event_bus
        .clone()
        .publish(event_utils::Kind::StartedDeployment {
            namespace_id: path.namespace_id.clone(),
            pipeline_id: path.pipeline_id.clone(),
            start_version: start_version as u64,
            end_version: end_version as u64,
        });

    // Step 3: We mark the new pipeline config as Live and Active, signifying that it is ready to take traffic.
    // If this wasn't a same version upgrade. We mark the old pipeline config as Deprecated and Disabled.
    // TODO(clintjedwards): Eventually this will become a more intricate function which will allow for more
    // complex deployment types.

    let mut tx = match conn.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return Err(http_error!(
                "Could not open database transaction",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    // Update end version config
    storage::pipeline_configs::update(
        &mut tx,
        &path.namespace_id,
        &path.pipeline_id,
        end_version,
        storage::pipeline_configs::UpdatableFields {
            state: Some(ConfigState::Live.to_string()),
            ..Default::default()
        },
    )
    .await
    .map_err(|err| {
        http_error!(
            "Could not update end_version pipeline config",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(err.into())
        )
    })?;

    // Update start version config
    if start_version != end_version {
        storage::pipeline_configs::update(
            &mut tx,
            &path.namespace_id,
            &path.pipeline_id,
            start_version,
            storage::pipeline_configs::UpdatableFields {
                state: Some(ConfigState::Deprecated.to_string()),
                deprecated: Some(epoch_milli().to_string()),
            },
        )
        .await
        .map_err(|err| {
            http_error!(
                "Could not update start_version pipeline config",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(err.into())
            )
        })?;
    }

    if let Err(e) = tx.commit().await {
        let status_reason_json = serde_json::to_string(&deployments::StatusReason {
            reason: deployments::StatusReasonType::Unknown,
            description: format!("Deployment has failed due to an internal error; {:#?}", e),
        })
        .ok();

        // Mark deployment as failed
        storage::deployments::update(
            &mut conn,
            &path.namespace_id,
            &path.pipeline_id,
            new_deployment_id,
            storage::deployments::UpdatableFields {
                ended: Some(epoch_milli().to_string()),
                state: Some(deployments::State::Complete.to_string()),
                status: Some(deployments::Status::Failed.to_string()),
                status_reason: status_reason_json,
                ..Default::default()
            },
        )
        .await
        .ok();

        return Err(http_error!(
            "Could not close database transaction",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    // Complete deployment
    storage::deployments::update(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
        new_deployment_id,
        storage::deployments::UpdatableFields {
            ended: Some(epoch_milli().to_string()),
            state: Some(deployments::State::Complete.to_string()),
            status: Some(deployments::Status::Successful.to_string()),
            ..Default::default()
        },
    )
    .await
    .map_err(|err| {
        http_error!(
            "Could not update object to database",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(err.into())
        )
    })?;

    // Lastly: We're done. So now we just need to complete the deployment.
    api_state
        .event_bus
        .clone()
        .publish(event_utils::Kind::CompletedDeployment {
            namespace_id: path.namespace_id.clone(),
            pipeline_id: path.pipeline_id.clone(),
            start_version: start_version as u64,
            end_version: end_version as u64,
        });

    let resp = DeployPipelineConfigResponse {
        deployment: new_deployment,
    };

    Ok(HttpResponseCreated(resp))
}

/// Delete pipeline config by version.
#[endpoint(
    method = DELETE,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs/{version}",
    tags = ["Configs"],
)]
pub async fn delete_config(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<PipelineConfigPathArgs>,
) -> Result<HttpResponseDeleted, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                check_namespace: Some(path.namespace_id.clone()),
                management_only: false,
            },
        )
        .await?;

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let mut tx = match conn.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return Err(http_error!(
                "Could not open database transaction",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let latest_config =
        match storage::pipeline_configs::get_latest(&mut tx, &path.namespace_id, &path.pipeline_id)
            .await
        {
            Ok(pipeline) => pipeline,
            Err(e) => match e {
                storage::StorageError::NotFound => {
                    return Err(HttpError::for_not_found(None, String::new()));
                }
                _ => {
                    return Err(http_error!(
                        "Could not get object from database",
                        http::StatusCode::INTERNAL_SERVER_ERROR,
                        rqctx.request_id.clone(),
                        Some(e.into())
                    ));
                }
            },
        };

    if latest_config.version == path.version {
        return Err(HttpError::for_bad_request(None,
            "Cannot delete latest config version; please upload a new one and then delete an older one.".into()));
    }

    let config = match storage::pipeline_configs::get(
        &mut tx,
        &path.namespace_id,
        &path.pipeline_id,
        path.version,
    )
    .await
    {
        Ok(pipeline) => pipeline,
        Err(e) => match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(None, String::new()));
            }
            _ => {
                return Err(http_error!(
                    "Could not get object from database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        },
    };

    let config_state = ConfigState::from_str(&config.state).map_err(|err| {
        http_error!(
            "Could not parse object from database",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(err.into())
        )
    })?;

    if config_state == ConfigState::Live {
        return Err(HttpError::for_bad_request(None,
            "Cannot delete a live configuration; Please deploy a new config and then delete the old one.".into()));
    }

    if let Err(e) = storage::pipeline_configs::delete(
        &mut tx,
        &path.namespace_id,
        &path.pipeline_id,
        path.version,
    )
    .await
    {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "pipeline config for version given does not exist".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not delete object from database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    if let Err(e) = tx.commit().await {
        return Err(http_error!(
            "Could not close database transaction",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    api_state
        .event_bus
        .clone()
        .publish(event_utils::Kind::DeletedPipeline {
            namespace_id: path.namespace_id,
            pipeline_id: path.pipeline_id,
        });

    Ok(HttpResponseDeleted())
}
