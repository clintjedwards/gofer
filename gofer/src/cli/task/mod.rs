use crate::cli::{
    colorize_status_text, colorize_status_text_comfy, duration, humanize_relative_duration, Cli,
};
use anyhow::{anyhow, bail, Context, Result};
use clap::{Args, Subcommand};
use colored::Colorize;
use comfy_table::{Cell, CellAlignment, Color, ContentArrangement};
use futures::{SinkExt, StreamExt};
use polyfmt::{error, print, println, success};
use std::{io::Write, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    signal,
    sync::Mutex,
};
use tokio_tungstenite::WebSocketStream;
use tungstenite::Message;

#[derive(Debug, Args, Clone)]
pub struct TaskSubcommands {
    #[clap(subcommand)]
    pub command: TaskCommands,

    /// Namespace Identifier.
    #[clap(long, global = true)]
    pub namespace: Option<String>,
}

#[derive(Debug, Subcommand, Clone)]
pub enum TaskCommands {
    /// List all task executions.
    List {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Run Identifier.
        run_id: u64,
    },

    /// Get details on a specific task execution.
    Get {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Run Identifier.
        run_id: u64,

        /// Task Identifier.
        task_id: String,
    },

    /// Attach to a running container.
    ///
    /// Gofer allows you to connect your terminal to a container and run commands.
    /// This is useful for debugging or just general informational gathering.
    ///
    /// The connection to the container only lasts as long as it is running and will
    /// be severed upon the container's completion.
    ///
    /// It should be also noted that this feature is a bit of an anti-pattern. In theory
    /// testing your containers locally + good logging + the information Gofer provides
    /// should be good enough to debug most issues. That being said there are always scheduler
    /// specific infrastructure issues that it is just helpful to have a shell within.
    Attach {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Run Identifier.
        run_id: u64,

        /// Task Identifier.
        task_id: String,

        /// The command to run when we first attach.
        #[arg(short, long, default_value = "/bin/sh")]
        cmd: String,
    },

    /// Cancel a specific task execution.
    ///
    /// Cancels a task execution by requesting that the scheduler gracefully stops it. Usually this means the
    /// scheduler will pass a SIGTERM to the container. If the container does not shut down within the API
    /// defined timeout or the user has passed the force flag the scheduler will then kill the container immediately.
    ///
    /// Cancelling a task execution might mean that downstream/dependent task executions are skipped.
    Cancel {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Run Identifier.
        run_id: u64,

        /// Task Identifier.
        task_id: String,

        /// Wait this many seconds and then force kill the container. 0 means immediately kill the container
        #[arg(short, long, default_value = "15")]
        wait_for: u64,
    },

    /// Examine logs for a particular task executions/container.
    Logs {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Run Identifier.
        run_id: u64,

        /// Task Identifier.
        task_id: String,
    },
}

impl Cli {
    pub async fn handle_task_subcommands(&self, command: TaskSubcommands) -> Result<()> {
        let cmds = command.command;
        match cmds {
            TaskCommands::List {
                pipeline_id,
                run_id,
            } => {
                self.task_list(command.namespace, &pipeline_id, run_id)
                    .await
            }
            TaskCommands::Get {
                pipeline_id,
                run_id,
                task_id,
            } => {
                self.task_get(command.namespace, &pipeline_id, run_id, &task_id)
                    .await
            }
            TaskCommands::Attach {
                pipeline_id,
                run_id,
                task_id,
                cmd,
            } => {
                self.task_attach(command.namespace, &pipeline_id, run_id, &task_id, &cmd)
                    .await
            }
            TaskCommands::Cancel {
                pipeline_id,
                run_id,
                task_id,
                wait_for,
            } => {
                self.task_cancel(command.namespace, &pipeline_id, run_id, &task_id, wait_for)
                    .await
            }
            TaskCommands::Logs {
                pipeline_id,
                run_id,
                task_id,
            } => {
                self.task_logs(command.namespace, &pipeline_id, run_id, &task_id)
                    .await
            }
        }
    }
}

impl Cli {
    pub async fn task_list(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        run_id: u64,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let tasks = self
            .client
            .list_task_executions(&namespace, pipeline_id, run_id)
            .await
            .context("Could not successfully retrieve tasks from Gofer api")?
            .into_inner()
            .task_executions;

        let mut table = comfy_table::Table::new();
        table
            .load_preset(comfy_table::presets::ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("id")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("started")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("ended")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("duration")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("state")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("status")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
            ]);

        for task in tasks {
            table.add_row(vec![
                Cell::new(task.task_id).fg(Color::Green),
                Cell::new(
                    humanize_relative_duration(task.started).unwrap_or("Unknown".to_string()),
                ),
                Cell::new(
                    humanize_relative_duration(task.ended).unwrap_or("Still running".to_string()),
                ),
                Cell::new(duration(task.started as i64, task.ended as i64)),
                Cell::new(task.state).fg(colorize_status_text_comfy(task.state)),
                Cell::new(task.status).fg(colorize_status_text_comfy(task.status)),
            ]);
        }

        println!("{}", &table.to_string());
        Ok(())
    }

    pub async fn task_get(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        run_id: u64,
        task_id: &str,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let task = self
            .client
            .get_task_execution(&namespace, pipeline_id, run_id, task_id)
            .await
            .context("Could not successfully retrieve task from Gofer api")?
            .into_inner()
            .task_execution;

        let mut variable_table = comfy_table::Table::new();
        variable_table
            .load_preset(comfy_table::presets::NOTHING)
            .set_content_arrangement(ContentArrangement::Dynamic);

        for variable in task.variables {
            variable_table.add_row(vec![
                Cell::new("│").fg(Color::Magenta),
                Cell::new(variable.key),
                Cell::new(variable.value).fg(Color::Blue),
                Cell::new(variable.source.to_string()).fg(Color::AnsiValue(245)),
            ]);
        }

        const TEMPLATE: &str = r#"
  {{ vertical_line }} Parent Pipeline: {{ pipeline_id }}
  {{ run_prefix }} Parent Run: {{ run_id }}
  {{ task_prefix }} Task ID: {{ task_id }}
  {{ vertical_line }} Image: {{ image_name }}
  {{ vertical_line }} Exit Code: {{ exit_code }}
  {{ vertical_line }} Started {{ started }} and ran for {{ duration }}

  {%- if status_reason %}
    Status Details:
    {{ vertical_line }} Reason: {{ status_reason.reason }}
    {{ vertical_line }} Description: {{ status_reason.description }}
  {%- endif %}
  {%- if env_vars is defined %}

  $ Environment Variables:
  {%- for line in env_vars %}
  {{ line }}
  {%- endfor %}
  {%- endif %}

* Use '{{ task_execution_cmd }}' to view logs.
"#;

        let mut tera = tera::Tera::default();
        tera.add_raw_template("main", TEMPLATE)
            .context("Failed to render context")?;

        let mut context = tera::Context::new();
        context.insert("vertical_line", &"│".magenta().to_string());
        context.insert("pipeline_id", &task.pipeline_id.blue().to_string());
        context.insert("run_prefix", &"├─".magenta().to_string());
        context.insert("run_id", &format!("#{}", &task.run_id).blue().to_string());
        context.insert("task_id", &task.task_id.blue().to_string());
        context.insert("task_prefix", &"├──".magenta().to_string());
        context.insert(
            "started",
            &humanize_relative_duration(task.started).unwrap_or_else(|| "Not yet".to_string()),
        );
        context.insert(
            "duration",
            &duration(task.started as i64, task.ended as i64),
        );
        context.insert("image_name", &task.task.image.blue().to_string());
        context.insert(
            "exit_code",
            &task
                .exit_code
                .map(|code| code.to_string())
                .unwrap_or("None".into()),
        );
        context.insert("status_reason", &task.status_reason);
        context.insert(
            "env_vars",
            &variable_table
                .lines()
                .map(|line| line.to_string())
                .collect::<Vec<String>>(),
        );
        context.insert(
            "task_execution_cmd",
            &format!(
                "gofer task logs {} {} {}",
                task.pipeline_id, task.run_id, task.task_id
            )
            .cyan()
            .to_string(),
        );

        let content = tera.render("main", &context)?;
        println!(
            "Task {} :: {} :: {}",
            task.task_id.blue(),
            colorize_status_text(task.state),
            colorize_status_text(task.status)
        );
        print!("{}", content);
        Ok(())
    }

    pub async fn task_logs(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        run_id: u64,
        task_id: &str,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let task_logs_conn = self
            .client
            .get_logs(&namespace, pipeline_id, run_id, task_id)
            .await
            .map_err(|e| anyhow!("could not get logs; {:#?}", e))?
            .into_inner();

        let stream = WebSocketStream::from_raw_socket(
            task_logs_conn,
            tokio_tungstenite::tungstenite::protocol::Role::Client,
            None,
        )
        .await;

        let (_, mut read) = stream.split();

        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Text(text)) => println!("{}", text),
                Ok(Message::Binary(_)) => println!("Received binary data"),
                Ok(Message::Close(frame)) => {
                    if let Some(frame) = frame {
                        match frame.code {
                            tungstenite::protocol::frame::coding::CloseCode::Normal => break,
                            _ => {
                                error!("Connection closed by server; {}", frame.reason)
                            }
                        }
                        break;
                    }
                    error!("Connection closed by server without reason");
                    break;
                }
                Err(tokio_tungstenite::tungstenite::Error::ConnectionClosed) => {
                    error!("Connection closed");
                    break;
                }
                Err(tokio_tungstenite::tungstenite::Error::Protocol(e))
                    if e.to_string()
                        .contains("Connection reset without closing handshake") =>
                {
                    error!("Connection reset without closing handshake");
                    break;
                }
                Err(e) => {
                    bail!("Error receiving message: {}", e);
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub async fn task_cancel(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        run_id: u64,
        task_id: &str,
        wait_for: u64,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        self.client
            .cancel_task_execution(&namespace, pipeline_id, run_id, task_id, wait_for)
            .await
            .context("Could not successfully cancel task")?
            .into_inner();

        success!("Successfully cancelled task '{}'", task_id);

        Ok(())
    }

    pub async fn task_attach(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        run_id: u64,
        task_id: &str,
        command: &str,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let task_attach_conn = self
            .client
            .attach_task_execution(&namespace, pipeline_id, run_id, task_id, command)
            .await
            .map_err(|e| anyhow!("could not get logs; {:#?}", e))?
            .into_inner();

        let stream = WebSocketStream::from_raw_socket(
            task_attach_conn,
            tokio_tungstenite::tungstenite::protocol::Role::Client,
            None,
        )
        .await;

        let (write, mut read) = stream.split();

        let shared_writer = Arc::new(Mutex::new(write));

        let close_writer = shared_writer.clone();

        tokio::spawn(async move {
            signal::ctrl_c()
                .await
                .expect("Failed to listen for Ctrl+C signal");

            let _ = close_writer.lock().await.send(Message::Close(None)).await;
            std::process::exit(0);
        });

        // Read handler
        tokio::spawn(async move {
            while let Some(message) = read.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        print!("{}", text);
                        std::io::stdout().flush().unwrap();
                    }
                    Ok(Message::Binary(text)) => {
                        print!("{}", String::from_utf8_lossy(&text));
                        std::io::stdout().flush().unwrap();
                    }
                    Ok(Message::Close(_)) => {
                        error!("Connection closed by server");
                        break;
                    }
                    Err(tokio_tungstenite::tungstenite::Error::ConnectionClosed) => {
                        error!("Connection closed");
                        break;
                    }
                    Err(tokio_tungstenite::tungstenite::Error::Protocol(e))
                        if e.to_string()
                            .contains("Connection reset without closing handshake") =>
                    {
                        error!("Connection reset without closing handshake");
                        break;
                    }
                    Err(e) => {
                        error!("Error receiving message: {}", e);
                    }
                    _ => {}
                }
            }
        });

        // Write handler
        let stdin = tokio::io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        while let Some(line) = lines
            .next_line()
            .await
            .context("Error while attempting to process user input")?
        {
            shared_writer
                .lock()
                .await
                .send(Message::Text(line))
                .await
                .context("Error while attempting to copy user input to server")?;
        }

        Ok(())
    }
}
