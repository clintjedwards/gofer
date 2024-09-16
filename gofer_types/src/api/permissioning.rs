use crate::storage;
use anyhow::bail;
use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

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

impl TryFrom<storage::role::Role> for InternalRole {
    type Error = anyhow::Error;

    fn try_from(value: storage::role::Role) -> Result<Self> {
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

impl TryFrom<InternalRole> for storage::role::Role {
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

    /// Special token given to extensions.
    Extension,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListRolesResponse {
    /// A list of all roles.
    pub roles: Vec<Role>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateRoleRequest {
    /// Short description about what the role is used for.
    pub description: Option<String>,

    /// Permissions that the role allows.
    pub permissions: Option<Vec<Permission>>,
}

impl TryFrom<UpdateRoleRequest> for storage::role::UpdatableFields {
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
