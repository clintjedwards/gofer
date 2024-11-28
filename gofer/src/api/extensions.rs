use crate::{
    api::{
        epoch_milli, event_utils, format_duration, listen_for_terminate_signal, load_tls,
        permissioning::{Action, InternalPermission, InternalRole, Resource},
        subscriptions, tokens, websocket_error, ApiState, PreflightOptions, RegistryAuth, Variable,
        VariableSource,
    },
    http_error,
    scheduler::{self, GetLogsRequest},
    storage,
};
use anyhow::{anyhow, bail, Context, Result};
use dropshot::{
    channel, endpoint, HttpError, HttpResponseCreated, HttpResponseDeleted, HttpResponseOk,
    HttpResponseUpdatedNoContent, Path, RequestContext, TypedBody, WebsocketChannelResult,
    WebsocketConnection,
};
use futures::{SinkExt, StreamExt};
use reqwest::{header, Client};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr, sync::Arc};
use strum::{Display, EnumString};
use tracing::{debug, error, info};
use tungstenite::protocol::{frame::coding::CloseCode, Role};
use tungstenite::Message;

/// The address Gofer tells the extension it should bind to on startup.
const EXTENSION_BIND_ADDRESS: &str = "0.0.0.0:8082";

fn extension_container_id(id: &str) -> String {
    format!("extension_{id}")
}

fn extension_role_id(extension_id: &str) -> String {
    format!("extension_{extension_id}")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExtensionPathArgs {
    /// The unique identifier for the target extension.
    pub extension_id: String,
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
#[schemars(rename = "extension_state")]
pub enum State {
    /// Should never be in this state.
    #[default]
    Unknown,

    /// Pre-scheduling validation and prep.
    Processing,

    /// Currently running as reported by scheduler.
    Running,

    /// Extension has exited; usually because of an error.
    Exited,
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
#[schemars(rename = "extension_status")]
pub enum Status {
    /// Cannot determine status of Extension; should never be in this status.
    #[default]
    Unknown,

    /// Installed and able to be used by pipelines.
    Enabled,

    /// Not available to be used by pipelines, either through lack of installation or being disabled by an admin.
    Disabled,
}

/// When installing a new extension, we allow the extension installer to pass a bunch of settings that allow us to
/// go get that extension on future startups.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Registration {
    /// Unique identifier for the extension.
    pub extension_id: String,

    /// Which container image this extension should run.
    pub image: String,

    /// Auth credentials for the image's registry.
    pub registry_auth: Option<RegistryAuth>,

    /// Extensions allow configuration through env vars passed to them through this field. Refer to the extension's
    /// documentation for setting values.
    pub settings: Vec<Variable>,

    /// Time of registration creation in epoch milliseconds.
    pub created: u64,

    /// Time of last modification in epoch milliseconds.
    pub modified: u64,

    /// Whether the extension is enabled or not; extensions can be disabled to prevent use by admins.
    pub status: Status,

    /// Additional roles allow the operator to add additional roles to the extension token. This allow extensions to
    /// have greater ranges of permissioning than the default.
    pub additional_roles: Vec<String>,

    /// Gofer creates an API key that it passes to extensions on start up in order to facilitate extensions talking
    /// back to the Gofer API. This is the identifier for that key.
    #[serde(skip)]
    key_id: String,
}

impl TryFrom<storage::extension_registrations::ExtensionRegistration> for Registration {
    type Error = anyhow::Error;

    fn try_from(value: storage::extension_registrations::ExtensionRegistration) -> Result<Self> {
        let created = value.created.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'created' from storage value '{}'",
                value.created
            )
        })?;

        let modified = value.modified.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'modified' from storage value '{}'",
                value.modified
            )
        })?;

        let status = Status::from_str(&value.status).with_context(|| {
            format!(
                "Could not parse field 'status' from storage value '{}'",
                value.status
            )
        })?;

        let registry_auth = serde_json::from_str(&value.registry_auth).with_context(|| {
            format!(
                "Could not parse field 'registry_auth' from storage value; '{:#?}'",
                value.registry_auth
            )
        })?;

        let settings = serde_json::from_str(&value.settings).with_context(|| {
            format!(
                "Could not parse field 'settings' from storage value; '{:#?}'",
                value.settings
            )
        })?;

        let additional_roles =
            serde_json::from_str(&value.additional_roles).with_context(|| {
                format!(
                    "Could not parse field 'additional_roles' from storage value; '{:#?}'",
                    value.additional_roles
                )
            })?;

        Ok(Registration {
            extension_id: value.extension_id,
            image: value.image,
            registry_auth,
            settings,
            created,
            modified,
            status,
            additional_roles,
            key_id: value.key_id,
        })
    }
}

impl TryFrom<Registration> for storage::extension_registrations::ExtensionRegistration {
    type Error = anyhow::Error;

    fn try_from(value: Registration) -> Result<Self> {
        let registry_auth = serde_json::to_string(&value.registry_auth).with_context(|| {
            format!(
                "Could not parse field 'registry_auth' to storage value; '{:#?}'",
                value.registry_auth
            )
        })?;

        let settings = serde_json::to_string(&value.settings).with_context(|| {
            format!(
                "Could not parse field 'settings' to storage value; '{:#?}'",
                value.settings
            )
        })?;

        let additional_roles =
            serde_json::to_string(&value.additional_roles).with_context(|| {
                format!(
                    "Could not parse field 'additional_roles' to storage value; '{:#?}'",
                    value.additional_roles
                )
            })?;

        Ok(Self {
            extension_id: value.extension_id,
            image: value.image,
            registry_auth,
            settings,
            created: value.created.to_string(),
            modified: value.modified.to_string(),
            status: value.status.to_string(),
            additional_roles,
            key_id: value.key_id,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Parameter {
    pub key: String,
    pub required: bool,
    pub documentation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Documentation {
    /// Each extension has configuration parameters that can be passed in at extension startup. These parameters
    /// should control extension behavior for it's entire lifetime.
    pub config_params: Vec<Parameter>,

    /// Each extension has pipeline subscription parameters that are passed in by a pipeline when it attempts to
    /// subscribe to an extension. This controls how the extension treats that specific pipeline subscription.
    pub pipeline_subscription_params: Vec<Parameter>,

    /// Anything the extension wants to explain to the user. This text is inserted into the documentation a user
    /// can look up about the extension. Supports AsciiDoc.
    pub body: String,
}

impl From<gofer_sdk::extension::api::types::Documentation> for Documentation {
    fn from(value: gofer_sdk::extension::api::types::Documentation) -> Self {
        Documentation {
            config_params: value
                .config_params
                .into_iter()
                .map(|param| Parameter {
                    key: param.key,
                    required: param.required,
                    documentation: param.documentation,
                })
                .collect(),
            pipeline_subscription_params: value
                .pipeline_subscription_params
                .into_iter()
                .map(|param| Parameter {
                    key: param.key,
                    required: param.required,
                    documentation: param.documentation,
                })
                .collect(),
            body: value.body,
        }
    }
}

/// An Extension is the way that pipelines add extra functionality to themselves. Pipelines can "subscribe" to
/// extensions and extensions then act on behalf of that pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Extension {
    /// Metadata about the extension as it is registered within Gofer.
    pub registration: Registration,

    /// The network address used to communicate with the extension by the main process.
    pub url: String,

    /// The start time of the extension in epoch milliseconds.
    pub started: u64,

    /// The current state of the extension as it exists within Gofer's operating model.
    pub state: State,

    /// Extension given documentation usually in markdown.
    pub documentation: Documentation,

    /// Key is an extension's authentication key used to validate requests from the Gofer main service. On every
    /// request the Gofer main service passes this key so that it is impossible for others to contact and manipulate
    /// extensions directly.
    #[serde(skip)]
    pub secret: String,
}

async fn start_extension(
    api_state: Arc<ApiState>,
    registration: Registration,
) -> Result<Extension> {
    // First we create a new token for the extension and then update the registration with the key_id.

    let token_roles = [
        vec![extension_role_id(&registration.extension_id)],
        registration.additional_roles.clone(),
    ]
    .concat();

    let (token, hash) = tokens::create_new_api_token();
    let new_token = tokens::Token::new(
        &hash.to_string(),
        HashMap::from([("extension_id".into(), registration.extension_id.clone())]),
        0, // Do not expire token.
        format!("{} (extension)", registration.extension_id.clone()),
        token_roles,
    );

    let mut tx = match api_state.storage.open_tx().await {
        Ok(conn) => conn,
        Err(e) => {
            error!(message = "Could not open connection to database", error = %e);
            bail!("Could not open connection to database")
        }
    };

    // If there was a previous key then just delete it, if there wasn't we don't particularly care.
    let _ = storage::tokens::delete(&mut tx, &registration.key_id).await;

    let registration_key_id = new_token.id.clone();

    let storage_new_token = new_token.try_into().map_err(|err| {
        anyhow!(
            "Could not serialize new token while attempting to start extension; {:#?}",
            err
        )
    })?;

    storage::tokens::insert(&mut tx, &storage_new_token)
        .await
        .map_err(|err| anyhow!("Could not insert token into storage; {:#?}", err))?;

    storage::extension_registrations::update(
        &mut tx,
        &registration.extension_id,
        storage::extension_registrations::UpdatableFields {
            image: None,
            registry_auth: None,
            settings: None,
            status: None,
            modified: epoch_milli().to_string(),
            additional_roles: None,
            key_id: Some(registration_key_id),
        },
    )
    .await
    .map_err(|err| anyhow!("Could not update registration in storage; {:#?}", err))?;

    if let Err(e) = tx.commit().await {
        error!(message = "Could not close transaction from database", error = %e);
        bail!("Could not close transaction from database; {:#?}", e)
    };

    // If we need to use TLS then load the keys provided, if not then just pass empty strings
    // as they'll never be used.
    let (cert, key) = if api_state.config.extensions.use_tls {
        load_tls(
            api_state.config.development.use_included_certs,
            api_state.config.extensions.tls_cert_path.clone(),
            api_state.config.extensions.tls_key_path.clone(),
        )
        .context("Could not load extension TLS keys")?
    } else {
        (vec![], vec![])
    };

    // Next we prep to start the Extension.
    //
    // We first need to populate the extension with their required environment variables.
    // These are passed to every extension.

    let system_extension_vars: Vec<Variable> = vec![
        Variable {
            key: "GOFER_EXTENSION_SYSTEM_USE_TLS".into(),
            value: api_state.config.extensions.use_tls.to_string(),
            source: VariableSource::System,
        },
        Variable {
            key: "GOFER_EXTENSION_SYSTEM_TLS_CERT".into(),
            value: String::from_utf8_lossy(&cert).to_string(),
            source: VariableSource::System,
        },
        Variable {
            key: "GOFER_EXTENSION_SYSTEM_TLS_KEY".into(),
            value: String::from_utf8_lossy(&key).to_string(),
            source: VariableSource::System,
        },
        // The extension id is simply a human readable name for the extension that also acts as the extension's unique ID
        // among all other extensions.
        Variable {
            key: "GOFER_EXTENSION_SYSTEM_ID".into(),
            value: registration.extension_id.clone(),
            source: VariableSource::System,
        },
        Variable {
            key: "GOFER_EXTENSION_SYSTEM_LOG_LEVEL".into(),
            value: api_state.config.api.log_level.clone(),
            source: VariableSource::System,
        },
        // The system key is a token generated for the sole purpose of authentication between Gofer and the Extension.
        // It serves as a pre-shared auth key that is verified on both sides when either side makes a request.
        Variable {
            key: "GOFER_EXTENSION_SYSTEM_SECRET".into(),
            value: token.clone(),
            source: VariableSource::System,
        },
        // The Gofer host is the url where extensions can contact the Gofer server. This is used by the extension to simply
        // communicate back to the original gofer host, in case it needs to execute any API calls.
        Variable {
            key: "GOFER_EXTENSION_SYSTEM_GOFER_HOST".into(),
            value: api_state.config.server.extension_address.clone(),
            source: VariableSource::System,
        },
        Variable {
            key: "GOFER_EXTENSION_SYSTEM_BIND_ADDRESS".into(),
            value: EXTENSION_BIND_ADDRESS.to_string(),
            source: VariableSource::System,
        },
    ];

    // Now that we've defined the system vars that are included on every extension launch we need to
    // insert the env vars that are from the extension registration.
    let extension_vars: Vec<Variable> = system_extension_vars
        .iter()
        .chain(registration.settings.iter())
        .cloned()
        .collect();

    debug!(id = registration.extension_id.clone(), "Starting extension");

    let ext_container_id = extension_container_id(&registration.extension_id);

    let start_container_request = scheduler::StartContainerRequest {
        id: ext_container_id.clone(),
        image: registration.image.clone(),
        variables: extension_vars
            .into_iter()
            .map(|var| (var.key, var.value))
            .collect(),
        registry_auth: registration
            .registry_auth
            .clone()
            .map(|auth| scheduler::RegistryAuth {
                user: auth.user,
                pass: auth.pass,
            }),
        always_pull: false,
        networking: Some(8082), //TODO(Replace this with port listed in the conf package)
        entrypoint: None,
        command: None,
    };

    let start_response = api_state
        .scheduler
        .start_container(start_container_request)
        .await
        .map_err(|err| anyhow!("Could not launch extension container; {:#?}", err))?;

    // Wait for scheduler to say that container is running.
    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let extension_container_state = api_state
            .scheduler
            .get_state(scheduler::GetStateRequest {
                id: ext_container_id.clone(),
            })
            .await
            .map_err(|err| {
                anyhow!(
                    "Could not verify container '{}' due to error with scheduler; {:#?}",
                    ext_container_id.clone(),
                    err
                )
            })?;

        match extension_container_state.state {
            scheduler::ContainerState::Running => break,
            scheduler::ContainerState::Unknown
            | scheduler::ContainerState::Paused
            | scheduler::ContainerState::Restarting => continue,
            scheduler::ContainerState::Exited => {
                error!(
                    extension_container_id = ext_container_id,
                    state = extension_container_state.state.to_string(),
                    exit_code = extension_container_state.exit_code,
                    "Could not start extension container"
                );
                bail!(
                    "Could not start extension container '{}'; Scheduler reported failed state; \
                please check container logs for more info.",
                    ext_container_id
                );
            }
        }
    }

    let mut scheme = "https://";
    if !api_state.config.extensions.use_tls {
        scheme = "http://";
    }

    let extension_url = format!(
        "{}{}",
        scheme,
        &start_response.url.clone().unwrap_or_default()
    );

    let extension_client = new_extension_client(
        &extension_url,
        &token,
        api_state.config.extensions.verify_certs,
    )
    .context("Could not create extension client while attempting to start extension")?;

    // We wait in a polling loop to see if the extension is ready by hitting the health endpoint.
    let mut attempts = 0;
    debug!(
        id = &registration.extension_id,
        url = extension_url,
        "Waiting for extension to be in ready state"
    );
    loop {
        if attempts >= 30 {
            bail!("Timed out while waiting for extension to be ready; extension unreachable.")
        }

        match extension_client.health().await {
            Ok(_) => break,
            Err(err) => {
                debug!(
                    attempt = attempts,
                    err = %err,
                    "Waiting for extension to be in ready state"
                );
                attempts += 1;

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        };
    }

    let info_response = extension_client
        .info()
        .await
        .context("Could not connect to extension")?
        .into_inner();

    let new_extension = Extension {
        registration: registration.clone(),
        url: extension_url.clone(),
        started: epoch_milli(),
        state: State::Running,
        documentation: info_response.documentation.into(),
        secret: token,
    };

    api_state
        .extensions
        .insert(registration.extension_id.clone(), new_extension.clone());

    info!(
        id = registration.extension_id.clone(),
        url = extension_url,
        "Started extension"
    );

    Ok(new_extension)
}

/// Attempts to start each extension from config on the provided scheduler. Once scheduled it then collects
/// the initial extension information so it can check for connectivity and store the network location.
/// This information will eventually be used in other parts of the API to communicate with said extensions.
pub async fn start_extensions(api_state: Arc<ApiState>) -> Result<()> {
    let mut conn = match api_state.storage.read_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            error!(message = "Could not open connection to database", error = %e);
            bail!("Could not open connection to database")
        }
    };

    let registrations = storage::extension_registrations::list(&mut conn)
        .await
        .context("Could not load extensions while attempting to start all extensions")?;

    for registration_raw in registrations {
        let registration: Registration = registration_raw
            .try_into()
            .context("Could not parse extension")?;

        start_extension(api_state.clone(), registration)
            .await
            .context("Could not start extension")?;
    }

    Ok(())
}

pub async fn stop_extensions(api_state: Arc<ApiState>) {
    for extension in api_state.extensions.iter() {
        let (id, extension) = extension.pair();
        if let Ok(extension_client) = new_extension_client(
            &extension.url,
            &extension.secret,
            api_state.config.extensions.verify_certs,
        ) {
            if let Err(e) = extension_client.shutdown().await {
                error!(error = %e, extension_id = id, "Could not call shutdown on extension");
                continue;
            }

            let container_id = extension_container_id(id);

            if let Err(e) = api_state
                .scheduler
                .stop_container(scheduler::StopContainerRequest {
                    id: container_id.clone(),
                    timeout: api_state.config.extensions.stop_timeout as i64,
                })
                .await
            {
                error!(error = %e, container_id = container_id, "Could not shutdown extension via scheduler");
                continue;
            }
        } else {
            error!("Could not create extension client while attempting to stop extensions");
            continue;
        };
    }
}

/// Gofer provides default extensions that the user can opt into via their configuration.
/// This function doesn't start those extensions it just makes sure they are registered
/// so the more broad [`start_extensions`] function can start them.
pub async fn install_std_extensions(api_state: Arc<ApiState>) -> Result<()> {
    let mut conn = match api_state.storage.write_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            error!(message = "Could not open connection to database", error = %e);
            bail!("Could not open connection to database")
        }
    };

    let extensions = storage::extension_registrations::list(&mut conn)
        .await
        .context("Could not list extensions while trying to register std extensions")?;

    // Return connection to the pool.
    drop(conn);

    let mut cron_installed = false;
    let mut interval_installed = false;

    for extension in extensions {
        if extension.extension_id == "cron" {
            cron_installed = true;
        }

        if extension.extension_id == "interval" {
            interval_installed = true;
        }
    }

    if !cron_installed {
        let install_req = InstallExtensionRequest {
            id: "cron".into(),
            image: "ghcr.io/clintjedwards/gofer/extensions/cron:latest".into(),
            settings: HashMap::new(),
            registry_auth: None,
            additional_roles: None,
        };

        let registration: Registration = install_req
            .clone()
            .try_into()
            .context("Could not serialize registration for extension 'cron'")?;

        if let Err(e) = install_new_extension(api_state.clone(), &registration).await {
            let err_str = e.to_string();
            if !err_str.contains("already exists") {
                return Err(e);
            }
        };

        info!(
            name = "cron",
            image = install_req.image,
            "Registered standard extension automatically due to 'install_std_extensions' config"
        )
    }

    if !interval_installed {
        let install_req = InstallExtensionRequest {
            id: "interval".into(),
            image: "ghcr.io/clintjedwards/gofer/extensions/interval:latest".into(),
            settings: HashMap::new(),
            registry_auth: None,
            additional_roles: None,
        };

        let registration: Registration = install_req
            .clone()
            .try_into()
            .context("Could not serialize registration for extension 'interval'")?;

        if let Err(e) = install_new_extension(api_state.clone(), &registration).await {
            let err_str = e.to_string();
            if !err_str.contains("already exists") {
                return Err(e);
            }
        };

        info!(
            name = "interval",
            image = install_req.image,
            "Registered standard extension automatically due to 'install_std_extensions' config"
        )
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListExtensionsResponse {
    /// A list of all extensions.
    pub extensions: Vec<Extension>,
}

/// List all extensions currently registered.
#[endpoint(
    method = GET,
    path = "/api/extensions",
    tags = ["Extensions"],
)]
pub async fn list_extensions(
    rqctx: RequestContext<Arc<ApiState>>,
) -> Result<HttpResponseOk<ListExtensionsResponse>, HttpError> {
    let api_state = rqctx.context();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                resources: vec![Resource::Extensions("".into())],
                action: Action::Read,
            },
        )
        .await?;

    let mut extensions: Vec<Extension> = vec![];

    for extension_ref in &api_state.extensions {
        let extension = extension_ref.value();
        extensions.push(extension.clone());
    }

    let resp = ListExtensionsResponse { extensions };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetExtensionResponse {
    /// The extension requested.
    pub extension: Extension,
}

/// Returns details about a specific extension.
#[endpoint(
    method = GET,
    path = "/api/extensions/{extension_id}",
    tags = ["Extensions"],
)]
pub async fn get_extension(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<ExtensionPathArgs>,
) -> Result<HttpResponseOk<GetExtensionResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                resources: vec![Resource::Extensions(path.extension_id.clone())],
                action: Action::Read,
            },
        )
        .await?;

    let extension =
        api_state
            .extensions
            .get(&path.extension_id)
            .ok_or(HttpError::for_not_found(
                None,
                "Extension does not exist".into(),
            ))?;

    let extension = extension.value();

    let resp = GetExtensionResponse {
        extension: extension.clone(),
    };

    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct InstallExtensionRequest {
    /// A unique id for the extension. Since this needs to only be unique across extensions simply using the
    /// extension's name usually suffices.
    pub id: String,

    /// The container image this extension should use.
    pub image: String,

    /// Each extension has a list of settings it takes to configure how it runs. You can usually find this in the
    /// documentation.
    pub settings: HashMap<String, String>,

    /// Registry auth credentials
    pub registry_auth: Option<RegistryAuth>,

    /// Additional roles to add to the extension. This allows operators to extend extension access to things that
    /// otherwise the extension might not be able to do with it's default role.
    pub additional_roles: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct InstallExtensionResponse {
    pub extension: Extension,
}

impl TryFrom<InstallExtensionRequest> for Registration {
    type Error = anyhow::Error;

    fn try_from(value: InstallExtensionRequest) -> Result<Self> {
        let mut settings: Vec<Variable> = vec![];

        for (key, value) in value.settings {
            settings.push(Variable {
                key,
                value,
                source: VariableSource::System,
            })
        }

        Ok(Registration {
            extension_id: value.id,
            image: value.image,
            registry_auth: value.registry_auth,
            settings,
            created: epoch_milli(),
            modified: 0,
            status: Status::Unknown,
            additional_roles: value.additional_roles.unwrap_or_default(),
            key_id: String::new(),
        })
    }
}

/// Register and start a new extension.
///
/// This route is only available to admin tokens.
#[endpoint(
    method = POST,
    path = "/api/extensions",
    tags = ["Extensions"],
)]
pub async fn install_extension(
    rqctx: RequestContext<Arc<ApiState>>,
    body: TypedBody<InstallExtensionRequest>,
) -> Result<HttpResponseCreated<InstallExtensionResponse>, HttpError> {
    let api_state = rqctx.context();
    let body = body.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: true,
                allow_anonymous: false,
                resources: vec![Resource::Extensions("".into())],
                action: Action::Write,
            },
        )
        .await?;

    let registration: Registration = body.try_into().map_err(|err| {
        error!(message = "Could not parse request into registration", error = %err);
        HttpError::for_bad_request(
            None,
            format!("Could not parse request into registration; {:#?}", err),
        )
    })?;

    if let Err(e) = install_new_extension(api_state.clone(), &registration).await {
        let err_str = e.to_string();
        if !err_str.contains("already exists") {
            return Err(http_error!(
                "Could not install extension",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into()),
                id = registration.extension_id
            ));
        }
    };

    let new_extension = start_extension(api_state.clone(), registration.clone())
        .await
        .map_err(|err| {
            http_error!(
                "Could not start extension",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(err.into()),
                id = registration.extension_id
            )
        })?;

    let resp = InstallExtensionResponse {
        extension: new_extension,
    };

    Ok(HttpResponseCreated(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateExtensionRequest {
    pub enable: bool,
}

/// Enable or disable an extension.
///
/// This route is only accessible for admin tokens.
#[endpoint(
    method = PATCH,
    path = "/api/extensions/{extension_id}",
    tags = ["Extensions"],
)]
pub async fn update_extension(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<ExtensionPathArgs>,
    body: TypedBody<UpdateExtensionRequest>,
) -> Result<HttpResponseUpdatedNoContent, HttpError> {
    let api_state = rqctx.context();
    let body = body.into_inner();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: true,
                allow_anonymous: false,
                resources: vec![Resource::Extensions(path.extension_id.clone())],
                action: Action::Write,
            },
        )
        .await?;

    let mut conn = match api_state.storage.write_conn().await {
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

    let status = match body.enable {
        true => Status::Enabled,
        false => Status::Disabled,
    };

    let updatable_fields = storage::extension_registrations::UpdatableFields {
        image: None,
        registry_auth: None,
        settings: None,
        key_id: None,
        status: Some(status.to_string()),
        additional_roles: None,
        modified: epoch_milli().to_string(),
    };

    if let Err(e) =
        storage::extension_registrations::update(&mut conn, &path.extension_id, updatable_fields)
            .await
    {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "Extension entry for id given does not exist".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not update object in database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    Ok(HttpResponseUpdatedNoContent())
}

/// Uninstall a registered extension.
///
/// This route is only accessible for admin tokens.
#[endpoint(
    method = DELETE,
    path = "/api/extensions/{extension_id}",
    tags = ["Extensions"],
)]
pub async fn uninstall_extension(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<ExtensionPathArgs>,
) -> Result<HttpResponseDeleted, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: true,
                allow_anonymous: false,
                resources: vec![Resource::Extensions(path.extension_id.clone())],
                action: Action::Delete,
            },
        )
        .await?;

    if !api_state.extensions.contains_key(&path.extension_id) {
        return Err(HttpError::for_not_found(
            None,
            format!("Extension id '{}' does not exist", &path.extension_id),
        ));
    };

    let container_id = extension_container_id(&path.extension_id);

    // We don't care about the error here. We'll just try to shut it down on best effort.
    let _ = api_state
        .scheduler
        .stop_container(scheduler::StopContainerRequest {
            id: container_id,
            timeout: 120, // 2 mins
        })
        .await;

    let _ = api_state.extensions.remove(&path.extension_id);

    let mut conn = match api_state.storage.write_conn().await {
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

    storage::extension_registrations::delete(&mut conn, &path.extension_id)
        .await
        .map_err(|err| {
            http_error!(
                "Could not delete object from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(err.into())
            )
        })?;

    storage::roles::delete(&mut conn, &extension_role_id(&path.extension_id))
        .await
        .map_err(|err| {
            http_error!(
                "Could not delete object from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(err.into())
            )
        })?;

    Ok(HttpResponseDeleted())
}

/// Retrieves logs from the extension container.
#[channel(
    protocol = WEBSOCKETS,
    path = "/api/extensions/{extension_id}/logs",
    tags = ["Extensions"],
)]
pub async fn get_extension_logs(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<ExtensionPathArgs>,
    conn: WebsocketConnection,
) -> WebsocketChannelResult {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                resources: vec![Resource::Extensions(path.extension_id.clone())],
                action: Action::Read,
            },
        )
        .await?;

    let start_time = std::time::Instant::now();

    let ws =
        tokio_tungstenite::WebSocketStream::from_raw_socket(conn.into_inner(), Role::Server, None)
            .await;

    if !api_state.extensions.contains_key(&path.extension_id) {
        return Err(websocket_error(
            "Extension ID given does not exist",
            CloseCode::Invalid,
            rqctx.request_id.clone(),
            ws,
            None,
        )
        .await
        .into());
    }

    let container_id = extension_container_id(&path.extension_id);

    let mut logs_stream = api_state
        .scheduler
        .get_logs(GetLogsRequest { id: container_id });

    // We need to launch two async functions to:
    // * Push logs to the user.
    // * Listen for the user closing the connection.
    // * Listen for shutdown signal from main process.
    //
    // The JoinSet below allows us to launch all of the functions and then
    // wait for one of them to return. Since all need to be running
    // or they are all basically useless, we wait for any one of them to finish
    // and then we simply abort the others and then close the stream.

    let mut set: tokio::task::JoinSet<std::result::Result<(), String>> =
        tokio::task::JoinSet::new();

    let (client_write, mut client_read) = ws.split();
    let client_writer = Arc::new(tokio::sync::Mutex::new(client_write));
    let client_writer_handle = client_writer.clone();

    // Listen for a terminal signal from the main process.
    set.spawn(async move {
        listen_for_terminate_signal().await;
        Err("Server is shutting down".into())
    });

    set.spawn(async move {
        while let Some(result) = logs_stream.next().await {
            let log = match result {
                Ok(log) => log,
                Err(e) => {
                    let mut locked_writer = client_writer_handle.lock().await;

                    if let Err(err) = locked_writer
                        .send(Message::text("Could not read log from scheduler"))
                        .await
                    {
                        error!(error = %err,"Could not process log line");
                        return Err("Could not process log line".into());
                    }


                    return Err(format!("Could not process log line: {:#?}", e));
                }
            };

            match log {
                scheduler::Log::Unknown => {
                    let mut locked_writer = client_writer_handle.lock().await;

                    if let Err(err) = locked_writer
                        .send(Message::text("Received Unknown log object during log reading for container"))
                        .await
                    {
                        error!(error = %err,"Received Unknown log object during log reading for container");
                        return Err("Received Unknown log object during log reading for container".into());
                    }

                    break;
                }
                scheduler::Log::Stdout(log_bytes) => {
                    let mut locked_writer = client_writer_handle.lock().await;

                    if let Err(err) = locked_writer.send(Message::text(String::from_utf8_lossy(&log_bytes))).await {
                        error!(error = %err,"Could not process log line");
                        return Err("Could not process log line".into());
                    }
                }
                scheduler::Log::Stderr(log_bytes) => {
                    let mut locked_writer = client_writer_handle.lock().await;

                    if let Err(err) = locked_writer.send(Message::text(String::from_utf8_lossy(&log_bytes))).await {
                        error!(error = %err,"Could not process log line");
                        return Err("Could not process log line".into());
                    }
                }
                _ => {
                    // There are no other types we care about for this so we just skip anything that isn't the above.
                }
            }
        }

        Ok(())
    });

    set.spawn(async move {
        loop {
            if let Some(output) = client_read.next().await {
                match output {
                    Ok(message) => match message {
                        tungstenite::protocol::Message::Close(_) => {
                            break;
                        }
                        _ => {
                            continue;
                        }
                    },
                    Err(_) => {
                        break;
                    }
                }
            }
        }

        Ok(())
    });

    // The first one to finish will return here. We can unwrap the option safely because it only returns a None if there
    // was nothing in the set.
    let result = set.join_next().await.unwrap()?;
    if let Err(err) = result {
        let mut locked_writer = client_writer.lock().await;

        let close_message = Message::Close(Some(tungstenite::protocol::CloseFrame {
            code: tungstenite::protocol::frame::coding::CloseCode::Error,
            reason: err.clone().into(),
        }));

        let _ = locked_writer.send(close_message).await;
        let _ = locked_writer.close().await;
        return Err(err.into());
    }

    set.shutdown().await; // When one finishes we no longer have use for the others, make sure they all shutdown.

    let mut locked_writer = client_writer.lock().await;

    let close_message = Message::Close(Some(tungstenite::protocol::CloseFrame {
        code: tungstenite::protocol::frame::coding::CloseCode::Normal,
        reason: "log stream finished".into(),
    }));

    let _ = locked_writer.send(close_message).await;
    let _ = locked_writer.close().await;

    debug!(
        duration = format_duration(start_time.elapsed()),
        request_id = rqctx.request_id.clone(),
        "Finished get_extension_logs",
    );

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListExtensionSubscriptionsResponse {
    /// A list of all pipeline subscriptions for the given extension.
    pub subscriptions: Vec<subscriptions::Subscription>,
}

/// List all extension subscriptions.
#[endpoint(
    method = GET,
    path = "/api/extensions/{extension_id}/subscriptions",
    tags = ["Extensions"],
)]
pub async fn list_extension_subscriptions(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<ExtensionPathArgs>,
) -> Result<HttpResponseOk<ListExtensionSubscriptionsResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                resources: vec![Resource::Extensions(path.extension_id.clone())],
                action: Action::Read,
            },
        )
        .await?;

    let mut conn = match api_state.storage.read_conn().await {
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

    let storage_subscriptions =
        match storage::extension_subscriptions::list_by_extension(&mut conn, &path.extension_id)
            .await
        {
            Ok(subscriptions) => subscriptions,
            Err(e) => {
                return Err(http_error!(
                    "Could not get objects from database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        };

    let mut subscriptions: Vec<subscriptions::Subscription> = vec![];

    for storage_subscription in storage_subscriptions {
        let subscription =
            subscriptions::Subscription::try_from(storage_subscription).map_err(|e| {
                http_error!(
                    "Could not parse object from database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                )
            })?;

        subscriptions.push(subscription);
    }

    let resp = ListExtensionSubscriptionsResponse { subscriptions };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DebugResponse {
    pub info: String,
}

/// Dump extension debug information.
#[endpoint(
    method = GET,
    path = "/api/extensions/{extension_id}/debug",
    tags = ["Extensions"],
)]
pub async fn get_extension_debug_info(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<ExtensionPathArgs>,
) -> Result<HttpResponseOk<DebugResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: true,
                allow_anonymous: false,
                resources: vec![Resource::Extensions(path.extension_id.clone())],
                action: Action::Read,
            },
        )
        .await?;

    let extension = match api_state.extensions.get(&path.extension_id) {
        Some(extension) => extension.value().clone(),
        None => {
            return Err(HttpError::for_bad_request(
                None,
                format!("extension_id '{}' not found", &path.extension_id,),
            ));
        }
    };

    let extension_client = new_extension_client(
        &extension.url,
        &extension.secret,
        api_state.config.extensions.verify_certs,
    )
    .map_err(|e| {
        http_error!(
            "Could not establish extension client",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    let debug_response = extension_client
        .debug()
        .await
        .map_err(|e| {
            http_error!(
                "Could not query extension's debug endpoint",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            )
        })?
        .into_inner();

    let resp = DebugResponse {
        info: debug_response.info,
    };
    Ok(HttpResponseOk(resp))
}

/// Creates a new HTTP client that is set up to talk to Gofer extensions.
pub fn new_extension_client(
    url: &str,
    token: &str,
    verify_certs: bool,
) -> Result<gofer_sdk::extension::api::Client> {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(&format!("Bearer {}", token))?,
    );

    let client = Client::builder()
        .danger_accept_invalid_certs(!verify_certs)
        .default_headers(headers)
        .build()?;

    Ok(gofer_sdk::extension::api::Client::new_with_client(
        url, client,
    ))
}

async fn install_new_extension(
    api_state: Arc<ApiState>,
    registration: &Registration,
) -> Result<()> {
    let mut conn = api_state.storage.write_conn().await?;

    // Check to make sure extension doesn't exist already.
    match storage::extension_registrations::get(&mut conn, &registration.extension_id).await {
        Ok(_) => {
            bail!(
                "Extension with id '{}' already exists.",
                registration.extension_id.clone()
            );
        }
        Err(e) => match e {
            storage::StorageError::NotFound => {}
            _ => {
                bail!("Could not get objects from database; {:#?}", e);
            }
        },
    };

    // We need to create a new role for the extension so that it has appropriate permissions to perform actions
    // to aid the user.
    let new_role = InternalRole {
        id: extension_role_id(&registration.extension_id),
        description:
            "Auto-created role for registered extension; Allows extension to access needful \
            resources"
                .to_string(),
        permissions: vec![
            // The only write access extensions need is to their own object store so they can use that as a database.
            InternalPermission {
                resources: vec![Resource::Extensions(format!(
                    "^{}$", // Match only exactly extension targets with this name.
                    registration.extension_id
                ))],
                actions: vec![Action::Read, Action::Write, Action::Delete],
            },
            // Allow extensions to start runs.
            InternalPermission {
                resources: vec![
                    Resource::Namespaces(".*".to_string()),
                    Resource::Pipelines(".*".to_string()),
                    Resource::Runs,
                ],
                actions: vec![Action::Read, Action::Write],
            },
            // Provide read to most resources so that extensions can be somewhat useful. The decision here on where
            // to provide access is quite difficult, but we went with a more open model assuming that the extensions
            // are from somewhat trusted parties and not allowing TOO much access to things that can really leak
            // intellectual propety.
            InternalPermission {
                resources: vec![
                    Resource::Configs,
                    Resource::Deployments,
                    Resource::Events,
                    Resource::Namespaces(".*".to_string()),
                    Resource::Pipelines(".*".to_string()),
                    Resource::Subscriptions,
                    Resource::System,
                    Resource::TaskExecutions,
                ],
                actions: vec![Action::Read],
            },
        ],
        system_role: true,
    };

    let new_role_storage = match new_role.clone().try_into() {
        Ok(role) => role,
        Err(e) => {
            bail!("Could not create new role for new extension; {:#?}", e);
        }
    };

    if let Err(e) = storage::roles::insert(&mut conn, &new_role_storage).await {
        match e {
            storage::StorageError::Exists => {
                bail!("Could not create new role for new extension; role already exists");
            }
            _ => {
                bail!("Could not insert objects into database; {:#?}", e);
            }
        }
    };

    api_state
        .event_bus
        .clone()
        .publish(event_utils::Kind::CreatedRole {
            role_id: new_role.id.clone(),
        });

    let new_extension_storage = registration
        .clone()
        .try_into()
        .map_err(|err: anyhow::Error| {
            anyhow::anyhow!("Could not serialize objects for database; {:#?}", err)
        })?;

    storage::extension_registrations::insert(&mut conn, &new_extension_storage).await?;

    Ok(())
}
