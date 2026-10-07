use crate::cli::{Cli, rail, rail_table};
use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use colored::Colorize;
use comfy_table::{Cell, CellAlignment, Color, ContentArrangement, presets::ASCII_MARKDOWN};
use gofer_sdk::api::types::{
    Action, ExtensionGrant, ExtensionResource, GlobalGrant, GlobalResource, Grants, NamespaceGrant,
    NamespaceResource,
};
use polyfmt::{error, pause, println, question, resume, success};
use std::{path::PathBuf, str::FromStr};

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
    ///
    /// Grants are read from a JSON file if one is given, otherwise you will be prompted for them.
    ///
    /// Example grants file:
    ///
    /// {
    ///   "namespaces": [
    ///     { "namespace": "devops.*", "resources": ["pipelines", "runs"], "actions": ["read", "write"] }
    ///   ],
    ///   "global": [
    ///     { "resources": ["events"], "actions": ["read"] }
    ///   ]
    /// }
    Create {
        /// Role Identifier.
        ///
        /// Must be:
        /// * 32 > characters < 3
        /// * Only alphanumeric characters or hyphens
        id: String,

        /// A short description about the role.
        description: String,

        /// Path to a JSON file containing the role's grants.
        #[arg(short, long)]
        file: Option<PathBuf>,
    },

    /// Update a role's grants or description.
    Update {
        /// Role Identifier.
        id: String,

        /// Short description about the role.
        #[arg(short, long)]
        description: Option<String>,

        /// Path to a JSON file containing grants that will replace the role's current grants.
        #[arg(short, long)]
        file: Option<PathBuf>,
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
            RoleCommands::Create {
                id,
                description,
                file,
            } => self.role_create(&id, &description, file).await,
            RoleCommands::Update {
                id,
                description,
                file,
            } => self.role_update(&id, description, file).await,
            RoleCommands::Delete { id } => self.role_delete(&id).await,
        }
    }
}

fn read_grants_file(path: &PathBuf) -> Result<Grants> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Could not read grants file '{}'", path.display()))?;

    serde_json::from_str(&contents)
        .with_context(|| format!("Could not parse grants file '{}'", path.display()))
}

fn join<T: ToString>(values: &[T]) -> String {
    values
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Asks for a comma separated list and parses each value, repeating the question until every value is valid.
fn ask_list<T: FromStr>(prompt: &str, options: &[&str]) -> Vec<T> {
    loop {
        println!("Options: {}", options.join(", ").cyan());
        let answer = question!("{prompt}: ");
        println!();

        let values: Vec<&str> = answer
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .collect();

        if values.is_empty() {
            error!("Must choose at least one");
            continue;
        }

        let parsed: Result<Vec<T>, _> = values.iter().map(|value| T::from_str(value)).collect();

        match parsed {
            Ok(parsed) => return parsed,
            Err(_) => error!("One or more values were not valid options"),
        }
    }
}

fn ask_actions() -> Result<Vec<Action>> {
    loop {
        println!("Choose which actions this grant allows:");

        let possible_actions = ["Read", "Write", "Delete"];
        let mut action_choices: Vec<(&str, bool)> = possible_actions
            .into_iter()
            .map(|value| (value, false))
            .collect();

        // choose_many draws directly to the terminal so we pause the formatter while it runs.
        pause!();
        let choice_result = polyfmt::tui::choose_many(&mut action_choices, possible_actions.len());
        resume!();
        choice_result?;

        let actions: Vec<Action> = action_choices
            .into_iter()
            .filter(|(_, chosen)| *chosen)
            .map(|(action, _)| match action {
                "Read" => Action::Read,
                "Write" => Action::Write,
                _ => Action::Delete,
            })
            .collect();

        if actions.is_empty() {
            error!("Must choose at least one action");
            continue;
        }

        return Ok(actions);
    }
}

fn ask_target(prompt: &str, required: bool) -> Option<String> {
    loop {
        let answer = question!("{prompt}: ");
        println!();
        let answer = answer.trim();

        if !answer.is_empty() {
            return Some(answer.to_string());
        }

        if !required {
            return None;
        }

        error!("A target is required; use '.*' to match everything");
    }
}

fn prompt_for_grants() -> Result<Grants> {
    let mut grants = Grants {
        namespaces: vec![],
        extensions: vec![],
        global: vec![],
    };

    println!(
        "Each grant stands on its own; a request is only allowed if a single grant covers it."
    );
    println!("Targets are regexes matched against the entire id; use '.*' to match everything.");
    println!();

    loop {
        let kind = question!("Grant type [namespace/extension/global]: ");
        println!();

        match kind.trim().to_lowercase().as_str() {
            "namespace" | "n" => {
                let namespace = ask_target("Namespace target", true).unwrap();
                let pipeline =
                    ask_target("Pipeline target (press enter for every pipeline)", false);
                let resources = ask_list::<NamespaceResource>(
                    "Resources (comma separated)",
                    &[
                        "pipelines",
                        "configs",
                        "deployments",
                        "runs",
                        "task_executions",
                        "objects",
                        "secrets",
                        "subscriptions",
                    ],
                );
                let actions = ask_actions()?;

                grants.namespaces.push(NamespaceGrant {
                    namespace,
                    pipeline,
                    resources,
                    actions,
                });
            }
            "extension" | "e" => {
                let extension = ask_target("Extension target", true).unwrap();
                let resources = ask_list::<ExtensionResource>(
                    "Resources (comma separated)",
                    &["objects", "subscriptions", "logs"],
                );
                let actions = ask_actions()?;

                grants.extensions.push(ExtensionGrant {
                    extension,
                    resources,
                    actions,
                });
            }
            "global" | "g" => {
                let resources = ask_list::<GlobalResource>(
                    "Resources (comma separated)",
                    &["events", "tokens", "roles", "secrets", "system"],
                );
                let actions = ask_actions()?;

                grants.global.push(GlobalGrant { resources, actions });
            }
            _ => {
                error!("Grant type must be one of namespace, extension, or global");
                continue;
            }
        }

        let answer = question!("Would you like to add another grant? [y/N]: ");
        println!();

        if !answer.to_lowercase().starts_with('y') {
            break;
        }
    }

    Ok(grants)
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

  $ Namespace Grants:
  {%- for line in namespaces %}
  {{ line }}
  {%- endfor %}

  $ Extension Grants:
  {%- for line in extensions %}
  {{ line }}
  {%- endfor %}

  $ Global Grants:
  {%- for line in global %}
  {{ line }}
  {%- endfor %}
"#;

        let namespace_rows = role
            .grants
            .namespaces
            .iter()
            .map(|grant| {
                vec![
                    Cell::new(&grant.namespace),
                    Cell::new(grant.pipeline.as_deref().unwrap_or(".*")),
                    Cell::new(join(&grant.resources)),
                    Cell::new(join(&grant.actions)).fg(Color::Blue),
                ]
            })
            .collect();

        let extension_rows = role
            .grants
            .extensions
            .iter()
            .map(|grant| {
                vec![
                    Cell::new(&grant.extension),
                    Cell::new(join(&grant.resources)),
                    Cell::new(join(&grant.actions)).fg(Color::Blue),
                ]
            })
            .collect();

        let global_rows = role
            .grants
            .global
            .iter()
            .map(|grant| {
                vec![
                    Cell::new(join(&grant.resources)),
                    Cell::new(join(&grant.actions)).fg(Color::Blue),
                ]
            })
            .collect();

        let mut tera = tera::Tera::default();
        tera.add_raw_template("main", TEMPLATE)
            .context("Failed to render context")?;

        let mut context = tera::Context::new();
        context.insert("vertical_line", &rail());
        context.insert("description", &role.description);
        context.insert(
            "namespaces",
            &rail_table(
                &["NAMESPACE", "PIPELINE", "RESOURCES", "ACTIONS"],
                namespace_rows,
            ),
        );
        context.insert(
            "extensions",
            &rail_table(&["EXTENSION", "RESOURCES", "ACTIONS"], extension_rows),
        );
        context.insert(
            "global",
            &rail_table(&["RESOURCES", "ACTIONS"], global_rows),
        );

        let content = tera.render("main", &context)?;
        println!("  Role {}", role.id.cyan());
        println!("{}", content.trim_end());
        Ok(())
    }

    pub async fn role_create(
        &self,
        id: &str,
        description: &str,
        file: Option<PathBuf>,
    ) -> Result<()> {
        let grants = match file {
            Some(path) => read_grants_file(&path)?,
            None => prompt_for_grants()?,
        };

        let role = self
            .client
            .create_role(&gofer_sdk::api::types::CreateRoleRequest {
                description: description.into(),
                id: id.into(),
                grants,
            })
            .await
            .context("Could not successfully create role from Gofer api")?
            .into_inner()
            .role;

        success!("Successfully created role '{}'!", role.id);
        Ok(())
    }

    pub async fn role_update(
        &self,
        id: &str,
        description: Option<String>,
        file: Option<PathBuf>,
    ) -> Result<()> {
        if description.is_none() && file.is_none() {
            bail!("Nothing to update; pass --description and/or --file");
        }

        let grants = file.map(|path| read_grants_file(&path)).transpose()?;

        self.client
            .update_role(
                id,
                &gofer_sdk::api::types::UpdateRoleRequest {
                    description,
                    grants,
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
