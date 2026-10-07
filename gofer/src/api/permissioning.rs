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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// Resources are representative group names for collections of endpoints and concepts within Gofer.
/// It's used mostly by the permissioning system to identify collections and grant users permissions
/// to those collections.
pub enum Resource {
    All,
    Configs,
    Deployments,
    Events,
    Extensions(String),
    Namespaces(String),
    Objects,
    Permissions,
    Pipelines(String),
    Runs,
    Secrets,
    Subscriptions,
    System,
    TaskExecutions,
    Tokens,
}

impl Resource {
    /// Parses the user facing "resource:target" form. Targeted resources must include a valid regex target and
    /// untargeted resources must not include one, so that a role can never be broader than it looks.
    fn parse(input: &str) -> Result<Self> {
        let (name, target) = match input.split_once(':') {
            Some((name, target)) => (name.to_lowercase(), Some(target.to_string())),
            None => (input.to_lowercase(), None),
        };

        let untargeted = match name.as_str() {
            "all" => Some(Resource::All),
            "configs" => Some(Resource::Configs),
            "deployments" => Some(Resource::Deployments),
            "events" => Some(Resource::Events),
            "objects" => Some(Resource::Objects),
            "permissions" => Some(Resource::Permissions),
            "runs" => Some(Resource::Runs),
            "secrets" => Some(Resource::Secrets),
            "subscriptions" => Some(Resource::Subscriptions),
            "system" => Some(Resource::System),
            "task_executions" => Some(Resource::TaskExecutions),
            "tokens" => Some(Resource::Tokens),
            "extensions" | "namespaces" | "pipelines" => None,
            _ => bail!("'{name}' is not a valid resource type"),
        };

        if let Some(resource) = untargeted {
            if target.is_some() {
                bail!("resource '{name}' does not accept a target");
            }
            return Ok(resource);
        }

        let target = match target {
            Some(target) if !target.is_empty() => target,
            _ => bail!("resource '{name}' requires a target, e.g. '{name}:.*' to match everything"),
        };

        Regex::new(&anchored(&target)).with_context(|| {
            format!("target '{target}' for resource '{name}' is not a valid regex")
        })?;

        Ok(match name.as_str() {
            "extensions" => Resource::Extensions(target),
            "namespaces" => Resource::Namespaces(target),
            _ => Resource::Pipelines(target),
        })
    }

    /// Reports whether this resource, as granted by a role, covers the resource a route requires.
    ///
    /// An empty route target is used by listing routes, which don't target a single object. It matches any grant
    /// for that resource type and the handler is then responsible for filtering results with
    /// [`RequestMetadata::allows`].
    fn covers(&self, required: &Resource) -> bool {
        match (self, required) {
            (Resource::All, _) => true,
            (Resource::Extensions(granted), Resource::Extensions(target))
            | (Resource::Namespaces(granted), Resource::Namespaces(target))
            | (Resource::Pipelines(granted), Resource::Pipelines(target)) => {
                target.is_empty()
                    || Regex::new(&anchored(granted)).is_ok_and(|regex| regex.is_match(target))
            }
            (granted, required) => granted == required,
        }
    }
}

/// Targets always match the whole identifier. Without this, a target of 'default' would also match 'not-default'.
fn anchored(target: &str) -> String {
    format!("^(?:{target})$")
}

impl std::fmt::Display for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Resource::All => write!(f, "all"),
            Resource::Configs => write!(f, "configs"),
            Resource::Deployments => write!(f, "deployments"),
            Resource::Events => write!(f, "events"),
            Resource::Extensions(target) => write!(f, "extensions:{target}"),
            Resource::Namespaces(target) => write!(f, "namespaces:{target}"),
            Resource::Objects => write!(f, "objects"),
            Resource::Permissions => write!(f, "permissions"),
            Resource::Pipelines(target) => write!(f, "pipelines:{target}"),
            Resource::Runs => write!(f, "runs"),
            Resource::Secrets => write!(f, "secrets"),
            Resource::Subscriptions => write!(f, "subscriptions"),
            Resource::System => write!(f, "system"),
            Resource::TaskExecutions => write!(f, "task_executions"),
            Resource::Tokens => write!(f, "tokens"),
        }
    }
}

#[derive(Debug, Clone, Display, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum Action {
    Read,
    Write,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InternalRole {
    /// Alphanumeric with dashes only
    pub id: String,
    pub description: String,
    pub permissions: Vec<InternalPermission>,

    /// If this role was created by Gofer itself. System roles cannot be modified.
    pub system_role: bool,
}

impl InternalRole {
    pub fn new(
        id: &str,
        description: &str,
        permissions: Vec<InternalPermission>,
        system_role: bool,
    ) -> Self {
        InternalRole {
            id: id.into(),
            description: description.into(),
            permissions,
            system_role,
        }
    }
}

impl TryFrom<storage::roles::Role> for InternalRole {
    type Error = anyhow::Error;

    fn try_from(value: storage::roles::Role) -> Result<Self> {
        let permissions: Vec<InternalPermission> = serde_json::from_str(&value.permissions)
            .with_context(|| {
                format!(
                    "Could not parse field 'permissions' from storage value '{}'",
                    value.permissions
                )
            })?;

        Ok(InternalRole {
            id: value.id,
            description: value.description,
            permissions,
            system_role: value.system_role,
        })
    }
}

impl TryFrom<InternalRole> for storage::roles::Role {
    type Error = anyhow::Error;

    fn try_from(value: InternalRole) -> Result<Self> {
        let permissions = serde_json::to_string(&value.permissions).with_context(|| {
            format!(
                "Could not parse field 'permissions' from storage value; '{:#?}'",
                value.permissions
            )
        })?;

        Ok(Self {
            id: value.id,
            description: value.description,
            permissions,
            system_role: value.system_role,
        })
    }
}

impl TryFrom<InternalRole> for Role {
    type Error = anyhow::Error;

    fn try_from(value: InternalRole) -> Result<Self> {
        let mut permissions = vec![];

        for permission_real in value.permissions {
            let permission: Permission = permission_real.into();
            permissions.push(permission);
        }

        Ok(Role {
            id: value.id,
            description: value.description,
            permissions,
            system_role: value.system_role,
        })
    }
}

/// Role is exactly like ['InternalRole'] except it abstracts away the type specification of the permissions.
/// This is used to interface with the user via the API.
///
/// The ['InternalRole'] object cannot be used due to issues with openapi and the generation of the ['Resource'] types.
/// Instead we replace the complicated enum system with a simple string declaration and manually do the translation
/// between the two types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Role {
    /// Alphanumeric with dashes only
    pub id: String,
    pub description: String,
    pub permissions: Vec<Permission>,

    /// If this role was created by Gofer itself. System roles cannot be modified.
    pub system_role: bool,
}

impl TryFrom<Role> for InternalRole {
    type Error = anyhow::Error;

    fn try_from(value: Role) -> Result<Self> {
        let mut permissions = vec![];

        for permission in value.permissions {
            let internal_permission: InternalPermission = permission.try_into()?;
            permissions.push(internal_permission);
        }

        Ok(InternalRole {
            id: value.id,
            description: value.description,
            permissions,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InternalPermission {
    /// Which resource we're targeting. A resource is also know as a collection in REST APIs. It refers to a particular
    /// group of endpoints. Resources might also have specific objects being targeted.
    pub resources: Vec<Resource>,

    /// Actions are specific operations a user is allowed to perform for those resources. Endpoints will define which
    /// "action" they belong under.
    pub actions: Vec<Action>,
}

/// Permission is exactly like ['InternalPermission'] except it abstracts away the type specification
/// of the permissions. This is used to interface with the user via the API.
///
/// The ['InternalPermissions'] object cannot be used due to issues with openapi and the generation of the
/// ['Resource'] types. Instead we replace the enum system with a simple string declaration and manually
/// do the translation between the two types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Permission {
    /// Which resource to target. A resource refers to a particular group of endpoints. Resources might also have
    /// specific objects being targeted. (Denoted by a '(target)')
    ///
    /// The current list of resources:
    ///
    /// "all"
    /// "configs"
    /// "deployments"
    /// "events"
    /// "extensions:(target)"
    /// "namespaces:(target)"
    /// "objects"
    /// "permissions"
    /// "pipelines:(target)"
    /// "runs"
    /// "secrets"
    /// "subscriptions"
    /// "system"
    /// "task_executions"
    /// "tokens"
    ///
    /// Example: ["configs", "namespaces:^default$", "pipelines:.*"]
    pub resources: Vec<String>,

    /// Actions are specific operations a user is allowed to perform for those resources. Endpoints will define which
    /// "action" they belong under.
    pub actions: Vec<Action>,
}

impl TryFrom<Permission> for InternalPermission {
    type Error = anyhow::Error;

    fn try_from(value: Permission) -> Result<Self> {
        let resources = value
            .resources
            .iter()
            .map(|resource| Resource::parse(resource))
            .collect::<Result<Vec<_>>>()?;

        Ok(InternalPermission {
            resources,
            actions: value.actions,
        })
    }
}

impl InternalPermission {
    /// A permission is a standalone grant; it allows a route only if it covers every resource the route requires
    /// along with the route's action. Resources are never combined across separate permissions.
    pub fn allows(&self, required: &[Resource], action: &Action) -> bool {
        self.actions.contains(action)
            && required.iter().all(|required| {
                self.resources
                    .iter()
                    .any(|granted| granted.covers(required))
            })
    }
}

impl From<InternalPermission> for Permission {
    fn from(value: InternalPermission) -> Self {
        let mut resources = vec![];

        for resource in value.resources {
            resources.push(resource.to_string());
        }

        Permission {
            resources,
            actions: value.actions,
        }
    }
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

impl ApiState {
    /// Resolves request specific context for handlers. This is used to perform auth checks and generally other
    /// actions that should happen before a route runs it's handler.
    ///
    /// **Should be called at the start of every handler**, regardless of if that handler needs auth or req_context.
    ///
    /// We specifically use a struct here so that the reader can easily verify which options are in which state for the
    /// route that it is included. The different options here map to different actions that are checked per call.
    ///
    /// When defining a preflight option resource, give the resource an empty string to communicate no specific targets
    /// otherwise include the path identifier. This is compared against the user's token permissions to see if they have
    /// access. Routes that use an empty target must filter what they return with [`RequestMetadata::allows`].
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

        let permissions = if admin {
            vec![]
        } else {
            self.get_permissions(&auth_ctx.roles).await?
        };

        let metadata = RequestMetadata {
            auth: auth_ctx,
            api_version,
            admin,
            permissions,
        };

        if !metadata.allows(&options.resources, &options.action) {
            return Err(HttpError::for_client_error(
                None,
                ClientErrorStatusCode::FORBIDDEN,
                format!(
                    "Token does not have permission to access this route. \
                    Route requires a single permission that grants action '{}' on resources [{}]",
                    options.action,
                    options
                        .resources
                        .iter()
                        .map(|resource| resource.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }

        Ok(metadata)
    }

    /// Collects every permission from the given roles. Since each permission is evaluated on its own there is no
    /// need to keep track of which role it came from.
    async fn get_permissions(
        &self,
        role_ids: &[String],
    ) -> Result<Vec<InternalPermission>, HttpError> {
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

        let mut permissions = vec![];

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

            let role = InternalRole::try_from(storage_role).map_err(|err| {
                error!(message = "Could not serialize role from storage", error = %err);
                http_error!(
                    "Could not parse role object from database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    "0".into(),
                    Some(err.into())
                )
            })?;

            permissions.extend(role.permissions);
        }

        Ok(permissions)
    }

    /// Checks request authentication and returns valid auth information.
    async fn get_auth_context(&self, request: &RequestInfo) -> Result<AuthContext, HttpError> {
        let auth_header =
            request
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
        if !auth_header.starts_with("Bearer ") {
            return Err(HttpError::for_bad_request(
                None,
                "Authorization header malformed; should start with 'Bearer'".into(),
            ));
        }

        let token = auth_header.strip_prefix("Bearer ").unwrap();

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

/// The roles Gofer ships with. Changing these requires a database migration, since existing installs keep whatever
/// was inserted when they were first started.
fn system_roles() -> Vec<InternalRole> {
    let bootstrap_role = InternalRole::new(
        &SystemRoles::Bootstrap.to_string(),
        "The original role that all other tokens/roles are created from.",
        vec![InternalPermission {
            resources: vec![Resource::All],
            actions: vec![Action::Read, Action::Write, Action::Delete],
        }],
        true,
    );

    let admin_role = InternalRole::new(
        &SystemRoles::Admin.to_string(),
        "Essentially root access. This role has unmitigated access to every resource.",
        vec![InternalPermission {
            resources: vec![Resource::All],
            actions: vec![Action::Read, Action::Write, Action::Delete],
        }],
        true,
    );

    let user_role = InternalRole::new(
        &SystemRoles::User.to_string(),
        "A common user role that has read/write/delete access to the default namespace.",
        vec![InternalPermission {
            resources: vec![
                Resource::Namespaces("^default$".into()),
                Resource::Pipelines(".*".into()),
                Resource::Configs,
                Resource::Deployments,
                Resource::Events,
                Resource::Objects,
                Resource::Runs,
                Resource::Secrets,
                Resource::Subscriptions,
                Resource::TaskExecutions,
            ],
            actions: vec![Action::Read, Action::Write, Action::Delete],
        }],
        true,
    );

    let anon_role = InternalRole::new(
        &SystemRoles::Anonymous.to_string(),
        "The role given when a user has not signed in at all.",
        vec![InternalPermission {
            resources: vec![
                Resource::Namespaces("^default$".into()),
                Resource::Pipelines(".*".into()),
                Resource::Runs,
            ],
            actions: vec![Action::Read],
        }],
        true,
    );

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
                resources: vec![Resource::Permissions],
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

    let mut roles: Vec<InternalRole> = vec![];

    for storage_role in storage_roles {
        let role = InternalRole::try_from(storage_role).map_err(|e| {
            http_error!(
                "Could not parse object from database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        roles.push(role);
    }

    let roles: Result<Vec<Role>> = roles.into_iter().map(|role| role.try_into()).collect();
    let roles = roles.map_err(|e| {
        http_error!(
            "Could not parse object role from database into api contract",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

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
                resources: vec![Resource::Permissions],
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

    let internal_role = InternalRole::try_from(storage_role).map_err(|e| {
        http_error!(
            "Could not parse object from database",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    let role: Role = internal_role.try_into().map_err(|e: anyhow::Error| {
        http_error!(
            "Could not parse object into api contract object",
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

    /// Permissions that the role allows.
    pub permissions: Vec<Permission>,
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
                resources: vec![Resource::Permissions],
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

    let permissions: Result<Vec<InternalPermission>> = body
        .permissions
        .into_iter()
        .map(|permission| permission.try_into())
        .collect();

    let permissions = permissions
        .map_err(|e| HttpError::for_bad_request(None, format!("Invalid permissions; {e:#}")))?;

    let new_role = InternalRole {
        id: body.id.to_string(),
        description: body.description.to_string(),
        permissions,
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

    let role = new_role.try_into().map_err(|e: anyhow::Error| {
        http_error!(
            "Could not parse role into api contract object",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    let resp = CreateRoleResponse { role };

    Ok(HttpResponseCreated(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateRoleRequest {
    /// Short description about what the role is used for.
    pub description: Option<String>,

    /// Permissions that the role allows.
    pub permissions: Option<Vec<Permission>>,
}

impl TryFrom<UpdateRoleRequest> for storage::roles::UpdatableFields {
    type Error = anyhow::Error;

    fn try_from(value: UpdateRoleRequest) -> Result<Self> {
        // Permissions have to go through the internal type first; storage holds the internal representation and
        // this is also where user supplied resources get validated.
        let permissions = match value.permissions {
            Some(permissions) => {
                let permissions = permissions
                    .into_iter()
                    .map(InternalPermission::try_from)
                    .collect::<Result<Vec<_>>>()?;

                Some(
                    serde_json::to_string(&permissions)
                        .context("Could not serialize permissions")?,
                )
            }
            None => None,
        };

        Ok(Self {
            description: value.description,
            permissions,
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
                resources: vec![Resource::Permissions],
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
                format!("Invalid permissions; {e:#}"),
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

    let internal_role = InternalRole::try_from(storage_role).map_err(|e| {
        http_error!(
            "Could not parse object from database",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    let role = internal_role.try_into().map_err(|e: anyhow::Error| {
        http_error!(
            "Could not parse role into api contract object",
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
                resources: vec![Resource::Permissions],
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

    fn permission(resources: &[&str], actions: &[Action]) -> InternalPermission {
        Permission {
            resources: resources.iter().map(|r| r.to_string()).collect(),
            actions: actions.to_vec(),
        }
        .try_into()
        .unwrap()
    }

    fn system_role(id: SystemRoles) -> InternalRole {
        system_roles()
            .into_iter()
            .find(|role| role.id == id.to_string())
            .unwrap()
    }

    fn allowed(permissions: &[InternalPermission], resources: &[Resource], action: Action) -> bool {
        permissions
            .iter()
            .any(|permission| permission.allows(resources, &action))
    }

    fn pipeline_route(namespace: &str, pipeline: &str, extra: &[Resource]) -> Vec<Resource> {
        let mut resources = vec![
            Resource::Namespaces(namespace.into()),
            Resource::Pipelines(pipeline.into()),
        ];
        resources.extend_from_slice(extra);
        resources
    }

    #[test]
    fn parse_rejects_invalid_resources() {
        assert!(Resource::parse("nonsense").is_err());
        assert!(Resource::parse("namespaces").is_err());
        assert!(Resource::parse("namespaces:").is_err());
        assert!(Resource::parse("namespaces:(").is_err());
        assert!(Resource::parse("secrets:.*").is_err());
    }

    #[test]
    fn parse_keeps_colons_in_targets() {
        assert_eq!(
            Resource::parse("namespaces:a:b").unwrap(),
            Resource::Namespaces("a:b".into())
        );
    }

    #[test]
    fn resources_round_trip_through_api_format() {
        for input in [
            "all",
            "secrets",
            "task_executions",
            "namespaces:^default$",
            "pipelines:.*",
        ] {
            assert_eq!(Resource::parse(input).unwrap().to_string(), input);
        }
    }

    #[test]
    fn targets_match_whole_identifier() {
        let granted = Resource::Namespaces("default".into());
        assert!(granted.covers(&Resource::Namespaces("default".into())));
        assert!(!granted.covers(&Resource::Namespaces("not-default".into())));
        assert!(!granted.covers(&Resource::Namespaces("default-two".into())));

        let prefix = Resource::Namespaces("devops.*".into());
        assert!(prefix.covers(&Resource::Namespaces("devops-test".into())));
        assert!(!prefix.covers(&Resource::Namespaces("not-devops".into())));
    }

    #[test]
    fn empty_route_target_matches_any_grant_of_that_type() {
        let granted = Resource::Namespaces("^devops$".into());
        assert!(granted.covers(&Resource::Namespaces("".into())));
        assert!(!granted.covers(&Resource::Pipelines("".into())));
    }

    #[test]
    fn all_covers_everything() {
        let permissions = [permission(&["all"], &[Action::Read])];
        assert!(allowed(
            &permissions,
            &pipeline_route("anything", "anything", &[Resource::Secrets]),
            Action::Read
        ));
        assert!(!allowed(&permissions, &[Resource::Secrets], Action::Write));
    }

    #[test]
    fn permissions_do_not_combine_targets() {
        let permissions = [
            permission(
                &["namespaces:frontend", "pipelines:website"],
                &[Action::Read],
            ),
            permission(
                &["namespaces:backend", "pipelines:billing"],
                &[Action::Read],
            ),
        ];

        assert!(allowed(
            &permissions,
            &pipeline_route("frontend", "website", &[]),
            Action::Read
        ));
        assert!(allowed(
            &permissions,
            &pipeline_route("backend", "billing", &[]),
            Action::Read
        ));
        assert!(!allowed(
            &permissions,
            &pipeline_route("frontend", "billing", &[]),
            Action::Read
        ));
        assert!(!allowed(
            &permissions,
            &pipeline_route("backend", "website", &[]),
            Action::Read
        ));
    }

    #[test]
    fn permissions_do_not_combine_untargeted_resources() {
        let permissions = [
            permission(
                &["namespaces:default", "pipelines:.*", "runs"],
                &[Action::Read],
            ),
            permission(
                &["namespaces:devops", "pipelines:.*", "secrets"],
                &[Action::Read],
            ),
        ];

        assert!(allowed(
            &permissions,
            &pipeline_route("devops", "x", &[Resource::Secrets]),
            Action::Read
        ));
        assert!(!allowed(
            &permissions,
            &pipeline_route("default", "x", &[Resource::Secrets]),
            Action::Read
        ));
    }

    #[test]
    fn permissions_do_not_combine_actions() {
        let permissions = [
            permission(
                &["namespaces:default", "pipelines:.*", "runs"],
                &[Action::Read],
            ),
            permission(&["secrets"], &[Action::Write]),
        ];

        assert!(!allowed(
            &permissions,
            &pipeline_route("default", "x", &[Resource::Runs]),
            Action::Write
        ));
    }

    #[test]
    fn user_role_access() {
        let user = system_role(SystemRoles::User).permissions;

        for action in [Action::Read, Action::Write, Action::Delete] {
            for extra in [
                vec![],
                vec![Resource::Configs],
                vec![Resource::Deployments],
                vec![Resource::Objects],
                vec![Resource::Runs],
                vec![Resource::Runs, Resource::Objects],
                vec![Resource::Runs, Resource::TaskExecutions],
                vec![Resource::Secrets],
                vec![Resource::Subscriptions],
            ] {
                assert!(
                    allowed(
                        &user,
                        &pipeline_route("default", "x", &extra),
                        action.clone()
                    ),
                    "user should have {action} on default/{extra:?}"
                );
                assert!(
                    !allowed(&user, &pipeline_route("other", "x", &extra), action.clone()),
                    "user should not have {action} on other/{extra:?}"
                );
            }
        }

        assert!(allowed(&user, &[Resource::Events], Action::Read));
        assert!(!allowed(
            &user,
            &[Resource::Extensions("x".into())],
            Action::Read
        ));
        assert!(!allowed(&user, &[Resource::Tokens], Action::Read));
        assert!(!allowed(&user, &[Resource::Permissions], Action::Read));
    }

    #[test]
    fn anonymous_role_access() {
        let anonymous = system_role(SystemRoles::Anonymous).permissions;

        assert!(allowed(
            &anonymous,
            &pipeline_route("default", "", &[]),
            Action::Read
        ));
        assert!(allowed(
            &anonymous,
            &pipeline_route("default", "x", &[]),
            Action::Read
        ));
        assert!(allowed(
            &anonymous,
            &pipeline_route("default", "x", &[Resource::Runs]),
            Action::Read
        ));
        assert!(!allowed(
            &anonymous,
            &pipeline_route("default", "x", &[Resource::Runs]),
            Action::Write
        ));
        assert!(!allowed(
            &anonymous,
            &pipeline_route("default", "x", &[Resource::Secrets]),
            Action::Read
        ));
        assert!(!allowed(
            &anonymous,
            &pipeline_route("other", "x", &[]),
            Action::Read
        ));
    }

    #[test]
    fn extension_role_access() {
        let extension = crate::api::extensions::extension_role("github").permissions;
        let own = Resource::Extensions("github".into());
        let other = Resource::Extensions("cron".into());

        for action in [Action::Read, Action::Write, Action::Delete] {
            assert!(allowed(
                &extension,
                &[own.clone(), Resource::Objects],
                action.clone()
            ));
            assert!(!allowed(
                &extension,
                &[other.clone(), Resource::Objects],
                action.clone()
            ));
        }

        assert!(allowed(
            &extension,
            &pipeline_route("any", "x", &[Resource::Runs]),
            Action::Write
        ));
        assert!(allowed(
            &extension,
            &pipeline_route("any", "x", &[Resource::Runs, Resource::TaskExecutions]),
            Action::Read
        ));
        assert!(!allowed(
            &extension,
            &pipeline_route("any", "x", &[Resource::Objects]),
            Action::Write
        ));
        assert!(!allowed(
            &extension,
            &pipeline_route("any", "x", &[Resource::Secrets]),
            Action::Read
        ));
    }

    #[test]
    fn migration_matches_role_definitions() {
        let migration = include_str!("../storage/migrations/1_permission_fixes.sql");

        let user = serde_json::to_string(&system_role(SystemRoles::User).permissions).unwrap();
        assert!(migration.contains(&format!("'{user}'")));

        let extension = serde_json::to_string(
            &crate::api::extensions::extension_role("EXTENSION_ID").permissions,
        )
        .unwrap();
        let templated = extension.replace("EXTENSION_ID", "' || substr(id, 11) || '");
        assert!(migration.contains(&format!("'{templated}'")));
    }
}
