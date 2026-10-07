use crate::cli::{Cli, rail, rail_table};
use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use colored::Colorize;
use comfy_table::{Cell, CellAlignment, Color, ContentArrangement, presets::ASCII_MARKDOWN};
use gofer_sdk::api::types::{Action, Permission};
use polyfmt::{error, pause, println, question, resume, success};
use std::collections::HashSet;

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
            .load_style(ASCII_MARKDOWN)
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

        const TEMPLATE: &str = r#"
  {{ vertical_line }} Description: {{ description }}

  $ Permissions:
  {%- for line in permissions %}
  {{ line }}
  {%- endfor %}
"#;

        let mut permission_map: std::collections::BTreeMap<String, HashSet<String>> =
            std::collections::BTreeMap::new();

        for permission in &role.permissions {
            for resource in &permission.resources {
                permission_map
                    .entry(resource.to_string())
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

        let mut permission_rows = vec![];

        let custom_order = ["Read", "Write", "Delete"];

        // Resources come from a BTreeMap so rows are already in a stable order; allows user to quickly scan.
        for (resource, action_list) in permission_map {
            let mut sorted_actions: Vec<_> = action_list.into_iter().collect();

            // We define a custom order above so that we roughly get an ordering comparable to unix permissions.
            sorted_actions.sort_by_key(|action| {
                custom_order
                    .iter()
                    .position(|&a| a.eq_ignore_ascii_case(action))
                    .unwrap_or(usize::MAX)
            });
            permission_rows.push(vec![
                Cell::new(resource),
                Cell::new(sorted_actions.join(", ")).fg(Color::Blue),
            ]);
        }

        let mut tera = tera::Tera::default();
        tera.add_raw_template("main", TEMPLATE)
            .context("Failed to render context")?;

        let mut context = tera::Context::new();
        context.insert("vertical_line", &rail());
        context.insert("description", &role.description);
        context.insert(
            "permissions",
            &rail_table(&["RESOURCE", "ACTIONS"], permission_rows),
        );

        let content = tera.render("main", &context)?;
        println!("  Role {}", role.id.cyan());
        println!("{}", content.trim_end());
        Ok(())
    }

    pub async fn role_create(&self, id: &str, description: &str) -> Result<()> {
        let mut permissions: Vec<Permission> = vec![];

        let resources = vec![
            "all",
            "configs",
            "deployments",
            "events",
            "extensions:<target>",
            "namespaces:<target>",
            "objects",
            "permissions",
            "pipelines:<target>",
            "runs",
            "secrets",
            "subscriptions",
            "system",
            "task_executions",
            "tokens",
        ];

        println!("Choose permissions for role:");
        println!();
        println!("Possible resources: {:?}", resources);
        println!();
        println!(
            "Extensions, namespaces, and pipelines require a 'target' after a colon. The target is a regex \
            matched against the entire id; use '.*' to match everything."
        );
        println!();
        println!("Example normal resource: {}", "deployments".cyan());
        println!(
            "Example resource with target specifier: {}",
            "namespaces:^default$".cyan()
        );
        println!(
            "Example mixed: {}",
            "namespaces:^default$,deployments,configs,pipelines:.*".cyan()
        );
        println!();
        println!(
            "Each permission is a standalone grant; a route is only allowed if a single permission covers every \
            resource it needs."
        );
        println!();
        println!("Enter a comma separated list of resources to give this token access to.");
        println!();

        loop {
            let user_given_resources = question!("Press enter when finished: ");
            println!();

            let user_given_resources: Vec<&str> = user_given_resources.split(',').collect();
            let resources: Vec<String> = user_given_resources
                .into_iter()
                .map(|resource| resource.trim().to_string())
                .filter(|resource| !resource.is_empty())
                .collect();

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

            // choose_many draws directly to the terminal so we pause the formatter while it runs.
            pause!();
            let choice_result =
                polyfmt::tui::choose_many(&mut action_choices, possible_actions.len());
            resume!();
            choice_result?;

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
