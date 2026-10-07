use super::{
    ApiState, PreflightOptions, RequestInfo, RequestMetadata, epoch_milli, event_utils,
    is_valid_identifier, storage, tokens,
};
use crate::http_error;
use anyhow::{Context, Result, bail};
use dropshot::{
    ClientErrorStatusCode, HttpError, HttpResponseCreated, HttpResponseDeleted, HttpResponseOk,
    Path, RequestContext, TypedBody, endpoint,
};
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use strum::{Display, EnumString};
use tracing::error;

#[derive(Debug, Clone, Display, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum Action {
    Read,
    Write,
    Delete,
}

/// Things that live under a namespace and pipeline.
#[derive(Debug, Clone, Display, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum NamespaceResource {
    Pipelines,
    Configs,
    Deployments,
    Runs,
    TaskExecutions,
    Objects,
    Secrets,
    Subscriptions,
}

/// Things that belong to a specific extension.
#[derive(Debug, Clone, Display, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum ExtensionResource {
    Objects,
    Subscriptions,
    Logs,
}

/// Things that aren't scoped to a namespace or extension.
#[derive(Debug, Clone, Display, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum GlobalResource {
    Events,
    Tokens,
    Roles,
    Secrets,
    System,
}

/// Grants access to resources within matching namespaces and pipelines.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct NamespaceGrant {
    /// Regex matched against the entire namespace id. Use '.*' to match every namespace.
    pub namespace: String,

    /// Regex matched against the entire pipeline id. Leaving it out matches every pipeline.
    #[serde(default)]
    pub pipeline: Option<String>,

    pub resources: Vec<NamespaceResource>,
    pub actions: Vec<Action>,
}

/// Grants access to resources belonging to matching extensions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct ExtensionGrant {
    /// Regex matched against the entire extension id. Use '.*' to match every extension.
    pub extension: String,

    pub resources: Vec<ExtensionResource>,
    pub actions: Vec<Action>,
}

/// Grants access to resources that aren't scoped to a namespace or extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct GlobalGrant {
    pub resources: Vec<GlobalResource>,
    pub actions: Vec<Action>,
}

/// Everything a role allows. Each grant stands on its own; a request is allowed only if a single grant matches its
/// target and includes both the resource and the action.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Grants {
    #[serde(default)]
    pub namespaces: Vec<NamespaceGrant>,

    #[serde(default)]
    pub extensions: Vec<ExtensionGrant>,

    #[serde(default)]
    pub global: Vec<GlobalGrant>,
}

/// What a route needs from the token in order to be accessed.
///
/// Targets set to `None` are for listing routes, which don't act on a single object. Those routes must filter what
/// they return with [`RequestMetadata::allows`].
#[derive(Debug, Clone)]
pub enum Requirement {
    /// Any valid token.
    Authenticated,

    /// Reading a namespace is implied by any grant that matches it. Writing or deleting namespaces is admin only.
    Namespace {
        namespace: Option<String>,
    },

    Pipeline {
        namespace: String,
        pipeline: Option<String>,
        resource: NamespaceResource,
    },

    /// A resource of `None` is reading the extension itself, which is implied by any grant that matches it.
    Extension {
        extension: Option<String>,
        resource: Option<ExtensionResource>,
    },

    Global(GlobalResource),
}

impl Requirement {
    pub fn pipeline(namespace: &str, pipeline: &str, resource: NamespaceResource) -> Self {
        Requirement::Pipeline {
            namespace: namespace.into(),
            pipeline: Some(pipeline.into()),
            resource,
        }
    }

    pub fn extension(extension: &str, resource: ExtensionResource) -> Self {
        Requirement::Extension {
            extension: Some(extension.into()),
            resource: Some(resource),
        }
    }
}

impl std::fmt::Display for Requirement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let target = |value: &Option<String>| value.clone().unwrap_or_else(|| "*".into());

        match self {
            Requirement::Authenticated => write!(f, "any valid token"),
            Requirement::Namespace { namespace } => {
                write!(f, "namespace '{}'", target(namespace))
            }
            Requirement::Pipeline {
                namespace,
                pipeline,
                resource,
            } => write!(
                f,
                "'{resource}' in namespace '{namespace}' pipeline '{}'",
                target(pipeline)
            ),
            Requirement::Extension {
                extension,
                resource: Some(resource),
            } => write!(f, "'{resource}' for extension '{}'", target(extension)),
            Requirement::Extension {
                extension,
                resource: None,
            } => write!(f, "extension '{}'", target(extension)),
            Requirement::Global(resource) => write!(f, "global '{resource}'"),
        }
    }
}

/// Targets always match the whole identifier. Without this, a target of 'default' would also match 'not-default'.
fn anchored(target: &str) -> String {
    format!("^(?:{target})$")
}

/// A `None` id comes from listing routes and matches any target.
fn target_matches(target: &str, id: Option<&str>) -> bool {
    match id {
        None => true,
        Some(id) => Regex::new(&anchored(target)).is_ok_and(|regex| regex.is_match(id)),
    }
}

fn validate_target(kind: &str, target: &str) -> Result<()> {
    if target.is_empty() {
        bail!("{kind} target cannot be empty; use '.*' to match everything");
    }

    Regex::new(&anchored(target))
        .with_context(|| format!("{kind} target '{target}' is not a valid regex"))?;

    Ok(())
}

impl Grants {
    pub fn allows(&self, requirement: &Requirement, action: &Action) -> bool {
        match requirement {
            Requirement::Authenticated => true,
            Requirement::Namespace { namespace } => {
                *action == Action::Read
                    && self
                        .namespaces
                        .iter()
                        .any(|grant| target_matches(&grant.namespace, namespace.as_deref()))
            }
            Requirement::Pipeline {
                namespace,
                pipeline,
                resource,
            } => self.namespaces.iter().any(|grant| {
                grant.actions.contains(action)
                    && grant.resources.contains(resource)
                    && target_matches(&grant.namespace, Some(namespace))
                    && grant
                        .pipeline
                        .as_ref()
                        .is_none_or(|target| target_matches(target, pipeline.as_deref()))
            }),
            Requirement::Extension {
                extension,
                resource: None,
            } => {
                *action == Action::Read
                    && self
                        .extensions
                        .iter()
                        .any(|grant| target_matches(&grant.extension, extension.as_deref()))
            }
            Requirement::Extension {
                extension,
                resource: Some(resource),
            } => self.extensions.iter().any(|grant| {
                grant.actions.contains(action)
                    && grant.resources.contains(resource)
                    && target_matches(&grant.extension, extension.as_deref())
            }),
            Requirement::Global(resource) => self
                .global
                .iter()
                .any(|grant| grant.actions.contains(action) && grant.resources.contains(resource)),
        }
    }

    /// Rejects grants with invalid targets or that could never match anything, so a role never behaves differently
    /// from how it reads.
    pub fn validate(&self) -> Result<()> {
        for grant in &self.namespaces {
            validate_target("namespace", &grant.namespace)?;
            if let Some(pipeline) = &grant.pipeline {
                validate_target("pipeline", pipeline)?;
            }
            if grant.resources.is_empty() || grant.actions.is_empty() {
                bail!(
                    "namespace grant for '{}' needs at least one resource and action",
                    grant.namespace
                );
            }
        }

        for grant in &self.extensions {
            validate_target("extension", &grant.extension)?;
            if grant.resources.is_empty() || grant.actions.is_empty() {
                bail!(
                    "extension grant for '{}' needs at least one resource and action",
                    grant.extension
                );
            }
        }

        for grant in &self.global {
            if grant.resources.is_empty() || grant.actions.is_empty() {
                bail!("global grant needs at least one resource and action");
            }
        }

        Ok(())
    }

    pub fn extend(&mut self, other: Grants) {
        self.namespaces.extend(other.namespaces);
        self.extensions.extend(other.extensions);
        self.global.extend(other.global);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Role {
    /// Alphanumeric with dashes only
    pub id: String,
    pub description: String,
    pub grants: Grants,

    /// If this role was created by Gofer itself. System roles cannot be modified.
    pub system_role: bool,
}

impl TryFrom<storage::roles::Role> for Role {
    type Error = anyhow::Error;

    fn try_from(value: storage::roles::Role) -> Result<Self> {
        let grants: Grants = serde_json::from_str(&value.grants).with_context(|| {
            format!(
                "Could not parse field 'grants' from storage value '{}'",
                value.grants
            )
        })?;

        Ok(Role {
            id: value.id,
            description: value.description,
            grants,
            system_role: value.system_role,
        })
    }
}

impl TryFrom<Role> for storage::roles::Role {
    type Error = anyhow::Error;

    fn try_from(value: Role) -> Result<Self> {
        let grants = serde_json::to_string(&value.grants).with_context(|| {
            format!("Could not serialize field 'grants'; '{:#?}'", value.grants)
        })?;

        Ok(Self {
            id: value.id,
            description: value.description,
            grants,
            system_role: value.system_role,
        })
    }
}

/// Special role ids that Gofer provides automatically with preset permissions. These roles cannot be edited or removed.
#[derive(Debug, Clone, Display, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema)]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum SystemRoles {
    /// Identical to the admin token. The first token ever that gets created will recieve this role.
    Bootstrap,

    /// Admin token; has access to just about everything.
    Admin,

    /// A regular user of the system.
    User,

    /// A role given to users who have not signed in. Only has access to pipelines and run information for the default namespace.
    Anonymous,
}

/// Contains information about the auth token sent with a request.
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// The unique identifier for the api token the current user is using.
    pub token_id: String,

    /// The plaintext username attached to the token.
    pub token_user: String,

    /// The role ids for the current token.
    pub roles: Vec<String>,
}

/// Pulls the raw token out of a request's `Authorization: Bearer <token>` header.
pub fn bearer_token(request: &RequestInfo) -> Result<&str, HttpError> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .ok_or(HttpError::for_client_error(
            None,
            ClientErrorStatusCode::UNAUTHORIZED,
            "Authorization header not found but required".into(),
        ))?;

    let auth_header = auth_header.to_str().map_err(|e| {
        HttpError::for_bad_request(
            None,
            format!("Could not parse Authorization header; {:#?}", e),
        )
    })?;

    auth_header.strip_prefix("Bearer ").ok_or_else(|| {
        HttpError::for_bad_request(
            None,
            "Authorization header malformed; should start with 'Bearer'".into(),
        )
    })
}

impl ApiState {
    /// Resolves request specific context for handlers. This is used to perform auth checks and generally other
    /// actions that should happen before a route runs it's handler.
    ///
    /// **Should be called at the start of every handler**, regardless of if that handler needs auth or req_context.
    ///
    /// We specifically use a struct here so that the reader can easily verify which options are in which state for the
    /// route that it is included. The different options here map to different actions that are checked per call.
    ///
    /// Routes that leave a target as `None` in their [`Requirement`] must filter what they return with
    /// [`RequestMetadata::allows`].
    pub async fn preflight_check(
        &self,
        request: &RequestInfo,
        options: PreflightOptions,
    ) -> Result<RequestMetadata, HttpError> {
        let mut bypass_auth = options.bypass_auth;

        if self.config.development.bypass_auth {
            bypass_auth = self.config.development.bypass_auth
        }

        // This is somewhat dangerous since we just assume the user is global admin, but since you cannot auth to
        // a different endpoint from this point on I think it's okay.
        let auth_ctx = if bypass_auth {
            AuthContext {
                token_id: "0".into(),
                token_user: "Anonymous".into(),
                // Allow access to all routes.
                roles: vec![SystemRoles::Admin.to_string()],
            }
            // Allow anonymous allows read-only access to the default namespace for a subset of routes.
            // It's only used to easily display the frontend for non-logged in users.
        } else if options.allow_anonymous {
            match self.get_auth_context(request).await {
                // If the user is actually logged in we still want to enable a login. If they are actually not logged
                // in then we want to use the anon user.
                Ok(auth) => auth,
                Err(_) => AuthContext {
                    token_id: "0".into(),
                    token_user: "Anonymous".into(),
                    roles: vec![SystemRoles::Anonymous.to_string()],
                },
            }
        } else {
            self.get_auth_context(request).await?
        };
        let api_version = super::check_version_handler(request)?;

        let admin = auth_ctx.roles.contains(&SystemRoles::Admin.to_string())
            || auth_ctx.roles.contains(&SystemRoles::Bootstrap.to_string());

        if options.admin_only && !admin {
            return Err(HttpError::for_client_error(
                None,
                ClientErrorStatusCode::FORBIDDEN,
                "Route requires admin level token".into(),
            ));
        }

        let grants = if admin {
            Grants::default()
        } else {
            self.get_grants(&auth_ctx.roles).await?
        };

        let metadata = RequestMetadata {
            auth: auth_ctx,
            api_version,
            admin,
            grants,
        };

        if !metadata.allows(&options.requires, &options.action) {
            return Err(HttpError::for_client_error(
                None,
                ClientErrorStatusCode::FORBIDDEN,
                format!(
                    "Token does not have permission to access this route. \
                    Route requires a single grant allowing '{}' on {}",
                    options.action, options.requires
                ),
            ));
        }

        Ok(metadata)
    }

    /// Collects every grant from the given roles. Since each grant is evaluated on its own there is no need to keep
    /// track of which role it came from.
    async fn get_grants(&self, role_ids: &[String]) -> Result<Grants, HttpError> {
        let mut conn = match self.storage.read_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                return Err(crate::http_error!(
                    "Could not open connection to database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    "None".into(),
                    Some(e.into())
                ));
            }
        };

        let mut grants = Grants::default();

        for role_id in role_ids {
            let storage_role = match storage::roles::get(&mut conn, role_id).await {
                Ok(role) => role,
                // A token can reference a role that has since been deleted; it simply grants nothing.
                Err(storage::StorageError::NotFound) => continue,
                Err(e) => {
                    return Err(http_error!(
                        "Could not query database for roles during authentication permission checking",
                        hyper::StatusCode::INTERNAL_SERVER_ERROR,
                        "0".into(),
                        Some(e.into())
                    ));
                }
            };

            let role = Role::try_from(storage_role).map_err(|err| {
                error!(message = "Could not serialize role from storage", error = %err);
                http_error!(
                    "Could not parse role object from database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    "0".into(),
                    Some(err.into())
                )
            })?;

            grants.extend(role.grants);
        }

        Ok(grants)
    }

    /// Checks request authentication and returns valid auth information.
    async fn get_auth_context(&self, request: &RequestInfo) -> Result<AuthContext, HttpError> {
        let token = bearer_token(request)?;

        let hash = super::tokens::hash_token(token);

        let mut conn = match self.storage.read_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                return Err(crate::http_error!(
                    "Could not open connection to database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    "None".into(),
                    Some(e.into())
                ));
            }
        };

        let storage_token = match storage::tokens::get_by_hash(&mut conn, &hash).await {
            Ok(token) => token,
            Err(e) => match e {
                storage::StorageError::NotFound => {
                    return Err(HttpError::for_client_error(
                        None,
                        ClientErrorStatusCode::UNAUTHORIZED,
                        "Unauthorized".into(),
                    ));
                }
                _ => {
                    return Err(crate::http_error!(
                        "Could not query database",
                        hyper::StatusCode::INTERNAL_SERVER_ERROR,
                        "None".into(),
                        Some(e.into())
                    ));
                }
            },
        };

        let token = tokens::Token::try_from(storage_token).map_err(|e| {
            crate::http_error!(
                "Could not parse token object from database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                "None".into(),
                Some(e.into())
            )
        })?;

        if token.disabled {
            return Err(HttpError::for_client_error(
                None,
                ClientErrorStatusCode::UNAUTHORIZED,
                "Token disabled".into(),
            ));
        }

        // If the token expires is 0 it's valid forever.
        if token.expires != 0 && epoch_milli() > token.expires {
            return Err(HttpError::for_client_error(
                None,
                ClientErrorStatusCode::UNAUTHORIZED,
                "Token expired".into(),
            ));
        }

        Ok(AuthContext {
            token_id: token.id,
            token_user: token.user,
            roles: token.roles,
        })
    }
}

/// Creates the default roles for Gofer. It is safe to call this even if the role has been already created.
///
/// We create most of the roles mentioned in the [`SystemRoles`] enum.
pub async fn create_system_roles(api_state: std::sync::Arc<ApiState>) -> Result<()> {
    let mut conn = match api_state.storage.write_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            error!(message = "Could not open connection to database", error = %e);
            bail!("Could not open connection to database")
        }
    };

    for role in system_roles() {
        let storage_role: storage::roles::Role = role.try_into().context(
            "Could not serialize role into storage role \
            while attempting to insert system roles.",
        )?;

        if let Err(e) = storage::roles::insert(&mut conn, &storage_role).await {
            match e {
                storage::StorageError::Exists => {
                    return Ok(());
                }
                _ => {
                    bail!("{e}")
                }
            }
        }
    }

    Ok(())
}

/// The roles Gofer ships with. Admin and bootstrap have no grants because they skip permission checks entirely.
fn system_roles() -> Vec<Role> {
    let all_actions = vec![Action::Read, Action::Write, Action::Delete];

    let bootstrap_role = Role {
        id: SystemRoles::Bootstrap.to_string(),
        description: "The original role that all other tokens/roles are created from. Has access to everything."
            .into(),
        grants: Grants::default(),
        system_role: true,
    };

    let admin_role = Role {
        id: SystemRoles::Admin.to_string(),
        description: "Essentially root access. This role has unmitigated access to every resource."
            .into(),
        grants: Grants::default(),
        system_role: true,
    };

    let user_role = Role {
        id: SystemRoles::User.to_string(),
        description:
            "A common user role that has read/write/delete access to the default namespace.".into(),
        grants: Grants {
            namespaces: vec![NamespaceGrant {
                namespace: "default".into(),
                pipeline: None,
                resources: vec![
                    NamespaceResource::Pipelines,
                    NamespaceResource::Configs,
                    NamespaceResource::Deployments,
                    NamespaceResource::Runs,
                    NamespaceResource::TaskExecutions,
                    NamespaceResource::Objects,
                    NamespaceResource::Secrets,
                    NamespaceResource::Subscriptions,
                ],
                actions: all_actions,
            }],
            extensions: vec![],
            global: vec![GlobalGrant {
                resources: vec![GlobalResource::Events],
                actions: vec![Action::Read],
            }],
        },
        system_role: true,
    };

    let anon_role = Role {
        id: SystemRoles::Anonymous.to_string(),
        description: "The role given when a user has not signed in at all.".into(),
        grants: Grants {
            namespaces: vec![NamespaceGrant {
                namespace: "default".into(),
                pipeline: None,
                resources: vec![NamespaceResource::Pipelines, NamespaceResource::Runs],
                actions: vec![Action::Read],
            }],
            extensions: vec![],
            global: vec![],
        },
        system_role: true,
    };

    vec![bootstrap_role, admin_role, user_role, anon_role]
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListRolesResponse {
    /// A list of all roles.
    pub roles: Vec<Role>,
}

/// List all roles.
#[endpoint(
    method = GET,
    path = "/api/roles",
    tags = ["Permissions"],
)]
pub async fn list_roles(
    rqctx: RequestContext<Arc<ApiState>>,
) -> Result<HttpResponseOk<ListRolesResponse>, HttpError> {
    let api_state = rqctx.context();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                requires: Requirement::Global(GlobalResource::Roles),
                action: Action::Read,
                allow_anonymous: false,
            },
        )
        .await?;

    let mut conn = match api_state.storage.read_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let storage_roles = match storage::roles::list(&mut conn).await {
        Ok(roles) => roles,
        Err(e) => {
            return Err(http_error!(
                "Could not get objects from database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let mut roles: Vec<Role> = vec![];

    for storage_role in storage_roles {
        let role = Role::try_from(storage_role).map_err(|e| {
            http_error!(
                "Could not parse object from database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        roles.push(role);
    }

    let resp = ListRolesResponse { roles };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetRoleResponse {
    /// The target role.
    pub role: Role,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RolePathArgs {
    /// The unique identifier for the target role.
    pub role_id: String,
}

/// Get api role by id.
#[endpoint(
    method = GET,
    path = "/api/roles/{role_id}",
    tags = ["Permissions"],
)]
pub async fn get_role(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<RolePathArgs>,
) -> Result<HttpResponseOk<GetRoleResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                requires: Requirement::Global(GlobalResource::Roles),
                action: Action::Read,
                allow_anonymous: false,
            },
        )
        .await?;

    let mut conn = match api_state.storage.read_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let storage_role = match storage::roles::get(&mut conn, &path.role_id).await {
        Ok(role) => role,
        Err(e) => match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(None, String::new()));
            }
            _ => {
                return Err(http_error!(
                    "Could not get object from database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        },
    };

    let role = Role::try_from(storage_role).map_err(|e| {
        http_error!(
            "Could not parse object from database",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    let resp = GetRoleResponse { role };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateRoleRequest {
    /// The unique identifier for the role. Only accepts alphanumeric chars with hyphens. No spaces.
    pub id: String,

    /// Short description about what the role is used for.
    pub description: String,

    /// What the role allows.
    pub grants: Grants,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateRoleResponse {
    /// Information about the role created.
    pub role: Role,
}

/// Create a new role.
///
/// This route is only accessible for admin tokens.
#[endpoint(
    method = POST,
    path = "/api/roles",
    tags = ["Permissions"],
)]
pub async fn create_role(
    rqctx: RequestContext<Arc<ApiState>>,
    body: TypedBody<CreateRoleRequest>,
) -> Result<HttpResponseCreated<CreateRoleResponse>, HttpError> {
    let api_state = rqctx.context();
    let body = body.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: true,
                requires: Requirement::Global(GlobalResource::Roles),
                action: Action::Write,
                allow_anonymous: false,
            },
        )
        .await?;

    if let Err(e) = is_valid_identifier(&body.id) {
        return Err(HttpError::for_bad_request(
            None,
            format!("'{}' is not a valid identifier; {}", body.id, e),
        ));
    };

    let mut conn = match api_state.storage.write_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    if let Err(e) = body.grants.validate() {
        return Err(HttpError::for_bad_request(
            None,
            format!("Invalid grants; {e:#}"),
        ));
    }

    let new_role = Role {
        id: body.id,
        description: body.description,
        grants: body.grants,
        system_role: false,
    };

    let new_role_storage = match new_role.clone().try_into() {
        Ok(role) => role,
        Err(e) => {
            return Err(http_error!(
                "Could not parse token into storage type while creating role",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(anyhow::anyhow!("{}", e).into())
            ));
        }
    };

    if let Err(e) = storage::roles::insert(&mut conn, &new_role_storage).await {
        match e {
            storage::StorageError::Exists => {
                return Err(HttpError::for_client_error(
                    None,
                    ClientErrorStatusCode::CONFLICT,
                    "role entry already exists".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not insert objects into database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    api_state
        .event_bus
        .clone()
        .publish(event_utils::Kind::CreatedRole {
            role_id: new_role.id.clone(),
        });

    let resp = CreateRoleResponse { role: new_role };

    Ok(HttpResponseCreated(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateRoleRequest {
    /// Short description about what the role is used for.
    pub description: Option<String>,

    /// Replaces everything the role allows.
    pub grants: Option<Grants>,
}

impl TryFrom<UpdateRoleRequest> for storage::roles::UpdatableFields {
    type Error = anyhow::Error;

    fn try_from(value: UpdateRoleRequest) -> Result<Self> {
        let grants = match value.grants {
            Some(grants) => {
                grants.validate()?;
                Some(serde_json::to_string(&grants).context("Could not serialize grants")?)
            }
            None => None,
        };

        Ok(Self {
            description: value.description,
            grants,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateRoleResponse {
    /// Information about the role updated.
    pub role: Role,
}

/// Update a role's details.
///
/// This route is only accessible for admin tokens.
#[endpoint(
    method = PATCH,
    path = "/api/roles/{role_id}",
    tags = ["Permissions"],
)]
pub async fn update_role(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<RolePathArgs>,
    body: TypedBody<UpdateRoleRequest>,
) -> Result<HttpResponseOk<UpdateRoleResponse>, HttpError> {
    let api_state = rqctx.context();
    let body = body.into_inner();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: true,
                requires: Requirement::Global(GlobalResource::Roles),
                action: Action::Write,
                allow_anonymous: false,
            },
        )
        .await?;

    let mut tx = match api_state.storage.open_tx().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let updatable_fields = match storage::roles::UpdatableFields::try_from(body.clone()) {
        Ok(fields) => fields,
        Err(e) => {
            return Err(HttpError::for_bad_request(
                None,
                format!("Invalid grants; {e:#}"),
            ));
        }
    };

    let storage_role = match storage::roles::get(&mut tx, &path.role_id).await {
        Ok(role) => role,
        Err(e) => match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "Role for id given does not exist".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not get object in database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        },
    };

    // If its a system role then we don't want any user to edit it.
    if storage_role.system_role {
        return Err(HttpError::for_client_error(
            None,
            ClientErrorStatusCode::FORBIDDEN,
            "Cannot edit system roles.".into(),
        ));
    }

    if let Err(e) = storage::roles::update(&mut tx, &path.role_id, updatable_fields).await {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "Role entry for id given does not exist".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not update object in database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    let storage_role = match storage::roles::get(&mut tx, &path.role_id).await {
        Ok(role) => role,
        Err(e) => match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "Role for id given does not exist".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not get object in database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        },
    };

    if let Err(e) = tx.commit().await {
        error!(message = "Could not close transaction from database", error = %e);
        return Err(HttpError::for_internal_error(format!(
            "Encountered error when attempting to write role to database; {:#?}",
            e
        )));
    };

    let role = Role::try_from(storage_role).map_err(|e| {
        http_error!(
            "Could not parse object from database",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    let resp = UpdateRoleResponse { role };

    Ok(HttpResponseOk(resp))
}

/// Delete api role by id.
///
/// This route is only accessible for admin tokens.
#[endpoint(
    method = DELETE,
    path = "/api/roles/{role_id}",
    tags = ["Permissions"],
)]
pub async fn delete_role(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<RolePathArgs>,
) -> Result<HttpResponseDeleted, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: true,
                requires: Requirement::Global(GlobalResource::Roles),
                action: Action::Delete,
                allow_anonymous: false,
            },
        )
        .await?;

    let mut tx = match api_state.storage.open_tx().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let storage_role = match storage::roles::get(&mut tx, &path.role_id).await {
        Ok(role) => role,
        Err(e) => match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "Role for id given does not exist".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not get object in database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        },
    };

    // If its a system role then we don't want any user to remove it.
    if storage_role.system_role {
        return Err(HttpError::for_client_error(
            None,
            ClientErrorStatusCode::FORBIDDEN,
            "Cannot remove system roles.".into(),
        ));
    }

    if let Err(e) = storage::roles::delete(&mut tx, &path.role_id).await {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "role for id given does not exist".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not delete object from database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    if let Err(e) = tx.commit().await {
        error!(message = "Could not close transaction from database", error = %e);
        return Err(HttpError::for_internal_error(format!(
            "Encountered error when attempting to write role to database; {:#?}",
            e
        )));
    };

    api_state
        .event_bus
        .clone()
        .publish(event_utils::Kind::DeletedRole {
            role_id: path.role_id.clone(),
        });

    Ok(HttpResponseDeleted())
}

#[cfg(test)]
mod tests {
    use super::*;
    use NamespaceResource as NR;

    fn system_role(id: SystemRoles) -> Grants {
        system_roles()
            .into_iter()
            .find(|role| role.id == id.to_string())
            .unwrap()
            .grants
    }

    fn namespace_grant(
        namespace: &str,
        pipeline: Option<&str>,
        resources: &[NR],
        actions: &[Action],
    ) -> NamespaceGrant {
        NamespaceGrant {
            namespace: namespace.into(),
            pipeline: pipeline.map(Into::into),
            resources: resources.to_vec(),
            actions: actions.to_vec(),
        }
    }

    fn pipeline(namespace: &str, pipeline: &str, resource: NR) -> Requirement {
        Requirement::pipeline(namespace, pipeline, resource)
    }

    fn namespace(namespace: &str) -> Requirement {
        Requirement::Namespace {
            namespace: Some(namespace.into()),
        }
    }

    #[test]
    fn targets_match_whole_identifier() {
        let grants = Grants {
            namespaces: vec![namespace_grant(
                "default",
                None,
                &[NR::Runs],
                &[Action::Read],
            )],
            ..Default::default()
        };

        assert!(grants.allows(&pipeline("default", "x", NR::Runs), &Action::Read));
        assert!(!grants.allows(&pipeline("not-default", "x", NR::Runs), &Action::Read));
        assert!(!grants.allows(&pipeline("default-two", "x", NR::Runs), &Action::Read));
    }

    #[test]
    fn missing_pipeline_target_matches_every_pipeline() {
        let grants = Grants {
            namespaces: vec![namespace_grant(
                "devops.*",
                None,
                &[NR::Runs],
                &[Action::Read],
            )],
            ..Default::default()
        };

        assert!(grants.allows(&pipeline("devops-one", "anything", NR::Runs), &Action::Read));
        assert!(!grants.allows(&pipeline("other", "anything", NR::Runs), &Action::Read));
    }

    #[test]
    fn grants_do_not_combine() {
        let grants = Grants {
            namespaces: vec![
                namespace_grant("frontend", Some("website"), &[NR::Runs], &[Action::Read]),
                namespace_grant("backend", Some("billing"), &[NR::Secrets], &[Action::Write]),
            ],
            ..Default::default()
        };

        assert!(grants.allows(&pipeline("frontend", "website", NR::Runs), &Action::Read));
        assert!(grants.allows(&pipeline("backend", "billing", NR::Secrets), &Action::Write));
        assert!(!grants.allows(&pipeline("frontend", "billing", NR::Runs), &Action::Read));
        assert!(!grants.allows(&pipeline("frontend", "website", NR::Secrets), &Action::Read));
        assert!(!grants.allows(&pipeline("frontend", "website", NR::Runs), &Action::Write));
    }

    #[test]
    fn namespace_read_is_implied_by_any_grant() {
        let grants = Grants {
            namespaces: vec![namespace_grant(
                "devops",
                Some("one"),
                &[NR::Runs],
                &[Action::Write],
            )],
            ..Default::default()
        };

        assert!(grants.allows(&namespace("devops"), &Action::Read));
        assert!(!grants.allows(&namespace("devops"), &Action::Write));
        assert!(!grants.allows(&namespace("other"), &Action::Read));
    }

    #[test]
    fn listing_targets_match_any_grant_of_that_kind() {
        let grants = Grants {
            namespaces: vec![namespace_grant(
                "devops",
                Some("one"),
                &[NR::Pipelines],
                &[Action::Read],
            )],
            ..Default::default()
        };

        assert!(grants.allows(&Requirement::Namespace { namespace: None }, &Action::Read));
        assert!(grants.allows(
            &Requirement::Pipeline {
                namespace: "devops".into(),
                pipeline: None,
                resource: NR::Pipelines
            },
            &Action::Read
        ));
        assert!(!grants.allows(
            &Requirement::Extension {
                extension: None,
                resource: None
            },
            &Action::Read
        ));
    }

    #[test]
    fn extension_grants() {
        let grants = Grants {
            extensions: vec![ExtensionGrant {
                extension: "github".into(),
                resources: vec![ExtensionResource::Objects],
                actions: vec![Action::Read],
            }],
            ..Default::default()
        };

        assert!(grants.allows(
            &Requirement::extension("github", ExtensionResource::Objects),
            &Action::Read
        ));
        assert!(!grants.allows(
            &Requirement::extension("github", ExtensionResource::Logs),
            &Action::Read
        ));
        assert!(!grants.allows(
            &Requirement::extension("cron", ExtensionResource::Objects),
            &Action::Read
        ));
        assert!(grants.allows(
            &Requirement::Extension {
                extension: Some("github".into()),
                resource: None
            },
            &Action::Read
        ));
    }

    #[test]
    fn global_secrets_are_separate_from_pipeline_secrets() {
        let user = system_role(SystemRoles::User);

        assert!(user.allows(&pipeline("default", "x", NR::Secrets), &Action::Read));
        assert!(!user.allows(&Requirement::Global(GlobalResource::Secrets), &Action::Read));
    }

    #[test]
    fn validate_rejects_bad_grants() {
        let bad = [
            namespace_grant("", None, &[NR::Runs], &[Action::Read]),
            namespace_grant("(", None, &[NR::Runs], &[Action::Read]),
            namespace_grant("default", Some(""), &[NR::Runs], &[Action::Read]),
            namespace_grant("default", None, &[], &[Action::Read]),
            namespace_grant("default", None, &[NR::Runs], &[]),
        ];

        for grant in bad {
            let grants = Grants {
                namespaces: vec![grant.clone()],
                ..Default::default()
            };
            assert!(grants.validate().is_err(), "{grant:?} should be invalid");
        }

        for role in system_roles() {
            role.grants.validate().unwrap();
        }
    }

    #[test]
    fn user_role_access() {
        let user = system_role(SystemRoles::User);
        let resources = [
            NR::Pipelines,
            NR::Configs,
            NR::Deployments,
            NR::Runs,
            NR::TaskExecutions,
            NR::Objects,
            NR::Secrets,
            NR::Subscriptions,
        ];

        for action in [Action::Read, Action::Write, Action::Delete] {
            for resource in &resources {
                assert!(user.allows(&pipeline("default", "x", resource.clone()), &action));
                assert!(!user.allows(&pipeline("other", "x", resource.clone()), &action));
            }
        }

        assert!(user.allows(&Requirement::Global(GlobalResource::Events), &Action::Read));
        assert!(!user.allows(&Requirement::Global(GlobalResource::Tokens), &Action::Read));
        assert!(!user.allows(&Requirement::Global(GlobalResource::Roles), &Action::Read));
        assert!(!user.allows(
            &Requirement::Extension {
                extension: None,
                resource: None
            },
            &Action::Read
        ));
    }

    #[test]
    fn anonymous_role_access() {
        let anonymous = system_role(SystemRoles::Anonymous);

        assert!(anonymous.allows(&pipeline("default", "x", NR::Pipelines), &Action::Read));
        assert!(anonymous.allows(&pipeline("default", "x", NR::Runs), &Action::Read));
        assert!(!anonymous.allows(&pipeline("default", "x", NR::Runs), &Action::Write));
        assert!(!anonymous.allows(&pipeline("default", "x", NR::Secrets), &Action::Read));
        assert!(!anonymous.allows(&pipeline("other", "x", NR::Pipelines), &Action::Read));
    }

    #[test]
    fn extension_role_access() {
        let extension = crate::api::extensions::extension_role("github").grants;

        for action in [Action::Read, Action::Write, Action::Delete] {
            assert!(extension.allows(
                &Requirement::extension("github", ExtensionResource::Objects),
                &action
            ));
            assert!(!extension.allows(
                &Requirement::extension("cron", ExtensionResource::Objects),
                &action
            ));
        }

        assert!(extension.allows(
            &Requirement::extension("github", ExtensionResource::Subscriptions),
            &Action::Read
        ));
        assert!(!extension.allows(
            &Requirement::extension("github", ExtensionResource::Logs),
            &Action::Read
        ));
        assert!(extension.allows(&pipeline("any", "x", NR::Runs), &Action::Write));
        assert!(extension.allows(&pipeline("any", "x", NR::TaskExecutions), &Action::Read));
        assert!(!extension.allows(&pipeline("any", "x", NR::Objects), &Action::Write));
        assert!(!extension.allows(&pipeline("any", "x", NR::Secrets), &Action::Read));
    }

    #[test]
    fn inject_api_token_role_access() {
        let token = crate::api::pipeline_configs::inject_api_token_role("devops", "build").grants;

        assert!(token.allows(&pipeline("devops", "build", NR::Objects), &Action::Write));
        assert!(token.allows(&pipeline("devops", "build", NR::Runs), &Action::Write));
        assert!(!token.allows(&pipeline("devops", "other", NR::Runs), &Action::Write));
        assert!(token.allows(&pipeline("devops", "other", NR::Runs), &Action::Read));
        assert!(token.allows(&pipeline("default", "other", NR::Runs), &Action::Read));
        assert!(!token.allows(&pipeline("other", "build", NR::Runs), &Action::Read));
        assert!(!token.allows(&pipeline("devops", "build", NR::Secrets), &Action::Read));
    }

    #[test]
    fn inject_api_token_roles_are_unique_per_namespace() {
        let a = crate::api::pipeline_configs::inject_api_token_role("one", "build");
        let b = crate::api::pipeline_configs::inject_api_token_role("two", "build");
        assert_ne!(a.id, b.id);
    }
}
