use super::{
    epoch_milli, event_utils, is_valid_identifier, storage, tokens, ApiState, PreflightOptions,
    RequestInfo, RequestMetadata,
};
use crate::http_error;
use anyhow::{bail, Context, Result};
use dropshot::{
    endpoint, HttpError, HttpResponseCreated, HttpResponseDeleted, HttpResponseOk, Path,
    RequestContext, TypedBody,
};
use http::StatusCode;
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Acquire;
use std::sync::Arc;
use strum::{Display, EnumString};
use tracing::error;

#[derive(Debug, Clone, Display, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
/// Resources are representitive group names for collections of endpoints and concepts within Gofer.
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
    fn from_str(input: &str) -> Option<Self> {
        let user_resource_target_split: Vec<&str> = input.split(':').collect();
        let user_resource = user_resource_target_split.first().unwrap().to_lowercase();
        let user_target = user_resource_target_split.get(1).unwrap_or(&"").to_string();

        let resource = match user_resource.as_str() {
            "all" => Resource::All,
            "configs" => Resource::Configs,
            "deployments" => Resource::Deployments,
            "events" => Resource::Events,
            "extensions" => Resource::Extensions(user_target),
            "namespaces" => Resource::Namespaces(user_target),
            "objects" => Resource::Objects,
            "permissions" => Resource::Permissions,
            "pipelines" => Resource::Pipelines(user_target),
            "runs" => Resource::Runs,
            "secrets" => Resource::Secrets,
            "subscriptions" => Resource::Subscriptions,
            "system" => Resource::System,
            "task_executions" => Resource::TaskExecutions,
            "tokens" => Resource::Tokens,
            _ => {
                return None;
            }
        };

        Some(resource)
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
        let mut resources = vec![];

        for resource_str in value.resources {
            let resource = match Resource::from_str(&resource_str) {
                Some(resource) => resource,
                None => {
                    bail!(
                        "Could not parse resource '{}', not a valid resource type",
                        resource_str
                    );
                }
            };

            resources.push(resource);
        }

        Ok(InternalPermission {
            resources,
            actions: value.actions,
        })
    }
}

impl From<InternalPermission> for Permission {
    fn from(value: InternalPermission) -> Self {
        let mut resources = vec![];

        for resource in value.resources {
            let internal_resource = Resource::to_string(&resource);
            resources.push(internal_resource);
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
    /// otherwise include the path identifer. This is compared against the user's token permissions to see if they have
    /// access.
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
        } else {
            self.get_auth_context(request).await?
        };
        let api_version = super::check_version_handler(request)?;

        // If the user is admin they automatically have access to every route.
        if auth_ctx.roles.contains(&SystemRoles::Admin.to_string())
            || auth_ctx.roles.contains(&SystemRoles::Bootstrap.to_string())
        {
            return Ok(RequestMetadata {
                auth: auth_ctx,
                api_version,
            });
        } else if options.admin_only {
            return Err(HttpError::for_client_error(
                None,
                StatusCode::UNAUTHORIZED,
                "Route requires admin level token".into(),
            ));
        }

        let mut conn = match self.storage.conn().await {
            Ok(conn) => conn,
            Err(e) => {
                return Err(crate::http_error!(
                    "Could not open connection to database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    "None".into(),
                    Some(e.into())
                ));
            }
        };

        // Check that token actually has the correct permissions for the targeted resource.
        for role_id in &auth_ctx.roles {
            let storage_role = match storage::roles::get(&mut conn, role_id).await {
                Ok(role) => role,
                Err(e) => match e {
                    storage::StorageError::NotFound => {
                        // If we find a role that doesn't exist then we don't care.
                        continue;
                    }
                    _ => {
                        return Err(http_error!(
                            "Could not query database for roles during authentication permission checking",
                            http::StatusCode::INTERNAL_SERVER_ERROR,
                            "0".into(),
                            Some(e.into())
                        ));
                    }
                },
            };

            let role = InternalRole::try_from(storage_role).map_err(|err| {
                error!(message = "Could not serialize role from storage", error = %err);
                http_error!(
                    "Could not parse role object from database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    "0".into(),
                    Some(err.into())
                )
            })?;

            let mut all_resources_valid = true;

            // TODO(): This should be simplified. This is the brute force solution, but I'm unsure of what an optimal
            // solution would look like.
            //
            // First we need to iterate through the route's resource declarations and make sure each of them are valid.
            for route_resource_declaration in &options.resources {
                let mut resource_valid = false;

                // For each of the route's resource declarations we need to iterate through all the user's
                // role permissions as they may have many.
                for permission in &role.permissions {
                    // For each of the user's role permissions, we look up the resource/target combinations and
                    // we must check that at least one of them satisfies the route resource constraint.
                    for user_resource in &permission.resources {
                        match (user_resource, route_resource_declaration) {
                            // Certain resources can have specific targets for which the user only has access to.
                            // This is represented by a regex on the Token's version of a 'Resource' object.
                            // We compare this to the resource target defined by the route being accessed to see if its a match.
                            //
                            // This functionality enables the ability for users to be able to be granted specific permissions for
                            // a single or set of specific resources. For example, a user might be created a token that has
                            // access to only roles whose ids start with 'devops_'
                            (
                                Resource::Namespaces(user_target),
                                Resource::Namespaces(route_target),
                            ) => {
                                // We first check if the target is empty here because some routes like listing routes
                                // don't have specific targets. So in order to get them to match, we leave them empty and
                                // then check for the empty target. We're rather not compile the user token resource regex
                                // if we don't have to because that wastes time and CPU cycles so we put this check ahead
                                // of that.
                                if route_target.is_empty()
                                    && permission.actions.contains(&options.action)
                                {
                                    resource_valid = true;
                                    break;
                                }

                                if let Ok(regex) = Regex::new(user_target) {
                                    if regex.is_match(route_target)
                                        && permission.actions.contains(&options.action)
                                    {
                                        resource_valid = true;
                                        break;
                                    }
                                }
                            }
                            (
                                Resource::Extensions(user_target),
                                Resource::Extensions(route_target),
                            ) => {
                                if route_target.is_empty()
                                    && permission.actions.contains(&options.action)
                                {
                                    resource_valid = true;
                                    break;
                                }

                                if let Ok(regex) = Regex::new(user_target) {
                                    if regex.is_match(route_target)
                                        && permission.actions.contains(&options.action)
                                    {
                                        resource_valid = true;
                                        break;
                                    }
                                }
                            }
                            (
                                Resource::Pipelines(user_target),
                                Resource::Pipelines(route_target),
                            ) => {
                                if route_target.is_empty()
                                    && permission.actions.contains(&options.action)
                                {
                                    resource_valid = true;
                                    break;
                                }

                                if let Ok(regex) = Regex::new(user_target) {
                                    if regex.is_match(route_target)
                                        && permission.actions.contains(&options.action)
                                    {
                                        resource_valid = true;
                                        break;
                                    }
                                }
                            }

                            // These resources don't have specific targets so there is no need to check the inner values.
                            (Resource::All, _) => {
                                if permission.actions.contains(&options.action) {
                                    resource_valid = true;
                                    break;
                                }
                            }
                            (Resource::Configs, Resource::Configs)
                            | (Resource::Deployments, Resource::Deployments)
                            | (Resource::Events, Resource::Events)
                            | (Resource::Objects, Resource::Objects)
                            | (Resource::Permissions, Resource::Permissions)
                            | (Resource::Runs, Resource::Runs)
                            | (Resource::Secrets, Resource::Secrets)
                            | (Resource::Subscriptions, Resource::Subscriptions)
                            | (Resource::System, Resource::System)
                            | (Resource::TaskExecutions, Resource::TaskExecutions)
                            | (Resource::Tokens, Resource::Tokens) => {
                                if permission.actions.contains(&options.action) {
                                    resource_valid = true;
                                    break;
                                }
                            }
                            _ => continue, // Catch-all for cases where the resource types don't match
                        }

                        if resource_valid {
                            break;
                        }
                    } // for user_resource in &permission.resources

                    if resource_valid {
                        break;
                    }
                } // for permission in &role.permissions

                if !resource_valid {
                    all_resources_valid = false;
                    break;
                }
            } // for route_resource_declaration in &options.resources

            if all_resources_valid {
                return Ok(RequestMetadata {
                    auth: auth_ctx,
                    api_version,
                });
            }
        } // for role_id in &auth_ctx.roles

        Err(HttpError::for_client_error(
            None,
            StatusCode::UNAUTHORIZED,
            format!(
                "Token does not contain role required for access to this route. \
                Route requires: resource '{:?}' and action '{}' permissions",
                options.resources, &options.action
            ),
        ))
    }

    /// Checks request authentication and returns valid auth information.
    async fn get_auth_context(&self, request: &RequestInfo) -> Result<AuthContext, HttpError> {
        let auth_header =
            request
                .headers()
                .get("Authorization")
                .ok_or(HttpError::for_bad_request(
                    None,
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

        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        let mut conn = match self.storage.conn().await {
            Ok(conn) => conn,
            Err(e) => {
                return Err(crate::http_error!(
                    "Could not open connection to database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
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
                        StatusCode::UNAUTHORIZED,
                        "Unauthorized".into(),
                    ));
                }
                _ => {
                    return Err(crate::http_error!(
                        "Could not query database",
                        http::StatusCode::INTERNAL_SERVER_ERROR,
                        "None".into(),
                        Some(e.into())
                    ));
                }
            },
        };

        let token = tokens::Token::try_from(storage_token).map_err(|e| {
            crate::http_error!(
                "Could not parse token object from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                "None".into(),
                Some(e.into())
            )
        })?;

        if token.disabled {
            return Err(HttpError::for_client_error(
                None,
                http::StatusCode::UNAUTHORIZED,
                "Token disabled".into(),
            ));
        }

        // If the token expires is 0 it's valid forever.
        if token.expires != 0 && epoch_milli() > token.expires {
            return Err(HttpError::for_client_error(
                None,
                http::StatusCode::UNAUTHORIZED,
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
        "A common user role that has access to the default namespace, but read-only for most other things.",
        vec![
            InternalPermission {
                resources: vec![Resource::All],
                actions: vec![Action::Read],
            },
            InternalPermission {
                resources: vec![Resource::Namespaces("^default$".into()),
                    Resource::Pipelines(".*".into()),
                    Resource::Configs, Resource::Deployments,
                    Resource::Subscriptions, Resource::Objects, Resource::Secrets],
                actions: vec![Action::Read, Action::Write,  Action::Delete],
            }
        ],
        true,
    );

    let roles = vec![bootstrap_role, admin_role, user_role];

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            error!(message = "Could not open connection to database", error = %e);
            bail!("Could not open connection to database")
        }
    };

    for role in roles {
        let storage_role: storage::roles::Role = role.try_into().context(
            "Could not seralized role into storage role \
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

    let storage_roles = match storage::roles::list(&mut conn).await {
        Ok(roles) => roles,
        Err(e) => {
            return Err(http_error!(
                "Could not get objects from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
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
                http::StatusCode::INTERNAL_SERVER_ERROR,
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
            http::StatusCode::INTERNAL_SERVER_ERROR,
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

    let storage_role = match storage::roles::get(&mut conn, &path.role_id).await {
        Ok(role) => role,
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

    let internal_role = InternalRole::try_from(storage_role).map_err(|e| {
        http_error!(
            "Could not parse object from database",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    let role: Role = internal_role.try_into().map_err(|e: anyhow::Error| {
        http_error!(
            "Could not parse object into api contract object",
            http::StatusCode::INTERNAL_SERVER_ERROR,
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
            },
        )
        .await?;

    if let Err(e) = is_valid_identifier(&body.id) {
        return Err(HttpError::for_bad_request(
            None,
            format!(
                "'{}' is not a valid identifier; {}",
                &body.id,
                &e.to_string()
            ),
        ));
    };

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

    let permissions: Result<Vec<InternalPermission>> = body
        .permissions
        .into_iter()
        .map(|permission| permission.try_into())
        .collect();
    let permissions = permissions.map_err(|e| {
        http_error!(
            "Could not parse permissions from api contract",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

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
                http::StatusCode::INTERNAL_SERVER_ERROR,
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
                    StatusCode::CONFLICT,
                    "role entry already exists".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not insert objects into database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
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
            http::StatusCode::INTERNAL_SERVER_ERROR,
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
        let permissions: Option<String> = match value.permissions {
            Some(value) => {
                let permission_str = serde_json::to_string(&value).with_context(|| {
                    format!(
                        "Could not parse field 'permissions' from value '{:#?}'",
                        value
                    )
                })?;

                Some(permission_str)
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

    let updatable_fields = match storage::roles::UpdatableFields::try_from(body.clone()) {
        Ok(fields) => fields,
        Err(e) => {
            return Err(http_error!(
                "Could not serialize role for database insertion",
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
                "Could not open transaction to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
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
                    http::StatusCode::INTERNAL_SERVER_ERROR,
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
            StatusCode::FORBIDDEN,
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
                    http::StatusCode::INTERNAL_SERVER_ERROR,
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
                    http::StatusCode::INTERNAL_SERVER_ERROR,
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
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    let role = internal_role.try_into().map_err(|e: anyhow::Error| {
        http_error!(
            "Could not parse role into api contract object",
            http::StatusCode::INTERNAL_SERVER_ERROR,
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
                "Could not open transaction to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
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
                    http::StatusCode::INTERNAL_SERVER_ERROR,
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
            StatusCode::FORBIDDEN,
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
                    http::StatusCode::INTERNAL_SERVER_ERROR,
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
