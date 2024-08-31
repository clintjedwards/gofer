use crate::cli::Cli;
use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use comfy_table::{presets::ASCII_MARKDOWN, Cell, CellAlignment, Color, ContentArrangement};
use gofer_sdk::api::types::{Action, Permission, Resource};
use polyfmt::{error, print, println, question, success};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Args, Clone)]
pub struct RoleSubcommands {
    #[clap(subcommand)]
    pub command: RoleCommands,
}

#[derive(Debug, Subcommand, Clone)]
pub enum RoleCommands {
    /// List all roles.
    List,

    /// Fetch information about an individual role.
    Get {
        /// Role Identifier.
        id: String,
    },

    /// Create a new role.
    Create {
        /// Role Identifier.
        ///
        /// Must be:
        /// * 32 > characters < 3
        /// * Only alphanumeric characters or hyphens
        id: String,

        /// A short description about the role.
        description: String,
    },

    /// Update a role's permissions or description.
    Update {
        /// Role Identifier.
        id: String,

        /// Short description about the role.
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Delete a role.
    Delete {
        /// Role Identifier.
        id: String,
    },
}

impl Cli {
    pub async fn handle_role_subcommands(&self, command: RoleSubcommands) -> Result<()> {
        let cmds = command.command;
        match cmds {
            RoleCommands::List => self.role_list().await,
            RoleCommands::Get { id } => self.role_get(&id).await,
            RoleCommands::Create { id, description } => self.role_create(&id, &description).await,
            RoleCommands::Update { id, description } => self.role_update(&id, description).await,
            RoleCommands::Delete { id } => self.role_delete(&id).await,
        }
    }
}

impl Cli {
    pub async fn role_list(&self) -> Result<()> {
        let roles = self
            .client
            .list_roles()
            .await
            .context("Could not successfully retrieve roles from Gofer api")?
            .into_inner()
            .roles;

        let mut table = comfy_table::Table::new();
        table
            .load_preset(ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("id")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("description")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("system_owned")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
            ]);

        for role in roles {
            table.add_row(vec![
                Cell::new(role.id).fg(Color::Green),
                Cell::new(role.description),
                Cell::new(role.system_role),
            ]);
        }

        println!("{}", &table.to_string());
        Ok(())
    }

    pub async fn role_get(&self, id: &str) -> Result<()> {
        let role = self
            .client
            .get_role(id)
            .await
            .context("Could not successfully retrieve role from Gofer api")?
            .into_inner()
            .role;

        const TEMPLATE: &str = r#"  Role: {{id}}

  {{description}}

  Permissions:

{%- for line in permissions %}
  {{ line }}
{%- endfor %}
"#;

        let mut permission_map: HashMap<String, HashSet<String>> = std::collections::HashMap::new();

        for permission in &role.permissions {
            for resource in &permission.resources {
                permission_map
                    .entry(format!("{:?}:", resource))
                    .and_modify(|actions| {
                        for value in &permission.actions {
                            actions.insert(value.to_string());
                        }
                    })
                    .or_insert_with(|| {
                        permission
                            .actions
                            .iter()
                            .map(|value| value.to_string())
                            .collect()
                    });
            }
        }

        let mut permission_table = comfy_table::Table::new();
        permission_table
            .load_preset(comfy_table::presets::NOTHING)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("Resource")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("Actions")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
            ]);

        // Add empty row to space out the header from the actual permissions.
        permission_table.add_row(vec!["", ""]);

        let custom_order = ["Read", "Write", "Delete"];

        for (resource, action_list) in permission_map {
            let mut sorted_actions: Vec<_> = action_list.into_iter().collect();

            // We define a custom order above so that we roughly get an ordering comparable to unix permissions.
            sorted_actions.sort_by_key(|action| {
                custom_order
                    .iter()
                    .position(|&a| a == action.as_str())
                    .unwrap_or(usize::MAX)
            });
            permission_table.add_row(vec![
                Cell::new(resource),
                Cell::new(sorted_actions.join(", ")).fg(Color::Blue),
            ]);
        }

        // Sort permissions so they always have the same ordering; allows user to quickly scan.
        let mut permissions = permission_table
            .lines()
            .map(|line| line.to_string())
            .collect::<Vec<String>>();
        permissions.sort();

        let mut tera = tera::Tera::default();
        tera.add_raw_template("main", TEMPLATE)
            .context("Failed to render context")?;

        let mut context = tera::Context::new();
        context.insert("id", &role.id);
        context.insert("description", &role.description);
        context.insert("permissions", &permissions);

        let content = tera.render("main", &context)?;
        print!("{}", content);
        Ok(())
    }

    pub async fn role_create(&self, id: &str, description: &str) -> Result<()> {
        let mut permissions: Vec<Permission> = vec![];

        let resources = vec![
            Resource::All,
            Resource::Configs,
            Resource::Deployments,
            Resource::Events,
            Resource::Extensions("".into()),
            Resource::Namespaces("".into()),
            Resource::Objects,
            Resource::Permissions,
            Resource::Pipelines("".into()),
            Resource::Runs,
            Resource::Secrets,
            Resource::Subscriptions,
            Resource::System,
            Resource::TaskExecutions,
            Resource::Tokens,
        ];

        println!("Choose permissions for role:");
        println!();
        println!("Possible resources: {:?}", resources);
        println!();
        println!("For some resources you are allowed to enter a 'target'. You can enter that target after a colon \
            after the resource name.");
        println!();
        println!("Normal resource: `deployments`");
        println!("Resource with target specifier: `namespaces:^default$`");
        println!("Mixed: `namespaces:^default$,deployments,configs,pipelines:.*`");
        println!();
        println!("Enter a comma separated list of resources to give this token access to.");
        println!();

        loop {
            let user_given_resources = question!("Press enter when finished: ");
            println!();

            let user_given_resources: Vec<&str> = user_given_resources.split(',').collect();

            let mut resources: Vec<Resource> = vec![];

            for user_resource_target in user_given_resources {
                let user_resource_target_split: Vec<&str> =
                    user_resource_target.split(':').collect();
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
                    "taskexecutions" => Resource::TaskExecutions,
                    "tokens" => Resource::Tokens,
                    _ => {
                        println!("{} is not a valid resource type", user_resource);
                        continue;
                    }
                };

                resources.push(resource);
            }

            if resources.is_empty() {
                error!("Must choose at least one resource");
                continue;
            }

            println!(
                "Choose which actions this role can perform on the previously chosen resource:"
            );

            let possible_actions = ["Read", "Write", "Delete"];

            let mut action_choices: Vec<(&str, bool)> = possible_actions
                .into_iter()
                .map(|value| (value, false))
                .collect();

            polyfmt::choose_many(&mut action_choices)?;

            let mut actions = vec![];

            for (action, chosen) in action_choices {
                if !chosen {
                    continue;
                }

                let chosen_action = match action.to_lowercase().as_str() {
                    "read" => Action::Read,
                    "write" => Action::Write,
                    "delete" => Action::Delete,
                    _ => {
                        println!("{} is not a valid action type", action);
                        continue;
                    }
                };

                actions.push(chosen_action);
            }

            if actions.is_empty() {
                error!("Must choose at least one action");
                continue;
            }

            let new_permission = Permission { resources, actions };
            permissions.push(new_permission);

            let answer = question!("Would you like to add another permission? [y/N]: ");
            println!();

            if !answer.to_lowercase().starts_with('y') {
                break;
            }
        }

        let role = self
            .client
            .create_role(&gofer_sdk::api::types::CreateRoleRequest {
                description: description.into(),
                id: id.into(),
                permissions,
            })
            .await
            .context("Could not successfully create role from Gofer api")?
            .into_inner()
            .role;

        success!("Successfully created role '{}'!", role.id);
        Ok(())
    }

    pub async fn role_update(&self, id: &str, description: Option<String>) -> Result<()> {
        self.client
            .update_role(
                id,
                &gofer_sdk::api::types::UpdateRoleRequest {
                    description,
                    permissions: None,
                },
            )
            .await
            .context("Could not successfully update role from Gofer api")?;

        success!("role '{}' updated!", id);
        Ok(())
    }

    pub async fn role_delete(&self, id: &str) -> Result<()> {
        self.client
            .delete_role(id)
            .await
            .context("Could not successfully retrieve role from Gofer api")?;

        success!("role '{}' deleted!", id);
        Ok(())
    }
}
